//! SOP class to service routing.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::scp::{CFindService, CGetService, CMoveService, CStoreService};
use crate::service::DicomService;

/// Routes DIMSE requests to registered SCP services.
#[derive(Default, Clone)]
pub struct ServiceRegistry {
    by_sop_class: HashMap<String, Arc<dyn DicomService>>,
    promiscuous: Option<Arc<dyn DicomService>>,
    cstore: Option<Arc<CStoreService>>,
    cfind: Option<Arc<CFindService>>,
    cmove: Option<Arc<CMoveService>>,
    cget: Option<Arc<CGetService>>,
}

impl fmt::Debug for ServiceRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceRegistry")
            .field("registered_sop_classes", &self.by_sop_class.len())
            .field("promiscuous", &self.promiscuous.is_some())
            .field("cstore", &self.cstore.is_some())
            .field("cfind", &self.cfind.is_some())
            .field("cmove", &self.cmove.is_some())
            .field("cget", &self.cget.is_some())
            .finish()
    }
}

impl ServiceRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a service for its declared SOP classes.
    pub fn register(&mut self, service: Arc<dyn DicomService>) {
        for sop_class in service.sop_classes() {
            if *sop_class == "*" {
                self.promiscuous = Some(Arc::clone(&service));
            } else {
                self.by_sop_class
                    .insert((*sop_class).to_string(), Arc::clone(&service));
            }
        }
    }

    /// Registers a C-STORE service and indexes it for streaming dispatch.
    pub fn register_cstore(&mut self, service: Arc<CStoreService>) {
        for sop_class in service.sop_classes() {
            if *sop_class == "*" {
                self.promiscuous = Some(service.clone());
            } else {
                self.by_sop_class
                    .insert((*sop_class).to_string(), service.clone());
            }
        }
        self.cstore = Some(service);
    }

    /// Registers a C-FIND service.
    pub fn register_cfind(&mut self, service: Arc<CFindService>) {
        for sop_class in service.sop_classes() {
            self.by_sop_class
                .insert((*sop_class).to_string(), service.clone());
        }
        self.cfind = Some(service);
    }

    /// Registers a C-MOVE service.
    pub fn register_cmove(&mut self, service: Arc<CMoveService>) {
        for sop_class in service.sop_classes() {
            self.by_sop_class
                .insert((*sop_class).to_string(), service.clone());
        }
        self.cmove = Some(service);
    }

    /// Registers a C-GET service.
    pub fn register_cget(&mut self, service: Arc<CGetService>) {
        for sop_class in service.sop_classes() {
            self.by_sop_class
                .insert((*sop_class).to_string(), service.clone());
        }
        self.cget = Some(service);
    }

    /// Returns the registered C-STORE service, if any.
    pub fn cstore(&self) -> Option<Arc<CStoreService>> {
        self.cstore.clone()
    }

    /// Returns the registered C-FIND service, if any.
    pub fn cfind(&self) -> Option<Arc<CFindService>> {
        self.cfind.clone()
    }

    /// Returns the registered C-MOVE service, if any.
    pub fn cmove(&self) -> Option<Arc<CMoveService>> {
        self.cmove.clone()
    }

    /// Returns the registered C-GET service, if any.
    pub fn cget(&self) -> Option<Arc<CGetService>> {
        self.cget.clone()
    }

    /// Resolves the service for a SOP class UID.
    pub fn get(&self, sop_class_uid: &str) -> Option<Arc<dyn DicomService>> {
        self.by_sop_class
            .get(sop_class_uid)
            .cloned()
            .or_else(|| self.promiscuous.clone())
    }

    /// Resolves a service or returns [`Error::UnknownSopClass`].
    pub fn resolve(&self, sop_class_uid: &str) -> Result<Arc<dyn DicomService>> {
        self.get(sop_class_uid)
            .ok_or_else(|| Error::UnknownSopClass {
                sop_class_uid: sop_class_uid.to_string(),
            })
    }
}
