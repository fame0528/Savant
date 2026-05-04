use arc_swap::ArcSwap;
use savant_core::traits::Tool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

#[derive(Clone, Debug)]
pub enum ToolRegistryEvent {
    ToolAdded { name: String },
    ToolRemoved { name: String },
    ToolUpdated { name: String },
}

#[derive(Clone)]
pub struct RegistryEpoch {
    pub tools: HashMap<String, Arc<dyn Tool>>,
    pub epoch_id: u64,
}

impl Default for RegistryEpoch {
    fn default() -> Self {
        RegistryEpoch {
            tools: HashMap::new(),
            epoch_id: 0,
        }
    }
}

pub struct SharedToolRegistry {
    current: ArcSwap<RegistryEpoch>,
    event_tx: broadcast::Sender<ToolRegistryEvent>,
}

impl SharedToolRegistry {
    pub fn new() -> Arc<Self> {
        let (event_tx, _) = broadcast::channel(128);
        Arc::new(SharedToolRegistry {
            current: ArcSwap::from_pointee(RegistryEpoch::default()),
            event_tx,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ToolRegistryEvent> {
        self.event_tx.subscribe()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let guard = self.current.load();
        guard.tools.get(name).cloned()
    }

    pub fn contains(&self, name: &str) -> bool {
        let guard = self.current.load();
        guard.tools.contains_key(name)
    }

    pub fn list_all(&self) -> HashMap<String, Arc<dyn Tool>> {
        let guard = self.current.load();
        guard.tools.clone()
    }

    pub fn register(&self, name: String, tool: Arc<dyn Tool>) {
        let guard = self.current.load();
        let mut new_epoch = (**guard).clone();
        new_epoch.tools.insert(name.clone(), tool);
        new_epoch.epoch_id = guard.epoch_id.wrapping_add(1);
        self.current.store(Arc::from(new_epoch));

        info!("[toolforge] Tool registered: {name}");
        let _ = self.event_tx.send(ToolRegistryEvent::ToolAdded { name });
    }

    pub fn remove(&self, name: &str) -> bool {
        let guard = self.current.load();
        let mut new_epoch = (**guard).clone();
        let existed = new_epoch.tools.remove(name).is_some();
        if existed {
            new_epoch.epoch_id = guard.epoch_id.wrapping_add(1);
            self.current.store(Arc::from(new_epoch));
            info!("[toolforge] Tool removed: {name}");
            let _ = self.event_tx.send(ToolRegistryEvent::ToolRemoved {
                name: name.to_string(),
            });
        }
        existed
    }

    pub fn epoch_id(&self) -> u64 {
        self.current.load().epoch_id
    }
}
