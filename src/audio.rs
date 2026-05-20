use std::process::Command;

use gdk_pixbuf::PixbufLoader;
use gtk4::prelude::*;
use gtk4::{self as gtk, EventControllerMotion, EventControllerScroll, EventControllerScrollFlags};

pub fn inject_outline_style(svg: &str, color: &str) -> String {
    let style = format!(
        r#"<style>path,circle,rect,ellipse,polygon,polyline,line{{fill:none!important;stroke:{}!important;stroke-width:1.5px;stroke-linecap:round;stroke-linejoin:round}}</style>"#,
        color
    );

    // Find the closing > of the <svg tag specifically
    if let Some(svg_start) = svg.find("<svg") {
        if let Some(rel_pos) = svg[svg_start..].find('>') {
            let insert_at = svg_start + rel_pos + 1;
            let (before, after) = svg.split_at(insert_at);
            return format!("{}{}{}", before, style, after);
        }
    }
    svg.to_string()
}

pub fn svg_to_img(svg: String) -> gtk::Image {
    let loader = PixbufLoader::with_type("svg").expect("Failed to create loader");
    loader.write(svg.as_bytes()).expect("Failed to load svg");
    loader.close().expect("Failed to close loader");

    let px = loader.pixbuf().expect("Failed to create pixbuf");
    return gtk::Image::from_pixbuf(Some(&px));
}

pub fn get_player_img(index: usize, clients: &Vec<&str>) -> String {
    let currplayer = clients.get(index).unwrap();

    let icon = String::from_utf8(
        Command::new("iconfinder")
            .arg(currplayer)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    let i1 = if icon == "" || icon.contains(".svgz") {
        vec!["/home/rc/default/window-icon.svg", "0"]
    } else {
        icon.split("<separator>").collect()
    };

    return std::fs::read_to_string(i1[0]).unwrap();
}

fn prev_widget() -> gtk::Image {
    let prev_widget = gtk::Image::from_file("/home/rc/default/assets/previous.svg");

    prev_widget
}

fn next_widget() -> gtk::Image {
    let next_widget = gtk::Image::from_file("/home/rc/default/assets/next.svg");

    next_widget
}

fn play_widget(container: &gtk::Revealer, clients: &Vec<&str>) -> gtk::Image {
    let curr_index = container.widget_name().parse::<i32>().unwrap();

    let ci = {
        if curr_index < 0 {
            container.set_widget_name("0");
            0
        } else if curr_index > (clients.len() - 1) as i32 {
            container.set_widget_name((clients.len() - 1).to_string().as_str());
            clients.len() - 1
        } else {
            curr_index as usize
        }
    };

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

    let play_widget = gtk::Image::from_file(format!(
        "/home/rc/default/assets/{}.svg",
        if is_playing { "pause" } else { "play" }
    ));
    play_widget.set_pixel_size(22);

    play_widget
}

macro_rules! setup_controllers {
    ($container: ident, $prev_widget: ident, $play_widget: ident, $next_widget: ident) => {
        let click_controller = GestureClick::new();
        let cl = $container.clone();

        click_controller.connect_pressed(move |_, _, _, _| {
            let output =
                String::from_utf8(Command::new("playerctl").arg("-l").output().unwrap().stdout)
                    .unwrap();

            let clients: Vec<&str> = output.split("\n").collect();

            let curr_index = cl.widget_name().parse::<i32>().unwrap();

            let ci = {
                if curr_index < 0 {
                    cl.set_widget_name("0");
                    0
                } else if curr_index > (clients.len() - 1) as i32 {
                    cl.set_widget_name((clients.len() - 1).to_string().as_str());
                    clients.len() - 1
                } else {
                    curr_index as usize
                }
            };

            let _ = Command::new("playerctl")
                .args(["previous", "-p", clients.get(ci).unwrap()])
                .spawn();
        });

        $prev_widget.add_controller(click_controller);

        let click_controller = GestureClick::new();
        let cl = $container.clone();

        click_controller.connect_pressed(move |_, _, _, _| {
            let output =
                String::from_utf8(Command::new("playerctl").arg("-l").output().unwrap().stdout)
                    .unwrap();

            let clients: Vec<&str> = output.split("\n").collect();

            let curr_index = cl.widget_name().parse::<i32>().unwrap();

            let ci = {
                if curr_index < 0 {
                    cl.set_widget_name("0");
                    0
                } else if curr_index > (clients.len() - 1) as i32 {
                    cl.set_widget_name((clients.len() - 1).to_string().as_str());
                    clients.len() - 1
                } else {
                    curr_index as usize
                }
            };

            let _ = Command::new("playerctl")
                .args(["play-pause", "-p", clients.get(ci).unwrap()])
                .spawn();
        });

        $play_widget.add_controller(click_controller);

        let click_controller = GestureClick::new();
        let cl = $container.clone();

        click_controller.connect_pressed(move |_, _, _, _| {
            let output =
                String::from_utf8(Command::new("playerctl").arg("-l").output().unwrap().stdout)
                    .unwrap();

            let clients: Vec<&str> = output.split("\n").collect();

            let curr_index = cl.widget_name().parse::<i32>().unwrap();

            let ci = {
                if curr_index < 0 {
                    cl.set_widget_name("0");
                    0
                } else if curr_index > (clients.len() - 1) as i32 {
                    cl.set_widget_name((clients.len() - 1).to_string().as_str());
                    clients.len() - 1
                } else {
                    curr_index as usize
                }
            };

            let _ = Command::new("playerctl")
                .args(["next", "-p", clients.get(ci).unwrap()])
                .spawn();
        });

        $next_widget.add_controller(click_controller);
    };
}

pub(crate) use setup_controllers;

pub fn audio() -> (gtk::Revealer, gtk::Image, gtk::Image, gtk::Image) {
    let container = gtk::Revealer::new();
    // Monitor current player using container's name
    container.set_widget_name("0");
    let c_clone = container.clone();

    let output =
        String::from_utf8(Command::new("playerctl").arg("-l").output().unwrap().stdout).unwrap();

    let clients: Vec<&str> = output.split("\n").collect();

    let inner_revealer = gtk::Revealer::new();

    let cbox = gtk::CenterBox::new();
    cbox.set_width_request(84);

    let prev = prev_widget();
    let play = play_widget(&c_clone, &clients);
    let next = next_widget();

    cbox.set_start_widget(Some(&prev));
    cbox.set_center_widget(Some(&play));
    cbox.set_end_widget(Some(&next));

    inner_revealer.set_child(Some(&cbox));
    inner_revealer.set_transition_type(gtk::RevealerTransitionType::SlideLeft);

    let motion_controller = EventControllerMotion::new();

    let revealer_clone = inner_revealer.clone();

    motion_controller.connect_enter(move |_, _, _| {
        revealer_clone.set_reveal_child(true);
    });

    let revealer_clone = inner_revealer.clone();
    motion_controller.connect_leave(move |_| {
        revealer_clone.set_reveal_child(false);
    });

    let base_img = get_player_img(0, &clients);

    let process = svg_to_img(inject_outline_style(&base_img, "#c0caf5"));

    let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);

    scroll_controller.connect_scroll(move |_, _, dy| {
        c_clone.set_widget_name(
            (c_clone.widget_name().as_str().parse::<i32>().unwrap() + {
                if dy > 0.0 { 1 } else { -1 }
            })
            .to_string()
            .as_str(),
        );

        glib::Propagation::Proceed
    });

    process.add_controller(scroll_controller);

    container.add_controller(motion_controller);

    let inbox = gtk::Box::new(gtk::Orientation::Horizontal, 14);

    container.set_child(Some(&inbox));
    container.set_transition_type(gtk::RevealerTransitionType::SlideLeft);

    if clients.len() > 0 {
        container.set_reveal_child(true);
    }

    inbox.append(&inner_revealer);
    inbox.append(&process);
    inbox.append(&gtk::Label::new(Some(&"|")));

    (container, prev, play, next)
}
