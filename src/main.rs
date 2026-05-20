mod audio;
mod bluetooth;
mod clients;
mod host;
mod ipc;
mod logout;
mod profile;
mod time;
mod title;
mod volume;
mod watcher;
mod workspaces;

use std::{process::Command, time::Duration};

use crate::audio::setup_controllers;
use audio::{get_player_img, inject_outline_style};
use gdk_pixbuf::PixbufLoader;
use gio::prelude::*;
use glib::{self, GString};
use gtk::prelude::*;
use gtk4::GestureClick;
use gtk4::{self as gtk, CssProvider, gdk::Display};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use ipc::HyprEvent;
use tokio::sync::mpsc;

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
    gtk::Label,
    gtk::Label,
) {
    let (ad, prev, play, next) = audio::audio();
    container.append(&ad);
    let pf = profile::profile();
    container.append(&pf);
    let vol = volume::volume();
    container.append(&vol);
    let bt = bluetooth::bluetooth();
    container.append(&bt);
    let time = time::time();
    container.append(&time);

    container.append(&logout::logout());

    (ad, prev, play, next, vol, bt, pf, time)
}

fn load_css() {
    let css = grass::from_path("src/style.scss", &grass::Options::default()).unwrap();

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
    gtk::Label,
    gtk::Label,
) {
    let window = gtk::ApplicationWindow::new(application);

    // Before the window is first realized, set it up to be a layer surface
    window.init_layer_shell();

    // Display above normal windows
    window.set_layer(Layer::Overlay);

    window.set_size_request(100, -1);

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

    let mut end_widget = gtk::Box::new(gtk::Orientation::Horizontal, 9);
    end_widget.add_css_class("rbackground");
    let (ad, prev, play, next, vol, bt, pf, time) = right(&mut end_widget);

    root_container.set_start_widget(Some(&start_widget));
    root_container.set_center_widget(Some(&center_widget));
    root_container.set_end_widget(Some(&end_widget));

    root_container.set_vexpand(true);

    window.set_child(Some(&root_container));
    window.add_css_class("bar");
    window.set_vexpand(true);

    window.show();

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
        pf,
        time,
    )
}

fn main() {
    let application = gtk::Application::new(Some("sh.rc"), Default::default());

    application.connect_startup(|_| load_css());

    application.connect_activate(|app| {
        let (wp, tbox, center_widget, mut imgs, ad, prev, play, next, vol, bt, pf, time) = activate(app);

        setup_controllers!(ad, prev, play, next);


        let (sender, mut receiver) = mpsc::channel(8);

        let _ = ipc::spawn_event_listener(sender);

        glib::MainContext::default().spawn_local(async move {
            loop {
                time.set_label(&time::current_time());

                glib::timeout_future(Duration::from_millis(500)).await;
            }
        });



        // Background tasks
        glib::MainContext::default().spawn_local(async move {
            loop {
                // Audio Player
                let output =
                    String::from_utf8(Command::new("playerctl").arg("-l").output().unwrap().stdout).unwrap();

                let clients: Vec<&str> = output.split("\n").collect();

                if clients.len() == 0 {
                    ad.set_reveal_child(false);
                }

                let curr_index = ad.widget_name().parse::<i32>().unwrap();
                let ci = {
                    if curr_index < 0 {
                        ad.set_widget_name("0");
                        0
                    } else if curr_index > (clients.len() - 1) as i32 {
                        ad.set_widget_name((clients.len() - 1).to_string().as_str());
                        clients.len() - 1
                    }
                    else {
                        curr_index as usize
                    }
                };
                let inbox = ad.child().unwrap().downcast::<gtk::Box>().unwrap();

                let is_playing = {
                    let s = String::from_utf8(
                        Command::new("playerctl")
                            .args(["status", "-p", clients.get(ci).unwrap()])
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

                play_widget.set_from_file(Some(format!(
                        "/home/rc/default/assets/{}.svg",
                        if is_playing { "pause" } else { "play" }
                    )));
                play_widget.set_pixel_size(22);

                let img = inbox.first_child().unwrap().next_sibling().unwrap().downcast::<gtk::Image>().unwrap();
                let base_img = get_player_img(ci, &clients);
                let svg = inject_outline_style(&base_img, "#c0caf5");
                let loader = PixbufLoader::with_type("svg").expect("Failed to create loader");
                loader.write(svg.as_bytes()).expect("Failed to load svg");
                loader.close().expect("Failed to close loader");

                let px = loader.pixbuf().expect("Failed to create pixbuf");
                img.set_from_pixbuf(Some(&px));


                // Volume
                let v = String::from_utf8(
                    Command::new("wpctl")
                        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
                        .output()
                        .unwrap()
                        .stdout,
                )
                .unwrap();

                let val = {
                    if v == "0" {
                        "0".to_string()
                    } else {
                        let ival = v.get(8..).unwrap().trim().parse::<f32>().unwrap();
                        format!(
                            "{} {}%",
                            if ival > 0.4 { " " } else { " " },
                            (ival * 100.0).round()
                        )
                    }
                };

                vol.set_text(&val);


                // Bluetooth
                let v = String::from_utf8(Command::new("btbattery").output().unwrap().stdout).unwrap();


                let val = {
                    if v == "" {
                        "󰂯".to_string()
                    } else {
                        let q = v.trim();
                        let index = f32::max(
                            f32::min((100.0 - q.parse::<f32>().unwrap()) / 10.0, 9.0),
                            0.0,
                        );

                        format!(
                            "{} {}%",
                            bluetooth::ICONS[index as usize],
                            q.parse::<f32>().unwrap().round()
                        )
                    }
                };

                bt.set_text(&val);



                // Audio Profile
                let args = [
                    "-c",
                    "pactl -f json list cards | jq -r '.[] | select(.name | contains(\"bluez\")) | .active_profile'",
                ];

                let active_profile =
                    String::from_utf8(Command::new("sh").args(args).output().unwrap().stdout).unwrap();

                let label = if active_profile.contains("sbc_xq") {
                    "XQ"
                } else if active_profile.contains("headset") && !active_profile.contains("cvsd") {
                    "VC"
                } else {
                    "UN"
                };
                pf.set_label(label);

                glib::timeout_future(Duration::from_secs(1)).await;
            }
        });

        // Hyprland events
        glib::MainContext::default().spawn_local(async move {

            macro_rules! workspace_focus {
                ($active : ident) => {
                    let output = String::from_utf8(Command::new("bash")
                            .args(["-c", "hyprctl activeworkspace -j | jq -r '(.id | tostring) + \",\" + (.windows | tostring)'"])
                            .output()
                            .expect("failed").stdout).unwrap();

                    let $active = output.get(0..1).unwrap().parse::<usize>().unwrap();
                    let num_windows = output.get(2..3).unwrap().parse::<usize>().unwrap();

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
                    let split_data: Vec<&str> = line.split(",").collect();
                    let num: usize = split_data.get(0).unwrap().parse::<usize>().unwrap();
                    let windows: usize = split_data.get(1).unwrap().parse::<usize>().unwrap();
                    data[num - 1] = windows;
                }

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
            };


            while let Some(msg) = receiver.recv().await {
                match msg {
                    HyprEvent::ActiveWindow => {
                        let title = String::from_utf8(Command::new("bash")
                                .args(["-c", "hyprctl activeworkspace -j | jq -r '(.lastwindowtitle | tostring)'"])
                                .output()
                                .expect("failed").stdout).unwrap();

                        tbox.set_text(&title::client_title_wrap(title.clone()));
                        tbox.set_tooltip_text(Some(&title.as_str()));

                        let pt = String::from_utf8(Command::new("bash")
                                .args(["-c", "hyprctl activeworkspace -j | jq -r '(.lastwindow | tostring)'"])
                                .output()
                                .expect("failed").stdout).unwrap();


                        for i in &imgs {
                            if i.widget_name().as_str() == pt.trim() {
                                i.add_css_class("active");
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
                            if id.parse::<usize>().unwrap() == active_workspace {
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
                            }
                            else {
                                if i.css_classes().contains(&GString::from_string_checked("clientchild".to_string()).unwrap()) {
                                    i.remove_css_class("clientchild");
                                }
                            }

                            if start.is_some() && start.unwrap() + curr_clients > counter {
                                end = Some(counter);
                            }

                            if active_window == pt {
                                i.add_css_class("active");
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
    });

    application.run();
}
