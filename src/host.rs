use futures::StreamExt;
use std::collections::HashMap;
use zbus::zvariant::Structure;
use zbus::{Connection, Proxy};

#[derive(Debug)]
pub struct IconPixmap {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct TrayItem {
    pub service: String,
    pub icon_name: String,
    pub icon_pixmaps: Vec<IconPixmap>,
    pub status: String,
    pub tooltip: String,
    pub menu_path: Option<String>,
}

pub struct StatusNotifierHost {
    conn: Connection,
    items: HashMap<String, TrayItem>,
}

impl StatusNotifierHost {
    pub async fn new(conn: Connection) -> anyhow::Result<Self> {
        let mut host = Self {
            conn,
            items: HashMap::new(),
        };
        host.register().await?;
        host.fetch_all_items().await;
        Ok(host)
    }

    async fn register(&self) -> anyhow::Result<()> {
        let watcher = self.watcher_proxy().await?;
        watcher
            .call_method(
                "RegisterStatusNotifierHost",
                &("org.kde.StatusNotifierHost",),
            )
            .await?;
        Ok(())
    }

    async fn watcher_proxy(&self) -> anyhow::Result<Proxy<'_>> {
        Ok(Proxy::new(
            &self.conn,
            "org.kde.StatusNotifierWatcher",
            "/StatusNotifierWatcher",
            "org.kde.StatusNotifierWatcher",
        )
        .await?)
    }

    async fn fetch_all_items(&mut self) {
        let Ok(watcher) = self.watcher_proxy().await else {
            return;
        };
        let Ok(services) = watcher
            .get_property::<Vec<String>>("RegisteredStatusNotifierItems")
            .await
        else {
            return;
        };

        for service in services {
            match Self::fetch_item(&self.conn, &service).await {
                Ok(item) => {
                    self.items.insert(service, item);
                }
                Err(e) => eprintln!("Failed to fetch {service}: {e}"),
            }
        }
    }

    async fn fetch_item(conn: &Connection, service: &str) -> anyhow::Result<TrayItem> {
        let (dest, path) = if let Some((d, p)) = service.split_once('/') {
            (d.to_string(), format!("/{p}"))
        } else {
            (service.to_string(), "/StatusNotifierItem".to_string())
        };

        let proxy = Proxy::new(conn, dest, path, "org.kde.StatusNotifierItem").await?;

        let icon_name: String = proxy.get_property("IconName").await.unwrap_or_default();
        let status: String = proxy.get_property("Status").await.unwrap_or_default();

        let tooltip: String = proxy
            .get_property::<Structure>("ToolTip")
            .await
            .ok()
            .and_then(|s| {
                s.fields()
                    .get(2)
                    .and_then(|v| String::try_from(v.clone()).ok())
            })
            .unwrap_or_default();

        let icon_pixmaps = proxy
            .get_property::<Vec<(i32, i32, Vec<u8>)>>("IconPixmap")
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(width, height, data)| IconPixmap {
                width,
                height,
                data,
            })
            .collect();

        let menu_path: Option<String> = proxy
            .get_property::<zbus::zvariant::OwnedObjectPath>("Menu")
            .await
            .ok()
            .map(|p| p.to_string())
            .filter(|p| p != "/" && !p.is_empty());

        Ok(TrayItem {
            service: service.to_string(),
            icon_name,
            icon_pixmaps,
            status,
            tooltip,
            menu_path,
        })
    }

    // Run the event loop, keeping the item list in sync
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let watcher = self.watcher_proxy().await?;
        let mut added = watcher
            .receive_signal("StatusNotifierItemRegistered")
            .await?;
        let mut removed = watcher
            .receive_signal("StatusNotifierItemUnregistered")
            .await?;

        // Also watch for items that vanish without unregistering
        let dbus_proxy = Proxy::new(
            &self.conn,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await?;
        let mut name_changes = dbus_proxy.receive_signal("NameOwnerChanged").await?;

        loop {
            tokio::select! {
                Some(msg) = added.next() => {
                    if let Ok((service,)) = msg.body().deserialize::<(String,)>() {
                        match Self::fetch_item(&self.conn, &service).await {
                            Ok(item) => { self.items.insert(service, item); }
                            Err(e) => eprintln!("Failed to fetch {service}: {e}"),
                        }
                    }
                }
                Some(msg) = removed.next() => {
                    if let Ok((service,)) = msg.body().deserialize::<(String,)>() {
                        self.items.remove(&service);
                    }
                }
                Some(msg) = name_changes.next() => {
                    // NameOwnerChanged fires as (name, old_owner, new_owner)
                    // An empty new_owner means the name has vanished
                    if let Ok((name, _old, new_owner)) = msg.body().deserialize::<(String, String, String)>() {
                        if new_owner.is_empty() {
                            self.items.remove(&name);
                        }
                    }
                }
            }
        }
    }

    pub fn items(&self) -> &HashMap<String, TrayItem> {
        &self.items
    }
}
