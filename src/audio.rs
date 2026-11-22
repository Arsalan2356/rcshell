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

/// Player names from `playerctl -l` output, without the trailing empty line.
pub fn player_list(output: &str) -> Vec<&str> {
    output.lines().filter(|l| !l.trim().is_empty()).collect()
}

/// Clamp the player index stored in the container's widget name to the current
/// player list. Returns `None` when there are no players.
pub fn current_player(container: &gtk::Revealer, clients: &[&str]) -> Option<usize> {
    if clients.is_empty() {
        return None;
    }

    let curr_index = container.widget_name().parse::<i32>().unwrap_or(0);
    let last = clients.len() - 1;

    if curr_index < 0 {
        container.set_widget_name("0");
        Some(0)
    } else if curr_index as usize > last {
        container.set_widget_name(last.to_string().as_str());
        Some(last)
    } else {
        Some(curr_index as usize)
    }
}

pub fn get_player_img(index: usize, clients: &[&str]) -> String {
    let Some(currplayer) = clients.get(index) else {
        return std::fs::read_to_string("/home/rc/default/window-icon.svg").unwrap();
    };

    let icon = String::from_utf8(
        Command::new("iconfinder")
            .arg(currplayer)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    player_img_from(&icon)
}

/// Read the SVG that `iconfinder` output points to, or the default icon.
pub fn player_img_from(icon: &str) -> String {
    let i1 = if icon == "" || icon.contains(".svgz") {
        vec!["/home/rc/default/window-icon.svg", "0"]
    } else {
        icon.split("<separator>").collect()
    };

    std::fs::read_to_string(i1[0]).unwrap()
}

fn prev_widget() -> gtk::Image {
    let prev_widget = gtk::Image::from_file("/home/rc/default/assets/previous.svg");

    prev_widget
}

fn next_widget() -> gtk::Image {
    let next_widget = gtk::Image::from_file("/home/rc/default/assets/next.svg");

    next_widget
}

fn play_widget(container: &gtk::Revealer, clients: &[&str]) -> gtk::Image {
    let is_playing = match current_player(container, clients) {
        Some(ci) => String::from_utf8(
            Command::new("playerctl")
                .args(["status", "-p", clients[ci]])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .contains("Playing"),
        None => false,
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

            let clients = crate::audio::player_list(&output);

            let Some(ci) = crate::audio::current_player(&cl, &clients) else {
                return;
            };

            let mut cmd = Command::new("playerctl");
            cmd.args(["previous", "-p", clients[ci]]);
            crate::spawn_reaped(cmd);
        });

        $prev_widget.add_controller(click_controller);

        let click_controller = GestureClick::new();
        let cl = $container.clone();

        click_controller.connect_pressed(move |_, _, _, _| {
            let output =
                String::from_utf8(Command::new("playerctl").arg("-l").output().unwrap().stdout)
                    .unwrap();

            let clients = crate::audio::player_list(&output);

            let Some(ci) = crate::audio::current_player(&cl, &clients) else {
                return;
            };

            let mut cmd = Command::new("playerctl");
            cmd.args(["play-pause", "-p", clients[ci]]);
            crate::spawn_reaped(cmd);
        });

        $play_widget.add_controller(click_controller);

        let click_controller = GestureClick::new();
        let cl = $container.clone();

        click_controller.connect_pressed(move |_, _, _, _| {
            let output =
                String::from_utf8(Command::new("playerctl").arg("-l").output().unwrap().stdout)
                    .unwrap();

            let clients = crate::audio::player_list(&output);

            let Some(ci) = crate::audio::current_player(&cl, &clients) else {
                return;
            };

            let mut cmd = Command::new("playerctl");
            cmd.args(["next", "-p", clients[ci]]);
            crate::spawn_reaped(cmd);
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

    let clients = player_list(&output);

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

    if !clients.is_empty() {
        container.set_reveal_child(true);
    }

    inbox.append(&inner_revealer);
    inbox.append(&process);
    inbox.append(&gtk::Label::new(Some(&"|")));

    (container, prev, play, next)
}
