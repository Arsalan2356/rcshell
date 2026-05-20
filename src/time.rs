use gtk4 as gtk;
use gtk4::prelude::*;

use chrono::prelude::*;

pub fn current_time() -> String {
    let local = Local::now();

    let t = local.format("%a %d/%m/%Y %H:%M:%S").to_string();

    return t;
}

pub fn time() -> gtk::Label {
    let container = gtk::Label::new(Some(&current_time()));
    container.add_css_class("custom_b");
    container.add_css_class("volume");
    let popover = gtk::Popover::new();
    popover.set_has_arrow(false);
    popover.set_parent(&container);
    let calendar = gtk::Calendar::new();
    popover.set_child(Some(&calendar));

    // Add event controller
    let click = gtk::GestureClick::new();

    click.connect_pressed(move |_, _, _, _| {
        if popover.is_visible() {
            popover.popdown();
        } else {
            popover.popup();
        }
    });

    container.add_controller(click);

    return container;
}
