mod audio;
mod bluetooth;
mod clients;
mod host;
mod ipc;
mod logout;
mod nix;
mod profile;
mod systray;
mod time;
mod title;
mod volume;
mod watcher;
mod workspaces;

use std::{process::Command, time::Duration};

use crate::audio::setup_controllers;
use audio::{get_player_img, inject_outline_style};
use debug_print::debug_println;
use gdk_pixbuf::PixbufLoader;
use gio::prelude::*;
use glib::{self, GString};
use gtk::prelude::*;
use gtk4::GestureClick;
use gtk4::{self as gtk, CssProvider, gdk::Display};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use ipc::HyprEvent;
use tokio::sync::mpsc;

const THREE_DAYS: i64 = 60 * 60 * 24 * 3;

/// Spawn a command without blocking, reaping it on a background thread so it
/// doesn't linger as a zombie after it exits.
pub(crate) fn spawn_reaped(mut cmd: Command) {
    if let Ok(mut child) = cmd.spawn() {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

/// Icons on the active workspace carry this class and are shown large.
const GROWN_CLASS: &str = "clientchild";

fn animate_pixel_size(image: &gtk::Image, from: i32, to: i32) {
    if from == to {
        return;
    }
    let steps = 64;
    let per_step = ((to - from).abs() + steps - 1) / steps;
    let i_c = image.clone();

    glib::timeout_add_local(Duration::from_millis(4), move || {
        let current = i_c.pixel_size();
        if current == to {
            return glib::ControlFlow::Break;
        }

        // Only grow icons that should be large and only shrink ones that
        // shouldn't, so a stale animation stops once a newer one takes over.
        let growing = current < to;
        if growing != i_c.has_css_class(GROWN_CLASS) {
            return glib::ControlFlow::Break;
        }

        let next = if growing {
            (current + per_step).min(to)
        } else {
            (current - per_step).max(to)
        };
        i_c.set_pixel_size(next);
        glib::ControlFlow::Continue
    });
}

fn left(container: &mut gtk::Box) -> (gtk::Box, gtk::Label) {
    let wp = workspaces::workspaces();
    let tbox = title::title();
    container.append(&wp);
    container.append(&tbox);

    (wp, tbox)
}

fn center(container: &gtk::Box) -> Vec<gtk::Image> {
    let imgs = clients::clients();
    let innerbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    for i in &imgs {
        innerbox.append(i);
    }
    container.append(&innerbox);

    imgs
}

fn right(
    container: &mut gtk::Box,
) -> (
    gtk::Revealer,
    gtk::Image,
    gtk::Image,
    gtk::Image,
    gtk::Label,
    gtk::Label,
    gtk::Image,
    gtk::Label,
    gtk::Label,
    gtk::Box,
    gtk::PopoverMenu,
) {
    let (ad, prev, play, next) = audio::audio();
    container.append(&ad);
    let pf = profile::profile();
    container.append(&pf);
    let vol = volume::volume();
    container.append(&vol);
    let bt = bluetooth::bluetooth();
    container.append(&bt);
    let nix = nix::nix();
    container.append(&nix);
    let (systray, systray_menu) = systray::systray();
    container.append(&systray);
    let time = time::time();
    container.append(&time);

    container.append(&logout::logout());

    (
        ad,
        prev,
        play,
        next,
        vol,
        bt,
        nix,
        pf,
        time,
        systray,
        systray_menu,
    )
}

fn load_css() {
    let css = grass::from_string(include_str!("style.scss"), &grass::Options::default()).unwrap();

    let provider = CssProvider::new();
    provider.load_from_data(&css);

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
}

fn activate(
    application: &gtk::Application,
) -> (
    gtk::Box,
    gtk::Label,
    gtk::Box,
    Vec<gtk::Image>,
    gtk::Revealer,
    gtk::Image,
    gtk::Image,
    gtk::Image,
    gtk::Label,
    gtk::Label,
    gtk::Image,
    gtk::Label,
    gtk::Label,
    gtk::Box,
    gtk::PopoverMenu,
) {
    let window = gtk::ApplicationWindow::new(application);

    // Before the window is first realized, set it up to be a layer surface
    window.init_layer_shell();

    // Display above normal windows
    window.set_layer(Layer::Top);

    window.set_size_request(100, 36);

    // Push other windows out of the way
    window.auto_exclusive_zone_enable();

    // Anchors are if the window is pinned to each edge of the output
    window.set_margin(Edge::Left, 0);
    window.set_margin(Edge::Right, 0);

    // ... or like this
    // Anchors are if the window is pinned to each edge of the output
    let anchors = [
        (Edge::Left, true),
        (Edge::Right, true),
        (Edge::Top, true),
        (Edge::Bottom, false),
    ];

    for (anchor, state) in anchors {
        window.set_anchor(anchor, state);
    }

    // Set up a widget
    let root_container = gtk::CenterBox::new();

    let mut start_widget = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    start_widget.add_css_class("rbackground");
    let (wp, tbox) = left(&mut start_widget);

    let mut center_widget = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    center_widget.add_css_class("cbackground");
    let imgs = center(&mut center_widget);

    let mut end_widget = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    end_widget.add_css_class("rbackground");
    let (ad, prev, play, next, vol, bt, nix, pf, time, tray_box, tray_box_popover) =
        right(&mut end_widget);
    tray_box_popover.set_parent(&window);

    root_container.set_start_widget(Some(&start_widget));
    root_container.set_center_widget(Some(&center_widget));
    root_container.set_end_widget(Some(&end_widget));

    root_container.set_vexpand(true);

    window.set_child(Some(&root_container));
    window.add_css_class("bar");
    window.set_vexpand(true);

    window.present();

    (
        wp,
        tbox,
        center_widget,
        imgs,
        ad,
        prev,
        play,
        next,
        vol,
        bt,
        nix,
        pf,
        time,
        tray_box,
        tray_box_popover,
    )
}

fn main() {
    // Register the watcher on the bus

    let application = gtk::Application::new(Some("sh.rc"), Default::default());

    application.connect_startup(|_| load_css());

    application.connect_activate(|app| {
        let (wp, tbox, center_widget, mut imgs, ad, prev, play, next, vol, bt, nix, pf, time, tray_box, tray_box_popover) = activate(app);
        setup_controllers!(ad, prev, play, next);


        let (sender, mut receiver) = mpsc::channel(8);

        let _ = ipc::spawn_event_listener(sender);

        let (tray_tx, mut tray_rx) = tokio::sync::mpsc::channel::<Vec<host::TrayItem>>(8);

        // System Tray

        glib::MainContext::default().spawn_local(async move {
            let w = watcher::StatusNotifierWatcher::spawn().await.unwrap();
            let watching_names = async {
                let _ = w.watch_names().await;
            };
            let mut host = systray::host().await;

            // Send initial items
            let initial: Vec<host::TrayItem> = host.items().values().cloned().collect();
            let _ = tray_tx.send(initial).await;

            // Updater reads from channel, no host access needed
            let conn = host.connection();
            let updater = async move {
                while let Some(items) = tray_rx.recv().await {
                    let refs: Vec<&host::TrayItem> = items.iter().collect();
                    systray::update_tray(&tray_box, &tray_box_popover, &refs, &conn);
                }
            };

            // Driver owns host exclusively, sends items on each change
            let driver = async move {
                let _ = host.run(tray_tx).await;
            };

            tokio::join!(driver, updater, watching_names);
        });


        // Time
        glib::MainContext::default().spawn_local(async move {
            loop {
                time.set_label(&time::current_time());

                glib::timeout_future(Duration::from_millis(100)).await;
            }
        });



        // Bluetooth battery: slow to query and changes slowly, so it runs in its
        // own loop with the command on a worker thread.
        glib::MainContext::default().spawn_local(async move {
            loop {
                let v = gio::spawn_blocking(|| {
                    Command::new("btbattery")
                        .output()
                        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                        .unwrap_or_default()
                })
                .await
                .unwrap_or_default();

                debug_println!("btbattery returned {}", v);

                let val = bluetooth::battery_label(&v);

                debug_println!("btbattery val is {}", val);

                bt.set_text(&val);

                glib::timeout_future(Duration::from_secs(10)).await;
            }
        });

        // Background tasks
        glib::MainContext::default().spawn_local(async move {
            // Only reload the player icons when the player or its state changes.
            let mut last_player = String::new();
            let mut last_playing: Option<bool> = None;

            loop {
                // Audio Player
                let output =
                    String::from_utf8(Command::new("playerctl").arg("-l").output().unwrap().stdout).unwrap();

                let clients = audio::player_list(&output);

                ad.set_reveal_child(!clients.is_empty());

                if clients.is_empty() {
                    last_player.clear();
                    last_playing = None;
                }

                if let Some(ci) = audio::current_player(&ad, &clients) {
                    let player_changed = last_player != clients[ci];
                    if player_changed {
                        last_player = clients[ci].to_string();
                    }

                    let inbox = ad.child().unwrap().downcast::<gtk::Box>().unwrap();

                    let is_playing = {
                        let s = String::from_utf8(
                            Command::new("playerctl")
                                .args(["status", "-p", clients[ci]])
                                .output()
                                .unwrap()
                                .stdout,
                        )
                        .unwrap();
                        if s.contains("Playing") { true } else { false }
                    };

                    let inner_revealer = inbox.first_child().unwrap().downcast::<gtk::Revealer>().unwrap();
                    let cbox = inner_revealer.child().unwrap().downcast::<gtk::CenterBox>().unwrap();
                    let play_widget = cbox.center_widget().unwrap().downcast::<gtk::Image>().unwrap();

                    if player_changed || last_playing != Some(is_playing) {
                        last_playing = Some(is_playing);
                        play_widget.set_from_file(Some(format!(
                                "/home/rc/default/assets/{}.svg",
                                if is_playing { "pause" } else { "play" }
                            )));
                        play_widget.set_pixel_size(22);
                    }

                    if player_changed {
                        let img = inbox.first_child().unwrap().next_sibling().unwrap().downcast::<gtk::Image>().unwrap();
                        let base_img = get_player_img(ci, &clients);
                        let svg = inject_outline_style(&base_img, "#c0caf5");
                        let loader = PixbufLoader::with_type("svg").expect("Failed to create loader");
                        loader.write(svg.as_bytes()).expect("Failed to load svg");
                        loader.close().expect("Failed to close loader");

                        let px = loader.pixbuf().expect("Failed to create pixbuf");
                        img.set_from_pixbuf(Some(&px));
                    }
                }


                // Volume
                let v = String::from_utf8(
                    Command::new("wpctl")
                        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
                        .output()
                        .unwrap()
                        .stdout,
                )
                .unwrap();

                debug_println!("wpctl get-volume @DEFAULT_AUDIO_SINK@ returned {}", v);

                let val = volume::volume_label(&v);

                debug_println!("wpctl val is {}", val);

                vol.set_text(&val);


                // Audio Profile
                let args = [
                    "-c",
                    "pactl -f json list cards | jq -r '.[] | select(.name | contains(\"bluez\")) | .active_profile'",
                ];

                let active_profile =
                    String::from_utf8(Command::new("sh").args(args).output().unwrap().stdout).unwrap();

                debug_println!("pactl active profile returned {}", active_profile);

                let label = if active_profile.trim() == "" {
                    "UN"
                } else if active_profile.contains("sbc_xq") {
                    "XQ"
                } else if active_profile.contains("headset") && !active_profile.contains("cvsd") {
                    "VC"
                } else {
                    "UN"
                };

                debug_println!("pactl label is {}", label);

                pf.set_label(label);
                if label == "VC" {
                    pf.add_css_class("vc");
                }
                else if pf.has_css_class("vc") {
                    pf.remove_css_class("vc");
                }

                glib::timeout_future(Duration::from_millis(50)).await;
            }
        });

        macro_rules! workspace_focus {
            ($active : ident) => {
                let output = String::from_utf8(Command::new("sh")
                        .args(["-c", "hyprctl activeworkspace -j | jq -r '(.id | tostring) + \",\" + (.windows | tostring)'"])
                        .output()
                        .expect("failed").stdout).unwrap();

                let (id, windows) = output.trim().split_once(',').unwrap_or(("0", "0"));
                let $active = id.parse::<i64>().unwrap_or(0).unsigned_abs() as usize;
                let num_windows = windows.parse::<usize>().unwrap_or(0);

                let mut curr_child = wp.first_child().unwrap();
                let mut upd = 0;

                for i in 1..=8 {

                    if curr_child.css_classes().contains(&GString::from_string_checked("active".to_string()).unwrap()) {
                        curr_child.remove_css_class("active");
                        upd += 1;
                    }

                    if i == $active {
                        curr_child.add_css_class("active");
                        upd += 1;
                    }

                    if upd == 2 { break };
                    if i < 8 {
                        curr_child = curr_child.next_sibling().unwrap();
                    }
                }


                let c_classes = center_widget.css_classes();
                if num_windows > 0 {
                    if c_classes.contains(&GString::from_string_checked("inactive".to_string()).unwrap()) {
                        center_widget.remove_css_class("inactive");
                        center_widget.set_opacity(1.0);
                    }
                }
                else {
                    if !c_classes.contains(&GString::from_string_checked("inactive".to_string()).unwrap()) {
                        center_widget.add_css_class("inactive");
                        center_widget.set_opacity(0.0);
                    };
                }
            }
        }


        let last_state = std::cell::RefCell::new([0usize; 8]);
        // Hyprland events
        // <20 ms on average
        glib::MainContext::default().spawn_local(async move {

            let client_counts = || {
                let mut data: [usize; 8] = [0; 8];

                let output = String::from_utf8(
                    Command::new("hyprworkspaces")
                        .output()
                        .expect("failed")
                        .stdout,
                )
                .unwrap();

                for line in output.lines() {
                    if let Some((num, windows)) = workspaces::parse_workspace_line(line) {
                        data[num - 1] = windows;
                    }
                }

                let mut p = last_state.borrow_mut();
                if data != *p {
                    *p = data;
                    let mut curr_child = wp.first_child().unwrap().downcast::<gtk::Label>().unwrap();

                    for i in 1..=8 {
                        let windows = data[i - 1];

                        let mkp = if windows > 0 {
                            if !curr_child.css_classes().contains(&GString::from_string_checked("occupied".to_string()).unwrap()) {
                                curr_child.add_css_class("occupied");
                            }

                            format!(
                                "<span size='x-small' rise='10000'>{}</span><span rise='3000'>/</span><span size='x-small'>{}</span>",
                                i, windows,
                            )
                        } else {
                            if curr_child.css_classes().contains(&GString::from_string_checked("occupied".to_string()).unwrap()) {
                                curr_child.remove_css_class("occupied");
                            }
                            format!("{}", i)
                        };
                        curr_child.set_markup(mkp.as_str());

                        if i < 8 {
                            curr_child = curr_child.next_sibling().unwrap().downcast::<gtk::Label>().unwrap();
                        }
                    }
                }

            };


            while let Some(msg) = receiver.recv().await {
                match msg {
                    HyprEvent::ActiveWindow => {
                        let title = String::from_utf8(Command::new("sh")
                                .args(["-c", "hyprctl activeworkspace -j | jq -r '(.lastwindowtitle | tostring)'"])
                                .output()
                                .expect("failed").stdout).unwrap();

                        tbox.set_text(&title::client_title_wrap(title.clone()));
                        tbox.set_tooltip_text(Some(&title.as_str()));

                        let pt = String::from_utf8(Command::new("sh")
                                .args(["-c", "hyprctl activeworkspace -j | jq -r '(.lastwindow | tostring)'"])
                                .output()
                                .expect("failed").stdout).unwrap();


                        for i in &imgs {
                            if i.widget_name().as_str() == pt.trim() {
                                i.add_css_class("active");
                                animate_pixel_size(i, i.pixel_size(), 36);
                                i.set_size_request(36, 36);
                            }
                            else if i.css_classes().contains(&GString::from_string_checked("active".to_string()).unwrap()) {
                                i.remove_css_class("active");
                            }
                        }

                    }

                    HyprEvent::WorkspaceMod => {
                        workspace_focus!(active_workspace);

                        let output =
                            String::from_utf8(Command::new("hyprclients").output().expect("failed").stdout).unwrap();

                        let mut curr_windows = vec![];
                        let mut active_window = "".to_string();
                        for line in output.lines() {
                            let split_data: Vec<&str> = line.split("<separator>").collect();
                            let id = split_data.get(2).unwrap();
                            let focus = split_data.get(3).unwrap();
                            if id.parse::<i64>().unwrap().unsigned_abs() as usize == active_workspace {
                                curr_windows.push(split_data.get(0).unwrap().to_string());
                            }
                            if focus.parse::<i32>().unwrap() == 0 {
                                active_window = split_data.get(0).unwrap().to_string();
                            }

                        }

                        let mut start : Option<usize> = None;
                        let mut end : Option<usize> = None;
                        let mut counter = 0;
                        let curr_clients = curr_windows.len();

                        for i in &imgs {
                            i.set_margin_start(0);
                            i.set_margin_end(0);
                            let pt = i.widget_name();
                            if curr_windows.contains(&pt.to_string()) {
                                if start.is_none() {
                                    start = Some(counter);
                                }
                                i.add_css_class("clientchild");
                                animate_pixel_size(i, i.pixel_size(), 36);
                                i.set_size_request(36, 36);
                            }
                            else {
                                if i.css_classes().contains(&GString::from_string_checked("clientchild".to_string()).unwrap()) {
                                    i.remove_css_class("clientchild");
                                    animate_pixel_size(i, i.pixel_size(), 16);
                                    i.set_size_request(16, 16);
                                }
                            }

                            if start.is_some() && start.unwrap() + curr_clients > counter {
                                end = Some(counter);
                            }

                            if active_window == pt {
                                i.add_css_class("active");
                            }
                            else if i.has_css_class("active") {
                                i.remove_css_class("active");
                            }
                            counter += 1;
                        }

                        if start.is_some() {
                            if start.unwrap() != 0 {
                                imgs.get(start.unwrap()).unwrap().set_margin_start(10);
                            }
                            if end.unwrap() != imgs.len() - 1 {
                                imgs.get(end.unwrap()).unwrap().set_margin_end(10);
                            }
                        }

                    }
                    HyprEvent::WindowAddedToActive => {
                        client_counts();

                        // Get rid of the first inner box
                        center_widget.remove(&center_widget.first_child().unwrap());

                        // Re-construct the inner box using the image function
                        imgs = center(&center_widget);

                        workspace_focus!(active_workspace);

                    }
                    HyprEvent::WindowRemovedFromActive => {
                        client_counts();

                        // Get rid of the first inner box
                        center_widget.remove(&center_widget.first_child().unwrap());

                        // Re-construct the inner box using the image function
                        imgs = center(&center_widget);

                        workspace_focus!(active_workspace);
                    }
                }
            }
        });

        // Nix Flake
        glib::MainContext::default().spawn_local(async move {
            loop {

                let mut svg = std::fs::read_to_string("/home/rc/default/assets/nix-snowflake-white.svg").unwrap();


                // Check the flake for uncommitted changes
                let output = Command::new("git")
                    .args(["-C", "/home/rc/flake", "status", "--porcelain"])
                    .output()
                    .unwrap();


                let mut p_changes = "".to_string();

                // Check how many days it has been since the last nix update
                let last_update = String::from_utf8(
                    Command::new("git")
                        .current_dir("/home/rc/flake")
                        .args(["log", "-1", "--format=%ct", "flake.lock"])
                        .output()
                        .unwrap().stdout
                )
                .unwrap();

                let parsed_update = last_update.trim().parse::<i64>();
                let update_ok = parsed_update.is_ok();
                let t_elapsed = chrono::Local::now().timestamp() - parsed_update.unwrap_or(0i64);

                if update_ok && t_elapsed > THREE_DAYS {
                    svg = nix::adjust_nix(svg.as_str(), "#dc2626");
                }
                else if !output.stdout.is_empty() {
                    svg = nix::adjust_nix(svg.as_str(), "#fdb022");
                    let num_files =  String::from_utf8(output.stdout).unwrap().lines().count();
                    p_changes = format!("\nPending Changes: {} file{}", num_files, if num_files > 1 {"s"} else {""});
                }
                else {
                    svg = nix::adjust_nix(svg.as_str(), "#c0caf5");
                }

                let loader = PixbufLoader::with_type("svg").expect("Failed to create loader");
                loader.write(svg.as_bytes()).expect("Failed to load svg");
                loader.close().expect("Failed to close loader");

                let px = loader.pixbuf().expect("Failed to create pixbuf");
                nix.set_from_pixbuf(Some(&px));

                let tooltip = format!("Last Update: {}{}", nix::pretty_time(t_elapsed), p_changes);

                nix.set_tooltip_text(Some(&tooltip));

                glib::timeout_future(Duration::from_millis(500)).await;
            }
        });


    });

    application.run();
}
