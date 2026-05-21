use gtk4::gdk::Rectangle;
use phf::phf_map;
use std::process::Command;

use gtk4 as gtk;
use gtk4::prelude::*;

static PROFILE_MAP: phf::Map<&'static str, &'static str> = phf_map! {
        "a2dp-sink" => "AAC",
        "a2dp-sink-sbc" => "SBC",
        "a2dp-sink-sbc_xq" =>  "XQ",
        "headset-head-unit" => "VC",
        "headset-head-unit-cvsd" => "SD",
};

pub fn context_menu(label: &gtk::Label, name: String) {
    // Add event controller
    let click = gtk::GestureClick::new();

    let popover = gtk::Popover::new();
    popover.set_parent(label);

    click.set_button(3);
    click.connect_pressed(move |_, _, x, y| {

        while let Some(_) = popover.child() {
            popover.set_child(gtk::Widget::NONE);
            break;
        }

        let args = [
            "-c",
            "pactl -f json list cards | jq -r '.[] | select(.name | contains(\"bluez\")) | .profiles | keys | join(\",
            \")'",
        ];

        let profiles =
            String::from_utf8(Command::new("sh").args(args).output().unwrap().stdout).unwrap();

        let menu_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        menu_box.add_css_class("menu");

        popover.set_child(Some(&menu_box));
        popover.set_has_arrow(false); // no speech-bubble arrow

        for p in profiles.split(",").collect::<Vec<&str>>() {
            if p.contains("off") {
                continue;
            }
            let q = p.trim().to_string();
            let btn = gtk::Button::with_label(PROFILE_MAP.get(&q).unwrap());
            btn.set_widget_name(&q);
            btn.add_css_class("flat"); // flat removes button chrome, looks like a menu item
            menu_box.append(&btn);

            let n2 = name.clone();

            let weak_ref = popover.downgrade();

            btn.connect_clicked(move |btn| {
                let _status = Command::new("pactl")
                    .args(["set-card-profile", &n2, btn.widget_name().as_str()])
                    .status()
                    .is_ok();

                // Dismiss the popover by walking up to it
                if let Some(popover) = weak_ref.upgrade() {
                    popover.popdown();
                }
            });
        }


        popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.popup();
    });

    label.add_controller(click);
}

pub fn profile() -> gtk::Label {
    let args = [
        "-c",
        "pactl -f json list cards | jq -r '.[] | select(.name | contains(\"bluez\")) | .name'",
    ];

    let name = String::from_utf8(Command::new("sh").args(args).output().unwrap().stdout).unwrap();

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

    let container = gtk::Label::new(Some(&label));
    container.add_css_class("custom_b");
    container.add_css_class("profile");

    if label == "VC" {
        container.add_css_class("vc");
    }

    context_menu(&container, name.trim().to_string());

    container.set_widget_name(name.trim());

    return container;
}
