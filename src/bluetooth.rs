use std::f32;
use std::process::Command;

use gtk4 as gtk;
use gtk4::prelude::*;

pub const ICONS: [&str; 10] = ["󰥈 ", "󰥆 ", "󰥅 ", "󰥄 ", "󰥃 ", "󰥂 ", "󰥁 ", "󰥀 ", "󰤿 ", "󰤾 "];

pub fn bluetooth() -> gtk::Label {
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
                ICONS[index as usize],
                q.parse::<f32>().unwrap().round()
            )
        }
    };

    let container = gtk::Label::new(Some(&val));
    container.set_valign(gtk::Align::Center);
    container.add_css_class("custom_b");

    // Add event controller
    let click = gtk::GestureClick::new();

    click.connect_released(|_, _, _, _| {
        let _ = Command::new("foot")
            .args(["-e", "bluetuith"])
            .spawn()
            .is_ok();
    });

    container.add_controller(click);

    return container;
}
