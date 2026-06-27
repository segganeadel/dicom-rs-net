//! Tracks active DICOM associations (dcm4che-style open association list).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::{broadcast, watch};

/// Lifecycle state of a tracked association.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum AssociationState {
    /// A-ASSOCIATE negotiation in progress.
    Negotiating,
    /// Association established and DIMSE active.
    Active,
    /// Waiting for in-flight operations before close.
    Draining,
}

/// Filter for registry queries and drain/force targeting.
#[derive(Debug, Clone)]
pub enum AssociationFilter {
    /// All open associations.
    All,
    /// Associations on a connection index.
    ConnectionIndex(usize),
    /// Associations on a connection id.
    ConnectionId(String),
    /// Associations for a config AE id.
    AeId(String),
    /// Associations on a specific AE + connection binding.
    Binding {
        /// Connection id from config.
        connection_id: String,
        /// AE id from config.
        ae_id: String,
    },
}

/// A single active tracked association (runtime-only, not in config).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssociationRecord {
    /// Monotonic registry id (dcm4che serialNo).
    pub id: u64,
    /// `true` when this AE initiated the association (SCU).
    pub requestor: bool,
    /// Config AE id for the local application entity.
    pub ae_id: String,
    /// Local AE title (called AE for SCP, calling AE for SCU).
    pub ae_title: String,
    /// Remote peer AE title.
    pub remote_ae_title: String,
    /// Stable connection id from configuration.
    pub connection_id: String,
    /// Index into the device connection list.
    pub connection_index: usize,
    /// Called AE title from A-ASSOCIATE.
    pub called_ae: String,
    /// Calling AE title from A-ASSOCIATE.
    pub calling_ae: String,
    /// Remote peer socket address.
    pub peer: String,
    /// Association lifecycle state.
    pub state: AssociationState,
    /// In-flight DIMSE operations.
    pub performing: u32,
    /// Current DIMSE operation name, if any.
    pub current_dimse: Option<String>,
    /// Unix timestamp when the association was registered.
    pub started_at: u64,
    /// Unix timestamp of last DIMSE activity.
    pub last_activity_at: u64,
}

#[derive(Debug)]
struct AssociationEntry {
    record: AssociationRecord,
    cancel_tx: watch::Sender<bool>,
}

/// Handle returned when registering an association; unregisters on drop.
pub struct AssociationGuard {
    id: u64,
    registry: SharedAssociationRegistry,
}

impl AssociationGuard {
    /// Registry id for this association.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Receiver fired when the association should abort (force shutdown).
    pub fn cancel_rx(&self) -> watch::Receiver<bool> {
        self.registry.cancel_rx(self.id)
    }

    /// Removes this association from the registry without waiting for drop.
    pub fn unregister(self) {
        let id = self.id;
        let registry = Arc::clone(&self.registry);
        std::mem::forget(self);
        registry.unregister(id);
    }
}

impl Drop for AssociationGuard {
    fn drop(&mut self) {
        self.registry.unregister(self.id);
    }
}

/// Thread-safe registry of active associations.
#[derive(Debug)]
pub struct AssociationRegistry {
    next_id: AtomicU64,
    records: Mutex<HashMap<u64, AssociationEntry>>,
    change_tx: Mutex<Option<broadcast::Sender<()>>>,
}

impl Default for AssociationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AssociationRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            records: Mutex::new(HashMap::new()),
            change_tx: Mutex::new(None),
        }
    }

    /// Notifies listeners when association state changes (register, DIMSE, unregister).
    pub fn set_change_notifier(&self, tx: broadcast::Sender<()>) {
        *self.change_tx.lock().unwrap() = Some(tx);
    }

    fn notify_change(&self) {
        if let Some(tx) = self.change_tx.lock().unwrap().as_ref() {
            let _ = tx.send(());
        }
    }

    /// Registers an inbound SCP association at A-ASSOCIATE-RQ.
    pub fn register_inbound(
        registry: &SharedAssociationRegistry,
        connection_id: impl Into<String>,
        connection_index: usize,
        called_ae: impl Into<String>,
        calling_ae: impl Into<String>,
        peer: impl Into<String>,
    ) -> AssociationGuard {
        let called_ae = called_ae.into();
        let calling_ae = calling_ae.into();
        Self::insert(
            registry,
            false,
            String::new(),
            called_ae.clone(),
            calling_ae.clone(),
            called_ae,
            calling_ae,
            connection_id,
            connection_index,
            peer,
        )
    }

    /// Registers an outbound SCU association after negotiation.
    pub fn register_outbound(
        registry: &SharedAssociationRegistry,
        ae_id: impl Into<String>,
        local_ae: impl Into<String>,
        remote_ae: impl Into<String>,
        connection_id: impl Into<String>,
        connection_index: usize,
        peer: impl Into<String>,
    ) -> AssociationGuard {
        let local_ae = local_ae.into();
        let remote_ae = remote_ae.into();
        Self::insert(
            registry,
            true,
            ae_id.into(),
            local_ae.clone(),
            remote_ae.clone(),
            remote_ae.clone(),
            local_ae.clone(),
            connection_id,
            connection_index,
            peer,
        )
    }

    fn insert(
        registry: &SharedAssociationRegistry,
        requestor: bool,
        ae_id: String,
        ae_title: String,
        remote_ae_title: String,
        called_ae: String,
        calling_ae: String,
        connection_id: impl Into<String>,
        connection_index: usize,
        peer: impl Into<String>,
    ) -> AssociationGuard {
        let id = registry.next_id.fetch_add(1, Ordering::Relaxed);
        let now = unix_now_secs();
        let (cancel_tx, _) = watch::channel(false);
        let record = AssociationRecord {
            id,
            requestor,
            ae_id,
            ae_title,
            remote_ae_title,
            connection_id: connection_id.into(),
            connection_index,
            called_ae,
            calling_ae,
            peer: peer.into(),
            state: AssociationState::Negotiating,
            performing: 0,
            current_dimse: None,
            started_at: now,
            last_activity_at: now,
        };
        registry.records.lock().unwrap().insert(
            id,
            AssociationEntry {
                record,
                cancel_tx,
            },
        );
        registry.notify_change();
        AssociationGuard {
            id,
            registry: Arc::clone(registry),
        }
    }

    /// Marks an association as active after negotiation.
    pub fn set_active(&self, id: u64) {
        if let Some(entry) = self.records.lock().unwrap().get_mut(&id) {
            entry.record.state = AssociationState::Active;
            entry.record.last_activity_at = unix_now_secs();
        }
        self.notify_change();
    }

    /// Sets the resolved config AE id after inbound AE lookup.
    pub fn set_ae(&self, id: u64, ae_id: impl Into<String>, ae_title: impl Into<String>) {
        if let Some(entry) = self.records.lock().unwrap().get_mut(&id) {
            entry.record.ae_id = ae_id.into();
            entry.record.ae_title = ae_title.into();
        }
        self.notify_change();
    }

    /// Marks the start of a DIMSE operation.
    pub fn begin_dimse(&self, id: u64, dimse: impl Into<String>) {
        if let Some(entry) = self.records.lock().unwrap().get_mut(&id) {
            entry.record.performing = entry.record.performing.saturating_add(1);
            entry.record.current_dimse = Some(dimse.into());
            entry.record.last_activity_at = unix_now_secs();
        }
        self.notify_change();
    }

    /// Marks the end of a DIMSE operation.
    pub fn end_dimse(&self, id: u64) {
        if let Some(entry) = self.records.lock().unwrap().get_mut(&id) {
            entry.record.performing = entry.record.performing.saturating_sub(1);
            if entry.record.performing == 0 {
                entry.record.current_dimse = None;
            }
            entry.record.last_activity_at = unix_now_secs();
        }
        self.notify_change();
    }

    /// Returns whether an association is draining and has no in-flight DIMSE.
    pub fn should_release(&self, id: u64) -> bool {
        let records = self.records.lock().unwrap();
        records.get(&id).is_some_and(|e| {
            e.record.state == AssociationState::Draining && e.record.performing == 0
        })
    }

    /// Marks associations on a connection as draining.
    pub fn mark_connection_draining(&self, connection_index: usize) {
        let mut records = self.records.lock().unwrap();
        for entry in records.values_mut() {
            if entry.record.connection_index == connection_index {
                entry.record.state = AssociationState::Draining;
            }
        }
        drop(records);
        self.notify_change();
    }

    /// Marks associations for a config AE as draining.
    pub fn mark_ae_draining(&self, ae_id: &str) {
        let mut records = self.records.lock().unwrap();
        for entry in records.values_mut() {
            if entry.record.ae_id == ae_id {
                entry.record.state = AssociationState::Draining;
            }
        }
        drop(records);
        self.notify_change();
    }

    /// Marks associations on a binding as draining.
    pub fn mark_binding_draining(&self, connection_id: &str, ae_id: &str) {
        let mut records = self.records.lock().unwrap();
        for entry in records.values_mut() {
            if entry.record.connection_id == connection_id && entry.record.ae_id == ae_id {
                entry.record.state = AssociationState::Draining;
            }
        }
        drop(records);
        self.notify_change();
    }

    /// Signals matching associations to abort.
    pub fn force_abort_matching(&self, filter: &AssociationFilter) {
        let records = self.records.lock().unwrap();
        for entry in records.values() {
            if matches_filter(&entry.record, filter) {
                let _ = entry.cancel_tx.send(true);
            }
        }
    }

    /// Removes an association from the registry.
    pub fn unregister(&self, id: u64) {
        let removed = self.records.lock().unwrap().remove(&id).is_some();
        if removed {
            self.notify_change();
        }
    }

    fn cancel_rx(&self, id: u64) -> watch::Receiver<bool> {
        self.records
            .lock()
            .unwrap()
            .get(&id)
            .map(|e| e.cancel_tx.subscribe())
            .unwrap_or_else(|| {
                let (_, rx) = watch::channel(false);
                rx
            })
    }

    /// Returns records matching an optional filter.
    pub fn list_filtered(&self, filter: Option<&AssociationFilter>) -> Vec<AssociationRecord> {
        self.records
            .lock()
            .unwrap()
            .values()
            .filter(|e| filter.map(|f| matches_filter(&e.record, f)).unwrap_or(true))
            .map(|e| e.record.clone())
            .collect()
    }

    /// Returns all active records.
    pub fn list(&self) -> Vec<AssociationRecord> {
        self.list_filtered(None)
    }

    /// Count of open associations.
    pub fn open_count(&self) -> usize {
        self.records.lock().unwrap().len()
    }

    /// Count of associations on a connection.
    pub fn count_on_connection(&self, connection_index: usize) -> usize {
        self.count_matching(&AssociationFilter::ConnectionIndex(connection_index))
    }

    fn count_matching(&self, filter: &AssociationFilter) -> usize {
        self.records
            .lock()
            .unwrap()
            .values()
            .filter(|e| matches_filter(&e.record, filter))
            .count()
    }

    /// Waits until no associations match the filter or timeout elapses.
    pub async fn wait_for_idle(
        &self,
        filter: &AssociationFilter,
        timeout: std::time::Duration,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if self.count_matching(filter) == 0 {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Waits until no associations remain on a connection or timeout elapses.
    pub async fn wait_for_connection_idle(
        &self,
        connection_index: usize,
        timeout: std::time::Duration,
    ) -> bool {
        self.wait_for_idle(
            &AssociationFilter::ConnectionIndex(connection_index),
            timeout,
        )
        .await
    }
}

/// Shared registry handle used by device listeners.
pub type SharedAssociationRegistry = Arc<AssociationRegistry>;

fn matches_filter(record: &AssociationRecord, filter: &AssociationFilter) -> bool {
    match filter {
        AssociationFilter::All => true,
        AssociationFilter::ConnectionIndex(idx) => record.connection_index == *idx,
        AssociationFilter::ConnectionId(id) => record.connection_id == *id,
        AssociationFilter::AeId(ae_id) => record.ae_id == *ae_id,
        AssociationFilter::Binding {
            connection_id,
            ae_id,
        } => record.connection_id == *connection_id && record.ae_id == *ae_id,
    }
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_unregister() {
        let reg = Arc::new(AssociationRegistry::new());
        let guard = AssociationRegistry::register_inbound(
            &reg,
            "conn",
            0,
            "PACS",
            "STORESCU",
            "127.0.0.1:1",
        );
        assert_eq!(reg.count_on_connection(0), 1);
        let id = guard.id();
        drop(guard);
        assert_eq!(reg.count_on_connection(0), 0);
        reg.unregister(id);
    }

    #[test]
    fn dimse_performing_count() {
        let reg = Arc::new(AssociationRegistry::new());
        let guard = AssociationRegistry::register_inbound(
            &reg,
            "conn",
            0,
            "PACS",
            "STORESCU",
            "127.0.0.1:1",
        );
        let id = guard.id();
        reg.begin_dimse(id, "C-STORE");
        assert_eq!(reg.list()[0].performing, 1);
        reg.end_dimse(id);
        assert_eq!(reg.list()[0].performing, 0);
        drop(guard);
    }
}
