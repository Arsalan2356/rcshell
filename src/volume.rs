use std::process::Command;

use gtk4 as gtk;
use gtk4::prelude::*;

use crate::spawn_reaped;

/// Build the volume label from `wpctl get-volume` output, e.g. "Volume: 0.25 [MUTED]".
pub fn volume_label(v: &str) -> String {
    if v.len() > 8 && v.contains("Volume") {
        // Only the numeric part, at most 4 chars ("0.25", "1.00").
        let num: String = v.get(8..).unwrap_or("").chars().take(4).collect();

        if let Ok(ival) = num.trim().parse::<f32>() {
            return format!(
                "{} {}%",
                if ival > 0.4 { " " } else { " " },
                (ival * 100.0).round()
            );
        }
    }

    "0".to_string()
}

pub fn volume() -> gtk::Label {
    let v = String::from_utf8(
        Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    let container = gtk::Label::new(Some(&volume_label(&v)));
    container.add_css_class("custom_b");
    container.add_css_class("volume");

    // Add event controller
    let click = gtk::GestureClick::new();

    click.connect_released(|_, _, _, _| {
        spawn_reaped(Command::new("pavucontrol"));
    });

    container.add_controller(click);

    container
}
