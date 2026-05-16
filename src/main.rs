mod clients;
mod host;
mod ipc;
mod launcher;
mod title;
mod watcher;
mod workspaces;

use std::process::Command;

use gio::prelude::*;
use glib::{self, GString};
use gtk::prelude::*;
use gtk4::{self as gtk, CssProvider, gdk::Display};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use ipc::HyprEvent;
use tokio::sync::mpsc;

fn left(container: &mut gtk::Box) -> (gtk::Box, gtk::Label) {
    let wp = workspaces::workspaces();
    let tbox = title::title();
    container.append(&launcher::launcher());
    container.append(&wp);
    container.append(&tbox);

    (wp, tbox)
}

fn center(container: &gtk::Box) -> (Vec<gtk::Image>, gtk::Box) {
    let imgs = clients::clients();
    let innerbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    for i in &imgs {
        innerbox.append(i);
    }
    container.append(&innerbox);

    (imgs, innerbox)
}

fn right(container: &mut gtk::Box) -> () {}

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
) -> (gtk::Box, gtk::Label, gtk::Box, Vec<gtk::Image>, gtk::Box) {
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
    let (imgs, innerbox) = center(&mut center_widget);

    let mut end_widget = gtk::Box::new(gtk::Orientation::Horizontal, 9);
    end_widget.add_css_class("rbackground");
    right(&mut end_widget);

    root_container.set_start_widget(Some(&start_widget));
    root_container.set_center_widget(Some(&center_widget));
    root_container.set_end_widget(Some(&end_widget));

    root_container.set_vexpand(true);

    window.set_child(Some(&root_container));
    window.add_css_class("bar");
    window.set_vexpand(true);

    window.show();

    (wp, tbox, center_widget, imgs, innerbox)
}

fn main() {
    let application = gtk::Application::new(Some("sh.wmww.gtk-layer-example"), Default::default());

    application.connect_startup(|_| load_css());

    application.connect_activate(|app| {
        let (wp, tbox, center_widget, mut imgs, mut innerbox) = activate(app);

        let (sender, mut receiver) = mpsc::channel(8);

        let _ = ipc::spawn_event_listener(sender);

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
                        (imgs, innerbox) = center(&center_widget);

                        workspace_focus!(active_workspace);

                    }
                    HyprEvent::WindowRemovedFromActive => {
                        client_counts();

                        // Get rid of the first inner box
                        center_widget.remove(&center_widget.first_child().unwrap());

                        // Re-construct the inner box using the image function
                        (imgs, innerbox) = center(&center_widget);

                        workspace_focus!(active_workspace);
                    }
                }
            }
        });
    });

    application.run();
}
