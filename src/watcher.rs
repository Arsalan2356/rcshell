use futures::StreamExt;
use std::collections::HashSet;
use zbus::object_server::SignalEmitter;
use zbus::{Connection, connection, fdo, interface};

struct WatcherState {
    items: HashSet<String>,
    hosts: HashSet<String>,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl WatcherState {
    async fn register_status_notifier_item(
        &mut self,
        service: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(signal_context)] ctx: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        // If service is a path, prepend the sender's unique bus name
        let owned = if service.starts_with('/') {
            let sender = header
                .sender()
                .ok_or(fdo::Error::Failed("no sender".into()))?
                .to_string();
            format!("{sender}{service}") // e.g. ":1.234/org/ayatana/NotificationItem/nm_applet"
        } else {
            service.to_string()
        };

        if self.items.insert(owned.clone()) {
            Self::status_notifier_item_registered(&ctx, &owned).await?;
        }
        Ok(())
    }

    async fn register_status_notifier_host(
        &mut self,
        service: &str,
        #[zbus(signal_context)] ctx: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        if self.hosts.insert(service.to_string()) {
            Self::status_notifier_host_registered(&ctx).await?;
        }
        Ok(())
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.iter().cloned().collect()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        !self.hosts.is_empty()
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        ctx: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        ctx: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(ctx: &SignalEmitter<'_>) -> zbus::Result<()>;
}

pub struct StatusNotifierWatcher {
    conn: Connection,
}

impl StatusNotifierWatcher {
    pub async fn spawn() -> anyhow::Result<Self> {
        let state = WatcherState {
            items: HashSet::new(),
            hosts: HashSet::new(),
        };

        let conn = connection::Builder::session()?
            .name("org.kde.StatusNotifierWatcher")?
            .serve_at("/StatusNotifierWatcher", state)?
            .build()
            .await?;

        Ok(Self { conn: conn })
    }

    pub async fn watch_names(&self) -> Result<(), anyhow::Error> {
        let conn = &self.conn;
        let dbus = zbus::Proxy::new(
            conn,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await?;
        let mut owner_changed = dbus.receive_signal("NameOwnerChanged").await?;
        let object_server = conn.object_server();
        while let Some(msg) = owner_changed.next().await {
            let Ok((name, _old_owner, new_owner)) =
                msg.body().deserialize::<(String, String, String)>()
            else {
                continue;
            };

            // Only care about unique names disappearing.
            if !name.starts_with(':') || !new_owner.is_empty() {
                continue;
            }

            let Ok(iface) = object_server
                .interface::<_, WatcherState>("/StatusNotifierWatcher")
                .await
            else {
                continue;
            };

            let mut watcher = iface.get_mut().await;

            let removed: Vec<String> = watcher
                .items
                .iter()
                .filter(|item| {
                    item.split_once('/')
                        .map(|(dest, _)| dest == name)
                        .unwrap_or(*item == &name)
                })
                .cloned()
                .collect();

            for item in removed {
                watcher.items.remove(&item);

                let _ =
                    WatcherState::status_notifier_item_unregistered(iface.signal_emitter(), &item)
                        .await;
            }
        }

        Ok(())
    }
}
