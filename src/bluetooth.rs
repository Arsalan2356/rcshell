use std::f32;
use std::process::Command;

use gtk4 as gtk;
use gtk4::prelude::*;

use crate::spawn_reaped;

pub const ICONS: [&str; 10] = ["󰥈 ", "󰥆 ", "󰥅 ", "󰥄 ", "󰥃 ", "󰥂 ", "󰥁 ", "󰥀 ", "󰤿 ", "󰤾 "];

/// Build the battery label from `btbattery` output. Only the first device is used.
pub fn battery_label(v: &str) -> String {
    let q = v.lines().next().unwrap_or("").trim();

    let Ok(pct) = q.parse::<f32>() else {
        return "󰂯".to_string();
    };

    let index = ((100.0 - pct) / 10.0).clamp(0.0, 9.0);

    format!("{} {}%", ICONS[index as usize], pct.round())
}

pub fn bluetooth() -> gtk::Label {
    let v = String::from_utf8(Command::new("btbattery").output().unwrap().stdout).unwrap();

    let container = gtk::Label::new(Some(&battery_label(&v)));
    container.set_valign(gtk::Align::Center);
    container.add_css_class("custom_b");

    // Add event controller
    let click = gtk::GestureClick::new();

    click.connect_released(|_, _, _, _| {
        let mut cmd = Command::new("foot");
        cmd.args(["-e", "bluetuith"])
            .stderr(std::process::Stdio::null());
        spawn_reaped(cmd);
    });

    container.add_controller(click);

    container
}
