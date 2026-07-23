use gtk4 as gtk;

use crate::audio::svg_to_img;

pub fn nix() -> gtk::Image {
    let svg = adjust_nix(
        std::fs::read_to_string("/home/rc/default/assets/nix-snowflake-white.svg")
            .unwrap()
            .as_str(),
        "#c0caf5",
    );

    let img = svg_to_img(svg);

    img.set_pixel_size(20);

    img
}

pub fn adjust_nix(svg: &str, arg: &str) -> String {
    svg.replace("stop-color:#ffffff", format!("stop-color:{arg}").as_str())
}

pub fn pretty_time(dur: i64) -> String {
    let seconds = dur;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    let remaining_hours = hours % 24;
    let remaining_minutes = minutes % 60;

    let mut parts = Vec::new();

    if days > 0 {
        parts.push(format!("{} day{}", days, if days == 1 { "" } else { "s" }));
    }
    if remaining_hours > 0 {
        parts.push(format!(
            "{} hour{}",
            remaining_hours,
            if remaining_hours == 1 { "" } else { "s" }
        ));
    }
    if remaining_minutes > 0 {
        parts.push(format!(
            "{} minute{}",
            remaining_minutes,
            if remaining_minutes == 1 { "" } else { "s" }
        ));
    }

    if parts.is_empty() {
        return "just now".to_string();
    }

    format!("{} ago", parts.join(" "))
}
