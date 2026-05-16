use std::process::Command;

use gtk4 as gtk;
use gtk4::prelude::*;

pub fn clients() -> Vec<gtk::Image> {
    let mut start_data: Vec<gtk::Image> = vec![];

    let output =
        String::from_utf8(Command::new("hyprclients").output().expect("failed").stdout).unwrap();

    let output2 =
        String::from_utf8(Command::new("hypractive").output().expect("failed").stdout).unwrap();

    let o2: Vec<&str> = output2.split("<separator>").collect();

    let active_workspace = o2.get(0).unwrap().parse::<usize>().unwrap();

    let _ = Command::new("iconfinderdb").output().is_ok();

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

        let pt = *split_data.get(0).unwrap();
        let init_title = split_data.get(1).unwrap();
        let id = split_data.get(2).unwrap();
        let focus = split_data.get(3).unwrap();
        let title = split_data.get(4).unwrap();
        let window_class = split_data.get(5).unwrap();

        l.add_css_class("baseclient");
        if id.parse::<usize>().unwrap() == active_workspace {
            if first {
                fimage = Some(i);
                first = false;
            }
            eimage = Some(i);
            l.add_css_class("clientchild");
        }
        if focus.parse::<i32>().unwrap() == 0 {
            l.add_css_class("active");
        }

        let icon = String::from_utf8(
            Command::new("iconfinder")
                .arg(init_title)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();

        let i1 = if icon == "" || icon.contains(".svgz") {
            vec!["/home/rc/default/window-icon.svg", "0"]
        } else {
            icon.split("<separator>").collect()
        };

        let icon2 = String::from_utf8(
            Command::new("iconfinder")
                .arg(window_class)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();

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
