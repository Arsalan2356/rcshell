use std::sync::atomic::{AtomicBool, Ordering};

use gtk4 as gtk;
use gtk4::prelude::*;

use crate::subprocess;

/// Set when the last call refreshed the icon database; it's refreshed every other call.
static DB_REFRESHED_LAST: AtomicBool = AtomicBool::new(false);

pub async fn clients() -> Vec<gtk::Image> {
    let mut start_data: Vec<gtk::Image> = vec![];

    let output = subprocess::output(&["hyprclients"]).await;

    let output2 = subprocess::output(&["hypractive"]).await;

    let o2: Vec<&str> = output2.split("<separator>").collect();

    let active_workspace = o2.get(0).unwrap().parse::<usize>().unwrap();

    if !DB_REFRESHED_LAST.fetch_xor(true, Ordering::Relaxed) {
        subprocess::output(&["iconfinderdb"]).await;
    }

    let mut v: Vec<&str> = output.lines().collect();
    v.sort_by_key(|x| {
        let q: Vec<&str> = x.split("<separator>").collect();
        q.get(2).unwrap().parse::<i32>().unwrap()
    });

    let mut fimage: Option<usize> = None;
    let mut eimage: Option<usize> = None;
    let mut first = true;
    let mut i = 0;

    for line in v {
        let split_data: Vec<&str> = line.split("<separator>").collect();
        let l = gtk::Image::new();
        l.set_pixel_size(16);

        let pt = *split_data.get(0).unwrap();
        let init_title = split_data.get(1).unwrap();
        let id = split_data.get(2).unwrap();
        let focus = split_data.get(3).unwrap();
        let title = split_data.get(4).unwrap();
        let window_class = split_data.get(5).unwrap();

        l.add_css_class("baseclient");
        l.set_size_request(16, 16);
        if id.parse::<i64>().unwrap().unsigned_abs() as usize == active_workspace {
            if first {
                fimage = Some(i);
                first = false;
            }
            eimage = Some(i);
            l.add_css_class("clientchild");
            l.set_pixel_size(36);
            l.set_size_request(36, 36);
        }
        if focus.parse::<i32>().unwrap() == 0 {
            l.add_css_class("active");
        }

        let icon = subprocess::output(&["iconfinder", *init_title]).await;

        let i1 = if icon == "" || icon.contains(".svgz") {
            vec!["/home/rc/default/window-icon.svg", "0"]
        } else {
            icon.split("<separator>").collect()
        };

        let icon2 = subprocess::output(&["iconfinder", *window_class]).await;

        let i2 = if icon2 == "" || icon2.contains(".svgz") {
            vec!["/home/rc/default/window-icon.svg", "0"]
        } else {
            icon2.split("<separator>").collect()
        };

        let fic = if i2.get(1).unwrap().trim().parse::<f32>().unwrap()
            - i1.get(1).unwrap().trim().parse::<f32>().unwrap()
            > 0.2
        {
            i2.get(0).unwrap()
        } else {
            i1.get(0).unwrap()
        };

        l.set_from_file(Some(fic));
        l.set_tooltip_text(Some(title));
        l.set_widget_name(pt);
        l.set_margin_start(0);
        l.set_margin_end(0);

        start_data.push(l.clone());
        i += 1;
    }

    if fimage.is_some() {
        if fimage.unwrap() != 0 {
            start_data
                .get(fimage.unwrap())
                .unwrap()
                .set_margin_start(10);
        }
        if eimage.unwrap() != start_data.len() - 1 {
            start_data.get(eimage.unwrap()).unwrap().set_margin_end(10);
        }
    }

    return start_data;
}
