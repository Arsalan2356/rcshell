use std::collections::HashMap;
use std::time::Duration;
use zbus::{Connection, Proxy};

#[derive(Debug, Clone, PartialEq)]
pub struct IconPixmap {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrayItem {
    pub service: String,
    pub icon_name: String,
    pub icon_pixmaps: Vec<IconPixmap>,
    pub tooltip: String,
    pub _menu_path: Option<String>,
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

    async fn get_service_pid(conn: &Connection, service: &str) -> zbus::Result<u32> {
        let (dest, _) = service
            .split_once('/')
            .map(|(d, p)| (d, p))
            .unwrap_or((service, ""));

        let proxy = Proxy::new(
            conn,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await?;

        proxy.call("GetConnectionUnixProcessID", &(dest,)).await
    }

    async fn add_item(&mut self, service: String) {
        match Self::get_service_pid(&self.conn, &service).await {
            Ok(_) => match Self::fetch_item(&self.conn, &service).await {
                Ok(item) => {
                    self.items.insert(service, item);
                }
                Err(e) => {
                    eprintln!("Failed to fetch {service}: {e}");
                }
            },

            Err(e) => {
                eprintln!("Ignoring stale service {service}: {e}");
            }
        }
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
            self.add_item(service).await;
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

        let tooltip: String = proxy.get_property("Title").await.unwrap_or_default();

        let icon_pixmaps: Vec<IconPixmap> = proxy
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
            tooltip,
            _menu_path: menu_path,
        })
    }

    // Run the event loop, keeping the item list in sync
    pub async fn run(
        &mut self,
        tx: tokio::sync::mpsc::Sender<Vec<TrayItem>>,
    ) -> anyhow::Result<()> {
        loop {
            let old_items: HashMap<String, TrayItem> = self.items.drain().collect();
            self.fetch_all_items().await;

            let items = self.items.values().cloned().collect();
            if old_items != self.items {
                let _ = tx.send(items).await;
            }

            glib::timeout_future(Duration::from_millis(300)).await;
        }
    }

    pub fn items(&self) -> &HashMap<String, TrayItem> {
        &self.items
    }
}
