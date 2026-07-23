use std::process::Command;

use gtk4 as gtk;
use gtk4::prelude::*;

pub fn client_title_wrap(s: String) -> String {
    let v = if s.contains("Zen") {
        format!("🞋 {}", s.get(0..s.rfind("—").unwrap_or(0)).unwrap_or(""))
    } else if s.contains("vim") {
        let index = s.find("vim").unwrap() + 3;
        format!(" {}", s.get(index..).unwrap())
    } else if s.contains("Vesktop") {
        format!(" ")
    } else if s.contains("Thunar") {
        let index = s.find("Thunar").unwrap() - 3;
        format!(" {}", s.get(0..index).unwrap())
    } else {
        s
    };

    if v.len() < 35 {
        return v.trim().to_string();
    } else {
        return format!("{} ...", v.get(0..35).unwrap_or_default())
            .trim()
            .to_string();
    }
}

pub fn title() -> gtk::Label {
    let title = String::from_utf8(
        Command::new("bash")
            .args([
                "-c",
                "hyprctl activeworkspace -j | jq -r '(.lastwindowtitle | tostring)'",
            ])
            .output()
            .expect("failed")
            .stdout,
    )
    .unwrap();

    let container = gtk::Label::new(Some(client_title_wrap(title.clone()).as_str()));
    container.add_css_class("client-title");
    container.set_vexpand(true);
    container.set_tooltip_text(Some(&title.as_str()));

    return container;
}
