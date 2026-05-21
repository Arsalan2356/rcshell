use crate::host::*;
use gtk4::prelude::*;
use gtk4::{self as gtk, gdk};
use zbus::{Connection, Proxy};

pub async fn host() -> StatusNotifierHost {
    let conn = Connection::session().await.unwrap();
    return StatusNotifierHost::new(conn).await.unwrap();
}

pub fn update_tray(container: &gtk::Box, items: &[&TrayItem]) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| b.service.cmp(&a.service));

    for item in &sorted {
        let image = build_icon(item);
        image.set_pixel_size(20);

        if !item.tooltip.is_empty() {
            image.set_tooltip_text(Some(&item.tooltip));
        }

        attach_gestures(&image, item);
        container.append(&image);
    }
}

fn build_icon(item: &TrayItem) -> gtk::Image {
    let l = {
        if !item.icon_name.is_empty() {
            gtk::Image::from_icon_name(&item.icon_name)
        } else if let Some(pixmap) = item.icon_pixmaps.iter().max_by_key(|p| p.width * p.height) {
            let mut rgba = pixmap.data.clone();
            for chunk in rgba.chunks_mut(4) {
                let (a, r, g, b) = (chunk[0], chunk[1], chunk[2], chunk[3]);
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
                chunk[3] = a;
            }
            let bytes = glib::Bytes::from(&rgba);
            let texture = gdk::MemoryTexture::new(
                pixmap.width,
                pixmap.height,
                gdk::MemoryFormat::R8g8b8a8,
                &bytes,
                (pixmap.width * 4) as usize,
            );
            gtk::Image::from_paintable(Some(&texture))
        } else {
            gtk::Image::from_icon_name("image-missing")
        }
    };

    l.add_css_class("systrayitem");
    l.set_pixel_size(12);
    l.set_size_request(12, 12);
    l.set_valign(gtk::Align::Center);
    l.set_halign(gtk::Align::Center);
    return l;
}

fn attach_gestures(image: &gtk::Image, item: &TrayItem) {
    let service = item.service.clone();

    let left = gtk::GestureClick::new();
    left.set_button(1);
    let svc = service.clone();
    left.connect_released(move |_, _, _, _| {
        let svc = svc.clone();
        glib::spawn_future_local(async move {
            let conn = Connection::session().await.unwrap();
            let (dest, path) = parse_service(&svc);
            if let Ok(proxy) = Proxy::new(&conn, dest, path, "org.kde.StatusNotifierItem").await {
                let _result = proxy.call_method("Activate", &(0i32, 0i32)).await;
            }
        });
    });

    let right = gtk::GestureClick::new();
    right.set_button(3);
    let svc = service.clone();
    right.connect_released(move |_, _, _, _| {
        println!("right click on {svc}");
        let svc = svc.clone();
        glib::spawn_future_local(async move {
            let conn = Connection::session().await.unwrap();
            println!("calling ContextMenu on {svc}");
            let (dest, path) = parse_service(&svc);
            println!("dest={dest} path={path}");
            if let Ok(proxy) = Proxy::new(&conn, dest, path, "org.kde.StatusNotifierItem").await {
                let res = proxy.call_method("ContextMenu", &(0i32, 0i32)).await;
                println!("ContextMenu result: {res:?}");
            }
        });
    });

    image.add_controller(left);
    image.add_controller(right);
}

fn parse_service(service: &str) -> (String, String) {
    if let Some((dest, path)) = service.split_once('/') {
        (dest.to_string(), format!("/{path}"))
    } else {
        (service.to_string(), "/StatusNotifierItem".to_string())
    }
}

pub fn systray() -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    b.add_css_class("systray");
    return b;
}
