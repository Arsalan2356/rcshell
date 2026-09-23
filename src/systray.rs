use crate::host::*;
use gtk4::prelude::*;
use gtk4::{self as gtk, gdk};
use std::collections::HashMap;
use std::process::Command;
use zbus::{Connection, Proxy};
use zvariant::OwnedValue;

type RawMenuItem = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

type RawGetLayout = (u32, RawMenuItem);

struct MenuAction {
    name: String,
    id: i32,
    enabled: bool,
}

pub async fn host() -> StatusNotifierHost {
    let conn = Connection::session().await.unwrap();
    return StatusNotifierHost::new(conn).await.unwrap();
}

pub fn update_tray(
    container: &gtk::Box,
    container_popover: &gtk::PopoverMenu,
    items: &[&TrayItem],
) {
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

        attach_gestures(&image, item, container_popover);
        container.append(&image);
    }
}

fn build_icon(item: &TrayItem) -> gtk::Image {
    let l = {
        if !item.icon_name.is_empty() {
            let path = String::from_utf8(
                Command::new("iconfinder")
                    .arg(&item.icon_name)
                    .output()
                    .unwrap()
                    .stdout,
            )
            .unwrap();

            let i1 = if path == "" || path.contains(".svgz") {
                vec!["/home/rc/default/window-icon.svg", "0"]
            } else {
                path.split("<separator>").collect()
            };

            gtk::Image::from_file(i1[0])
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
    l.set_tooltip_text(Some(&item.tooltip));
    return l;
}

fn attach_gestures(image: &gtk::Image, item: &TrayItem, popover: &gtk::PopoverMenu) {
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

    // TODO Rewrite the system here to keep a menu for clients and show that when we're right-clicking
    // Or call Activate/SecondaryActivate

    let right = gtk::GestureClick::new();
    right.set_button(3);
    let i = item.clone();
    let p = popover.clone();
    right.connect_released(move |gesture, _, _, _| {
        let Some(widget) = gesture.widget() else {
            return;
        };
        let i = i.clone();
        let p = p.clone();

        glib::spawn_future_local(async move {
            let conn = Connection::session().await.unwrap();

            let (model, action_group) = match create_tray_menu(&conn, &i).await {
                Ok(result) => result,
                Err(err) => {
                    eprintln!("Failed to create tray menu: {err}");
                    return;
                }
            };

            p.set_menu_model(Some(&model));
            p.set_height_request(model.n_items() * 55);

            p.insert_action_group("menu", None::<&gio::ActionGroup>);
            p.insert_action_group("menu", Some(&action_group));

            if let Some(parent) = p.parent() {
                if let Some(point) = widget.compute_point(
                    &parent,
                    &gtk::graphene::Point::new(0.0, widget.height() as f32),
                ) {
                    let rect = gtk::gdk::Rectangle::new(
                        point.x() as i32,
                        point.y() as i32,
                        widget.width(),
                        1,
                    );

                    p.set_pointing_to(Some(&rect));
                }
            }

            p.popup();
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

pub fn systray() -> (gtk::Box, gtk::PopoverMenu) {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let popover = gtk::PopoverMenu::from_model(None::<&gio::MenuModel>);
    popover.set_has_arrow(false);
    b.add_css_class("systray");
    return (b, popover);
}

async fn get_menu_layout(
    conn: &Connection,
    service: &str,
    menu_path: &str,
) -> zbus::Result<RawGetLayout> {
    let (dest, _) = parse_service(service);

    let proxy = Proxy::new(conn, dest, menu_path, "com.canonical.dbusmenu").await?;

    let result: RawGetLayout = proxy
        .call("GetLayout", &(0i32, -1i32, Vec::<String>::new()))
        .await?;

    Ok(result)
}
fn property_string(props: &HashMap<String, OwnedValue>, name: &str) -> Option<String> {
    props.get(name).and_then(|v| v.clone().try_into().ok())
}

fn property_bool(props: &HashMap<String, OwnedValue>, name: &str) -> Option<bool> {
    props.get(name).and_then(|v| v.clone().try_into().ok())
}

fn build_menu(item: &RawMenuItem, actions: &mut Vec<MenuAction>) -> gio::Menu {
    let (_, _, children) = item;

    let menu = gio::Menu::new();
    let mut section = gio::Menu::new();

    for child in children {
        let child: RawMenuItem = match child.clone().try_into() {
            Ok(child) => child,
            Err(err) => {
                eprintln!("Failed to parse DBusMenu child: {err}");
                continue;
            }
        };

        let (id, props, _) = &child;

        if property_bool(props, "visible") == Some(false) {
            continue;
        }

        if property_string(props, "type").as_deref() == Some("separator") {
            if section.n_items() > 0 {
                menu.append_section(None, &section);
                section = gio::Menu::new();
            }

            continue;
        }

        let label = property_string(props, "label").unwrap_or_default();

        let action_name = format!("item_{id}");

        actions.push(MenuAction {
            name: action_name.clone(),
            id: *id,
            enabled: property_bool(props, "enabled").unwrap_or(true),
        });

        if property_string(props, "children-display").as_deref() == Some("submenu") {
            let submenu = build_menu(&child, actions);

            section.append_submenu(Some(&label), &submenu);
        } else {
            section.append(Some(&label), Some(&format!("menu.{action_name}")));
        }
    }

    if section.n_items() > 0 {
        menu.append_section(None, &section);
    }

    menu
}

async fn create_tray_menu(
    conn: &Connection,
    item: &TrayItem,
) -> anyhow::Result<(gio::Menu, gio::SimpleActionGroup)> {
    let menu_path = item
        ._menu_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("No DBusMenu"))?;

    let (_, root) = get_menu_layout(conn, &item.service, menu_path).await?;

    let mut actions = Vec::new();
    let model = build_menu(&root, &mut actions);

    let action_group = gio::SimpleActionGroup::new();

    for action in actions {
        let gtk_action = gio::SimpleAction::new(&action.name, None);

        gtk_action.set_enabled(action.enabled);

        let service = item.service.clone();
        let menu_path = menu_path.to_owned();
        let id = action.id;

        gtk_action.connect_activate(move |_, _| {
            let service = service.clone();
            let menu_path = menu_path.clone();

            glib::spawn_future_local(async move {
                let conn = match Connection::session().await {
                    Ok(conn) => conn,
                    Err(err) => {
                        eprintln!("DBus connection failed: {err}");
                        return;
                    }
                };

                if let Err(err) = send_menu_event(&conn, &service, &menu_path, id).await {
                    eprintln!("DBusMenu Event failed: {err}");
                }
            });
        });

        action_group.add_action(&gtk_action);
    }

    Ok((model, action_group))
}
async fn send_menu_event(
    conn: &Connection,
    service: &str,
    menu_path: &str,
    id: i32,
) -> zbus::Result<()> {
    let (dest, _) = parse_service(service);
    let proxy = Proxy::new(conn, dest, menu_path, "com.canonical.dbusmenu").await?;

    let data = zvariant::OwnedValue::from(0u32);

    proxy
        .call_method("Event", &(id, "clicked", data, 0u32))
        .await?;

    Ok(())
}
