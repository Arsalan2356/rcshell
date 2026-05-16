use std::process::Command;

use gtk4 as gtk;
use gtk4::prelude::*;

pub fn launcher() -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    container.add_css_class("custom_b");

    let icon = gtk::Image::new();
    icon.set_from_file(Some("/home/rc/default/assets/gamma.svg"));
    icon.set_pixel_size(26);
    icon.set_tooltip_text(Some("Application Drawer"));
    icon.add_css_class("launcher");

    container.append(&icon);
    // Add event controller
    let click = gtk::GestureClick::new();

    click.connect_released(|_, _, _, _| {
        let _ = Command::new("nwg-drawer").spawn().is_ok();
    });

    container.add_controller(click);
    container.set_vexpand(true);

    return container;
}
