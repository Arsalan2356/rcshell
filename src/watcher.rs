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
        #[zbus(signal_context)] ctx: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        let owned = service.to_string();
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

        Ok(Self { conn })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}
