use std::process::Command;

use gtk4::{self as gtk, EventControllerScroll};
use gtk4::{EventControllerScrollFlags, prelude::*};

use crate::ipc::{dispatch_down, dispatch_up, dispatch_workspace};

pub fn workspaces() -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    container.add_css_class("workspaces");

    let mut start_data: [usize; 8] = [0; 8];

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
        start_data[num - 1] = windows;
    }

    let output = String::from_utf8(Command::new("bash")
            .args(["-c", "hyprctl activeworkspace -j | jq -r '(.id | tostring) + \",\" + (.windows | tostring)'"])
            .output()
            .expect("failed").stdout).unwrap();

    let active_workspace = output.get(0..1).unwrap().parse::<usize>().unwrap();

    for i in 1..=8 {
        let windows = start_data[i - 1];
        let child = gtk::Label::new(Some(""));
        child.set_use_markup(true);
        let mkp = if windows > 0 {
            format!(
                "<span size='x-small' rise='10000'>{}</span><span rise='3000'>/</span><span size='x-small'>{}</span>",
                i, windows,
            )
        } else {
            format!("{}", i)
        };
        child.set_markup(mkp.as_str());
        let click = gtk::GestureClick::new();

        click.connect_released(move |_, _, _, _| {
            dispatch_workspace(i);
        });

        child.add_css_class("workspacechild");
        if windows > 0 {
            child.add_css_class("occupied");
        }
        if i == active_workspace {
            child.add_css_class("active");
        }

        child.add_controller(click);
        container.append(&child);
    }

    container.set_vexpand(true);

    let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);

    scroll_controller.connect_scroll(move |_, _, dy| {
        if dy > 0.0 {
            dispatch_up();
        } else {
            dispatch_down();
        };

        glib::Propagation::Proceed
    });

    container.add_controller(scroll_controller);

    return container;
}
