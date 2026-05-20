use std::process::Command;

use gtk4 as gtk;
use gtk4::prelude::*;

pub fn volume() -> gtk::Label {
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

    let container = gtk::Label::new(Some(&val));
    container.add_css_class("custom_b");
    container.add_css_class("volume");

    // Add event controller
    let click = gtk::GestureClick::new();

    click.connect_released(|_, _, _, _| {
        let _ = Command::new("pavucontrol").spawn().is_ok();
    });

    container.add_controller(click);

    return container;
}
