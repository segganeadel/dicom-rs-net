//! DICOM device container with multi-AE connection binding.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use bytes::BytesMut;
use dicom_ul::association::read_pdu_from_wire_async;
use dicom_ul::pdu::{
    AssociationRJ, AssociationRJResult, AssociationRJServiceUserReason, AssociationRJSource,
    AssociationRQ, Pdu, write_pdu,
};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Semaphore};
use tracing::{error, info};

use crate::association::{handle_association, AssociationTracker};
use crate::device::application_entity::{ApplicationEntity, normalize_ae_title};
use crate::device::{AssociationFilter, AssociationRegistry, SharedAssociationRegistry};
use crate::device::connection::Connection;
use crate::error::{Error, Result};

#[derive(Debug)]
struct ConnectionRuntime {
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Default)]
struct DeviceRuntimeState {
    association_limit: Option<Arc<Semaphore>>,
    connections: HashMap<usize, ConnectionRuntime>,
}

/// Root container for connections and application entities (dcm4che-style device model).
pub struct Device {
    /// Optional human-readable device name.
    pub device_name: Option<String>,
    /// Network connections owned by this device.
    pub connections: Vec<Connection>,
    /// Application entities keyed by AE title.
    pub application_entities: RwLock<HashMap<String, ApplicationEntity>>,
    /// Maximum concurrent inbound associations (backpressure).
    pub max_concurrent_associations: Option<usize>,
    /// Active association registry for admin and hot-reload.
    pub association_registry: SharedAssociationRegistry,
    runtime: Mutex<DeviceRuntimeState>,
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Device")
            .field("device_name", &self.device_name)
            .field("connections", &self.connections.len())
            .field("application_entities", &self.application_entities.read().unwrap().len())
            .field(
                "max_concurrent_associations",
                &self.max_concurrent_associations,
            )
            .finish_non_exhaustive()
    }
}

impl Device {
    /// Creates an empty device.
    pub fn new() -> Self {
        Self {
            device_name: None,
            connections: Vec::new(),
            application_entities: RwLock::new(HashMap::new()),
            max_concurrent_associations: None,
            association_registry: Arc::new(crate::device::AssociationRegistry::new()),
            runtime: Mutex::new(DeviceRuntimeState::default()),
        }
    }

    /// Returns the shared association registry.
    pub fn registry(&self) -> SharedAssociationRegistry {
        Arc::clone(&self.association_registry)
    }

    /// Limits how many associations may be handled concurrently across all connections.
    pub fn max_concurrent_associations(mut self, limit: usize) -> Self {
        self.max_concurrent_associations = Some(limit);
        self
    }

    /// Sets the device name.
    pub fn device_name(mut self, name: impl Into<String>) -> Self {
        self.device_name = Some(name.into());
        self
    }

    /// Adds a connection and returns its index.
    pub fn add_connection(&mut self, connection: Connection) -> usize {
        let index = self.connections.len();
        self.connections.push(connection);
        index
    }

    /// Registers an application entity.
    pub fn add_application_entity(&self, ae: ApplicationEntity) {
        let title = normalize_ae_title(&ae.ae_title);
        self.application_entities
            .write()
            .unwrap()
            .insert(title, ae);
    }

    /// Replaces an application entity for new negotiations.
    pub fn update_application_entity(&self, ae: ApplicationEntity) {
        self.add_application_entity(ae);
    }

    /// Looks up an application entity by called AE title.
    pub fn find_ae(&self, called_aet: &str) -> Option<ApplicationEntity> {
        let key = normalize_ae_title(called_aet);
        self.application_entities.read().unwrap().get(&key).cloned()
    }

    /// Returns application entities linked to a connection index.
    pub fn aes_on_connection(&self, conn_index: usize) -> Vec<ApplicationEntity> {
        self.application_entities
            .read()
            .unwrap()
            .values()
            .filter(|ae| ae.connection_indices.contains(&conn_index))
            .cloned()
            .collect()
    }

    /// Returns the first registered application entity, if any.
    pub fn default_ae(&self) -> Option<ApplicationEntity> {
        self.application_entities
            .read()
            .unwrap()
            .values()
            .next()
            .cloned()
    }

    /// Whether a connection listener is currently bound.
    pub async fn is_connection_bound(&self, conn_index: usize) -> bool {
        self.runtime
            .lock()
            .await
            .connections
            .contains_key(&conn_index)
    }

    /// Binds all connections that are not already listening.
    pub async fn bind_connections(self: Arc<Self>) -> Result<()> {
        for index in 0..self.connections.len() {
            if !self.is_connection_bound(index).await {
                self.clone().bind_connection(index).await?;
            }
        }
        Ok(())
    }

    /// Binds a single connection listener.
    pub async fn bind_connection(self: Arc<Self>, conn_index: usize) -> Result<()> {
        let conn = self.connections.get(conn_index).ok_or_else(|| Error::InvalidCommand {
            message: format!("unknown connection index {conn_index}"),
        })?;

        let mut runtime = self.runtime.lock().await;
        if runtime.connections.contains_key(&conn_index) {
            return Err(Error::InvalidCommand {
                message: format!("connection {conn_index} is already bound"),
            });
        }

        if runtime.association_limit.is_none() {
            runtime.association_limit = self
                .max_concurrent_associations
                .map(|n| Arc::new(Semaphore::new(n)));
        }

        let addr = conn.socket_addr()?;
        let listener = bind_listener(conn)?;
        info!("device listening on tcp://{addr}");

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let device = Arc::clone(&self);
        let conn = conn.clone();
        let association_limit = runtime.association_limit.clone();

        let task = tokio::spawn(async move {
            accept_loop(
                device,
                conn_index,
                conn,
                listener,
                addr,
                shutdown_rx,
                association_limit,
            )
            .await;
        });

        runtime
            .connections
            .insert(conn_index, ConnectionRuntime { shutdown_tx, task });
        Ok(())
    }

    /// Stops accepting new associations on a single connection.
    pub async fn unbind_connection(&self, conn_index: usize) -> Result<()> {
        let mut runtime = self.runtime.lock().await;
        if let Some(conn_rt) = runtime.connections.remove(&conn_index) {
            let _ = conn_rt.shutdown_tx.send(true);
            conn_rt.task.abort();
        }
        Ok(())
    }

    /// Returns open associations (dcm4che `listOpenAssociations`).
    pub fn list_open_associations(&self) -> Vec<crate::device::AssociationRecord> {
        self.association_registry.list()
    }

    /// Waits until all open associations close or timeout elapses.
    pub async fn wait_for_no_open_associations(&self, timeout: Duration) -> bool {
        self.association_registry
            .wait_for_idle(&AssociationFilter::All, timeout)
            .await
    }

    /// Stops accepting on a connection and waits for active associations to finish.
    pub async fn drain_connection(
        &self,
        conn_index: usize,
        timeout: Duration,
    ) -> Result<bool> {
        self.association_registry
            .mark_connection_draining(conn_index);
        self.unbind_connection(conn_index).await?;
        Ok(self
            .association_registry
            .wait_for_connection_idle(conn_index, timeout)
            .await)
    }

    /// Drains associations on a specific AE + connection binding.
    pub async fn drain_binding(
        &self,
        connection_id: &str,
        ae_id: &str,
        conn_index: usize,
        timeout: Duration,
    ) -> Result<bool> {
        self.association_registry
            .mark_binding_draining(connection_id, ae_id);
        self.unbind_connection(conn_index).await?;
        Ok(self
            .association_registry
            .wait_for_idle(
                &AssociationFilter::Binding {
                    connection_id: connection_id.to_string(),
                    ae_id: ae_id.to_string(),
                },
                timeout,
            )
            .await)
    }

    /// Stops accepting on a connection and force-aborts active associations.
    pub async fn force_unbind_connection(&self, conn_index: usize) -> Result<()> {
        self.association_registry
            .mark_connection_draining(conn_index);
        self.association_registry
            .force_abort_matching(&AssociationFilter::ConnectionIndex(conn_index));
        self.unbind_connection(conn_index).await
    }

    /// Stops accepting new associations on all connections.
    pub async fn unbind_connections(&self) -> Result<()> {
        let indices: Vec<usize> = self.runtime.lock().await.connections.keys().copied().collect();
        for index in indices {
            self.unbind_connection(index).await?;
        }
        Ok(())
    }
}

impl Default for Device {
    fn default() -> Self {
        Self::new()
    }
}

async fn accept_loop(
    device: Arc<Device>,
    conn_index: usize,
    conn: Connection,
    listener: TcpListener,
    addr: std::net::SocketAddr,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    association_limit: Option<Arc<Semaphore>>,
) {
    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_ok() && *shutdown_rx.borrow() {
                    break;
                }
            }
            accept = listener.accept() => {
                match accept {
                    Ok((socket, peer)) => {
                        let permit = match &association_limit {
                            Some(sem) => match sem.clone().acquire_owned().await {
                                Ok(p) => Some(p),
                                Err(_) => continue,
                            },
                            None => None,
                        };
                        let device = Arc::clone(&device);
                        let conn = conn.clone();
                        tokio::spawn(async move {
                            let _permit = permit;
                            if let Err(e) = handle_incoming(
                                device,
                                conn_index,
                                &conn,
                                socket,
                                peer,
                            )
                            .await
                            {
                                error!("association with {peer} failed: {e}");
                            }
                        });
                    }
                    Err(source) => {
                        error!("accept failed on tcp://{addr}: {source}");
                        break;
                    }
                }
            }
        }
    }
}

async fn handle_incoming(
    device: Arc<Device>,
    conn_index: usize,
    conn: &Connection,
    socket: tokio::net::TcpStream,
    peer: std::net::SocketAddr,
) -> Result<()> {
    #[cfg(feature = "tls")]
    if conn.tls_server_config.is_some() {
        return handle_incoming_tls(device, conn_index, conn, socket, peer).await;
    }

    handle_incoming_plain(device, conn_index, conn, socket, peer, None).await
}

#[cfg(feature = "tls")]
async fn handle_incoming_tls(
    device: Arc<Device>,
    conn_index: usize,
    conn: &Connection,
    socket: tokio::net::TcpStream,
    peer: std::net::SocketAddr,
) -> Result<()> {
    use tokio_rustls::TlsAcceptor;

    let tls_config = conn
        .tls_server_config
        .as_ref()
        .ok_or_else(|| Error::InvalidCommand {
            message: "TLS server config missing".to_string(),
        })?;
    let acceptor = TlsAcceptor::from(tls_config.clone());
    let tls_stream = acceptor.accept(socket).await.map_err(|source| Error::Io {
        source: std::io::Error::other(source.to_string()),
    })?;
    handle_incoming_plain(device, conn_index, conn, tls_stream, peer, None).await
}

async fn handle_incoming_plain<S>(
    device: Arc<Device>,
    conn_index: usize,
    conn: &Connection,
    mut socket: S,
    peer: std::net::SocketAddr,
    read_buffer: Option<BytesMut>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let mut read_buffer = read_buffer.unwrap_or_else(|| {
        BytesMut::with_capacity((conn.max_pdu_length as usize).min(64 * 1024) + 6)
    });

    let pdu = read_pdu_from_wire_async(
        &mut socket,
        &mut read_buffer,
        conn.max_pdu_length,
        conn.strict,
    )
    .await
    .map_err(|source| Error::Ul { source })?;

    route_incoming_association(device, conn_index, conn, socket, peer, pdu, read_buffer).await
}

async fn route_incoming_association<S>(
    device: Arc<Device>,
    conn_index: usize,
    conn: &Connection,
    mut socket: S,
    peer: std::net::SocketAddr,
    pdu: Pdu,
    read_buffer: BytesMut,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (calling_ae, called_ae) = match &pdu {
        Pdu::AssociationRQ(AssociationRQ {
            calling_ae_title,
            called_ae_title,
            ..
        }) => (
            normalize_ae_title(calling_ae_title),
            normalize_ae_title(called_ae_title),
        ),
        _ => {
            reject_association(&mut socket, AssociationRJServiceUserReason::NoReasonGiven).await?;
            return Err(Error::InvalidCommand {
                message: format!("expected AssociationRQ from {peer}"),
            });
        }
    };

    let guard = AssociationRegistry::register_inbound(
        &device.association_registry,
        &conn.connection_id,
        conn_index,
        &called_ae,
        &calling_ae,
        peer.to_string(),
    );
    let reg_id = guard.id();

    let result = async {
        let ae = device
            .find_ae(&called_ae)
            .ok_or_else(|| Error::InvalidCommand {
                message: format!("unknown called AE title: {called_ae}"),
            })?;

        device.association_registry.set_ae(reg_id, &ae.ae_id, &ae.ae_title);

        if !ae.acceptor {
            reject_association(
                &mut socket,
                AssociationRJServiceUserReason::CalledAETitleNotRecognized,
            )
            .await?;
            return Err(Error::InvalidCommand {
                message: format!("AE {called_ae} is not an acceptor"),
            });
        }

        if !ae.connection_indices.contains(&conn_index) {
            reject_association(
                &mut socket,
                AssociationRJServiceUserReason::CalledAETitleNotRecognized,
            )
            .await?;
            return Err(Error::InvalidCommand {
                message: format!("AE {called_ae} is not registered on this connection"),
            });
        }

        if !ae.accepts_calling_ae(&calling_ae) {
            reject_association(
                &mut socket,
                AssociationRJServiceUserReason::CallingAETitleNotRecognized,
            )
            .await?;
            return Err(Error::InvalidCommand {
                message: format!("calling AE {calling_ae} not accepted by {called_ae}"),
            });
        }

        let options = ae.build_server_options(conn);
        let services = Arc::new(ae.services.clone());

        match options
            .establish_async_with_rq(socket, pdu, read_buffer)
            .await
        {
            Ok(association) => {
                device.association_registry.set_active(reg_id);
                tracing::info!(target: "dicom_net.metrics", event = "association_accepted", peer = %peer);
                let tracker = AssociationTracker::new(
                    reg_id,
                    device.registry(),
                    &ae.ae_id,
                    &conn.connection_id,
                    conn_index,
                    guard.cancel_rx(),
                );
                handle_association(association, services, tracker).await
            }
            Err(source) => Err(Error::Ul { source }),
        }
    }
    .await;

    drop(guard);
    result
}

fn bind_listener(conn: &Connection) -> Result<TcpListener> {
    use socket2::{Domain, Socket, Type};

    let addr = conn.socket_addr()?;
    let domain = if addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let socket = Socket::new(domain, Type::STREAM, None).map_err(|source| Error::Io { source })?;
    socket
        .set_reuse_address(true)
        .map_err(|source| Error::Io { source })?;
    socket
        .bind(&addr.into())
        .map_err(|source| Error::Io { source })?;
    socket
        .listen(conn.backlog as i32)
        .map_err(|source| Error::Io { source })?;
    socket
        .set_nonblocking(true)
        .map_err(|source| Error::Io { source })?;
    TcpListener::from_std(socket.into()).map_err(|source| Error::Io { source })
}

async fn reject_association<S>(socket: &mut S, reason: AssociationRJServiceUserReason) -> Result<()>
where
    S: tokio::io::AsyncWrite + Unpin,
{
    let association_rj = AssociationRJ {
        result: AssociationRJResult::Permanent,
        source: AssociationRJSource::ServiceUser(reason),
    };
    let pdu = Pdu::AssociationRJ(association_rj);
    let mut buf = Vec::new();
    write_pdu(&mut buf, &pdu).map_err(|e| Error::InvalidCommand {
        message: format!("failed to encode association rejection: {e}"),
    })?;
    socket
        .write_all(&buf)
        .await
        .map_err(|source| Error::Io { source })?;
    Ok(())
}
