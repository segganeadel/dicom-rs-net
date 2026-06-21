//! DIMSE request handling loop for an established association.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use dicom_ul::association::Association;
use dicom_ul::association::server::AsyncServerAssociation;
use dicom_ul::pdu::{PDataValue, PDataValueType, Pdu};
use tracing::{debug, info, instrument, warn};

use crate::association::AssociationContext;
use crate::association::DatasetReader;
use crate::association::retrieve::{run_cget_subops, run_cmove_subops};
use crate::dimse::{Dimse, DimseMessage, parse::parse_command, response};
use crate::error::{Error, Result};
use crate::service::ServiceRegistry;
use crate::status::Status;

#[allow(clippy::enum_variant_names)]
enum PendingOperation {
    CStore(DimseMessage),
    CFind(DimseMessage),
    CMove(DimseMessage),
    CGet(DimseMessage),
}

struct AssociationState {
    pending: Option<PendingOperation>,
    dataset_buffer: Vec<u8>,
    cancelled: Arc<AtomicBool>,
    active_message_id: Option<u16>,
}

impl AssociationState {
    fn new() -> Self {
        Self {
            pending: None,
            dataset_buffer: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
            active_message_id: None,
        }
    }
}

/// Handles DIMSE traffic on an established server association.
#[instrument(skip(association, services))]
pub async fn handle_association<S>(
    mut association: AsyncServerAssociation<S>,
    services: Arc<ServiceRegistry>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let peer = association.peer_ae_title().to_string();
    info!("Association established with {peer}");

    let ctx = AssociationContext::from_association(&association);
    let mut state = AssociationState::new();

    loop {
        match association.receive().await {
            Ok(pdu) => {
                debug!("scu ----> scp: {}", pdu.short_description());
                match pdu {
                    Pdu::PData { data } => {
                        if data.is_empty() {
                            continue;
                        }
                        if let Err(e) =
                            handle_pdata(&mut association, &services, &ctx, &mut state, data).await
                        {
                            warn!("DIMSE handling error: {e}");
                        }
                    }
                    Pdu::ReleaseRQ => {
                        let _ = association.send(&Pdu::ReleaseRP).await;
                        info!("Released association with {peer}");
                        break;
                    }
                    Pdu::AbortRQ { source } => {
                        warn!("Aborted connection from {peer}: {source:?}");
                        break;
                    }
                    _ => {}
                }
            }
            Err(err @ dicom_ul::association::Error::ReceivePdu { .. }) => {
                debug!("Association receive ended: {err}");
                break;
            }
            Err(err) => {
                warn!("Unexpected association error: {err}");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_pdata<S>(
    association: &mut AsyncServerAssociation<S>,
    services: &ServiceRegistry,
    ctx: &AssociationContext,
    state: &mut AssociationState,
    data: Vec<PDataValue>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    for data_value in data {
        match (data_value.value_type, data_value.is_last) {
            (PDataValueType::Command, is_last_cmd) => {
                let command = parse_command(&data_value.data, data_value.presentation_context_id)?;

                match command.dimse {
                    Dimse::CEcho => {
                        if !is_last_cmd {
                            return Err(Error::InvalidCommand {
                                message: "fragmented C-ECHO command not supported".to_string(),
                            });
                        }
                        let status = handle_cecho(services)?;
                        send_command_response(
                            association,
                            data_value.presentation_context_id,
                            response::build_cecho_rsp(command.message_id, status)?,
                        )
                        .await?;
                    }
                    Dimse::CStore => {
                        state.pending = Some(PendingOperation::CStore(command));
                        state.dataset_buffer.clear();
                    }
                    Dimse::CFind => {
                        state.cancelled.store(false, Ordering::SeqCst);
                        state.active_message_id = Some(command.message_id);
                        state.pending = Some(PendingOperation::CFind(command));
                        state.dataset_buffer.clear();
                    }
                    Dimse::CMove => {
                        state.cancelled.store(false, Ordering::SeqCst);
                        state.active_message_id = Some(command.message_id);
                        state.pending = Some(PendingOperation::CMove(command));
                        state.dataset_buffer.clear();
                    }
                    Dimse::CGet => {
                        state.cancelled.store(false, Ordering::SeqCst);
                        state.active_message_id = Some(command.message_id);
                        state.pending = Some(PendingOperation::CGet(command));
                        state.dataset_buffer.clear();
                    }
                    Dimse::CCancel => {
                        if state.active_message_id == Some(command.message_id) {
                            state.cancelled.store(true, Ordering::SeqCst);
                        }
                    }
                }
            }
            (PDataValueType::Data, is_last) if state.pending.is_some() => {
                state.dataset_buffer.extend(data_value.data);

                if is_last {
                    let pending = state.pending.take().expect("pending operation");
                    let pc_id = data_value.presentation_context_id;
                    let identifier = std::mem::take(&mut state.dataset_buffer);

                    match pending {
                        PendingOperation::CStore(command) => {
                            let status = process_cstore(
                                association,
                                services,
                                ctx,
                                &command,
                                pc_id,
                                identifier,
                                true,
                            )
                            .await?;
                            send_cstore_response(association, &command, pc_id, status).await?;
                        }
                        PendingOperation::CFind(command) => {
                            process_cfind(association, services, &command, pc_id, &identifier)
                                .await?;
                        }
                        PendingOperation::CMove(command) => {
                            process_cmove(association, services, ctx, &command, pc_id, &identifier)
                                .await?;
                        }
                        PendingOperation::CGet(command) => {
                            process_cget(association, services, ctx, &command, pc_id, &identifier)
                                .await?;
                        }
                    }
                    state.active_message_id = None;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

async fn process_cfind<S>(
    association: &mut AsyncServerAssociation<S>,
    services: &ServiceRegistry,
    command: &DimseMessage,
    pc_id: u8,
    identifier: &[u8],
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let Some(cfind) = services.cfind() else {
        send_command_response(
            association,
            pc_id,
            response::build_cfind_rsp(command.message_id, Status::SOP_CLASS_NOT_SUPPORTED)?,
        )
        .await?;
        return Ok(());
    };

    let pc = association
        .presentation_contexts()
        .iter()
        .find(|pc| pc.id == pc_id)
        .ok_or_else(|| Error::MissingPresentationContext {
            abstract_syntax: command.affected_sop_class_uid.clone().unwrap_or_default(),
        })?;

    let matches = cfind
        .find_matches(identifier, &pc.transfer_syntax)
        .await
        .unwrap_or_default();

    for match_data in &matches {
        tracing::info!(target: "dicom_net.metrics", event = "cfind_pending", count = matches.len());
        let cmd = response::build_cfind_rsp(command.message_id, Status::PENDING)?;
        let pdu = Pdu::PData {
            data: vec![
                PDataValue {
                    presentation_context_id: pc_id,
                    value_type: PDataValueType::Command,
                    is_last: false,
                    data: cmd,
                },
                PDataValue {
                    presentation_context_id: pc_id,
                    value_type: PDataValueType::Data,
                    is_last: true,
                    data: match_data.clone(),
                },
            ],
        };
        association
            .send(&pdu)
            .await
            .map_err(|source| Error::Ul { source })?;
    }

    send_command_response(
        association,
        pc_id,
        response::build_cfind_rsp(command.message_id, Status::SUCCESS)?,
    )
    .await
}

async fn process_cmove<S>(
    association: &mut AsyncServerAssociation<S>,
    services: &ServiceRegistry,
    ctx: &AssociationContext,
    command: &DimseMessage,
    pc_id: u8,
    identifier: &[u8],
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let Some(cmove) = services.cmove() else {
        send_command_response(
            association,
            pc_id,
            response::build_cmove_rsp(command.message_id, Status::SOP_CLASS_NOT_SUPPORTED, None)?,
        )
        .await?;
        return Ok(());
    };

    let move_dest = command
        .move_destination
        .as_deref()
        .ok_or_else(|| Error::InvalidCommand {
            message: "missing MoveDestination".to_string(),
        })?;

    let Some(remote) = cmove.move_destinations.get(move_dest).cloned() else {
        send_command_response(
            association,
            pc_id,
            response::build_cmove_rsp(command.message_id, Status::MOVE_DESTINATION_UNKNOWN, None)?,
        )
        .await?;
        return Ok(());
    };

    let pc = association
        .presentation_contexts()
        .iter()
        .find(|pc| pc.id == pc_id)
        .ok_or_else(|| Error::MissingPresentationContext {
            abstract_syntax: command.affected_sop_class_uid.clone().unwrap_or_default(),
        })?;

    let instances = cmove
        .sink
        .locate(identifier, &pc.transfer_syntax)
        .await
        .unwrap_or_default();

    let total = instances.len() as u16;
    send_command_response(
        association,
        pc_id,
        response::build_cmove_rsp(
            command.message_id,
            Status::PENDING,
            Some(response::SubOperationCounts {
                remaining: total,
                completed: 0,
                failed: 0,
                warning: 0,
            }),
        )?,
    )
    .await?;

    let counts = run_cmove_subops(
        &instances,
        move_dest,
        &remote,
        ctx.called_ae(),
        ctx.calling_ae(),
        command.message_id,
    )
    .await
    .unwrap_or_default();

    tracing::info!(
        target: "dicom_net.metrics",
        event = "cmove_complete",
        completed = counts.completed,
        failed = counts.failed
    );

    send_command_response(
        association,
        pc_id,
        response::build_cmove_rsp(command.message_id, Status::SUCCESS, Some(counts))?,
    )
    .await
}

async fn process_cget<S>(
    association: &mut AsyncServerAssociation<S>,
    services: &ServiceRegistry,
    ctx: &AssociationContext,
    command: &DimseMessage,
    pc_id: u8,
    identifier: &[u8],
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let Some(cget) = services.cget() else {
        send_command_response(
            association,
            pc_id,
            response::build_cget_rsp(command.message_id, Status::SOP_CLASS_NOT_SUPPORTED, None)?,
        )
        .await?;
        return Ok(());
    };

    let pc = association
        .presentation_contexts()
        .iter()
        .find(|pc| pc.id == pc_id)
        .ok_or_else(|| Error::MissingPresentationContext {
            abstract_syntax: command.affected_sop_class_uid.clone().unwrap_or_default(),
        })?;

    let instances = cget
        .sink
        .locate(identifier, &pc.transfer_syntax)
        .await
        .unwrap_or_default();

    let total = instances.len() as u16;
    send_command_response(
        association,
        pc_id,
        response::build_cget_rsp(
            command.message_id,
            Status::PENDING,
            Some(response::SubOperationCounts {
                remaining: total,
                completed: 0,
                failed: 0,
                warning: 0,
            }),
        )?,
    )
    .await?;

    let counts = run_cget_subops(association, &instances, command.message_id, None, None)
        .await
        .unwrap_or_default();

    tracing::info!(
        target: "dicom_net.metrics",
        event = "cget_complete",
        completed = counts.completed,
        failed = counts.failed
    );

    let _ = ctx;
    send_command_response(
        association,
        pc_id,
        response::build_cget_rsp(command.message_id, Status::SUCCESS, Some(counts))?,
    )
    .await
}

async fn process_cstore<S>(
    association: &mut AsyncServerAssociation<S>,
    services: &ServiceRegistry,
    ctx: &AssociationContext,
    command: &DimseMessage,
    presentation_context_id: u8,
    initial_data: Vec<u8>,
    data_complete: bool,
) -> Result<Status>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let sop_class = command.affected_sop_class_uid.as_deref().unwrap_or("");

    if services.resolve(sop_class).is_err() {
        return Ok(Status::SOP_CLASS_NOT_SUPPORTED);
    }

    let Some(cstore) = services.cstore() else {
        return Ok(Status::PROCESSING_FAILURE);
    };

    let pc = ctx
        .presentation_context(presentation_context_id)
        .ok_or_else(|| Error::MissingPresentationContext {
            abstract_syntax: sop_class.to_string(),
        })?;
    let transfer_syntax = pc.transfer_syntax.clone();

    let mut reader = DatasetReader::new(
        association,
        presentation_context_id,
        initial_data,
        data_complete,
    );
    cstore
        .handle_stream(command, &transfer_syntax, &mut reader, ctx)
        .await
}

async fn send_cstore_response<S>(
    association: &mut AsyncServerAssociation<S>,
    command: &DimseMessage,
    presentation_context_id: u8,
    status: Status,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let sop_class = command.affected_sop_class_uid.as_deref().unwrap_or("");
    let sop_instance = command.affected_sop_instance_uid.as_deref().unwrap_or("");
    send_command_response(
        association,
        presentation_context_id,
        response::build_cstore_rsp(command.message_id, sop_class, sop_instance, status)?,
    )
    .await
}

fn handle_cecho(services: &ServiceRegistry) -> Result<Status> {
    match services.resolve(dicom_dictionary_std::uids::VERIFICATION) {
        Ok(_) => Ok(Status::SUCCESS),
        Err(_) => Ok(Status::SOP_CLASS_NOT_SUPPORTED),
    }
}

async fn send_command_response<S>(
    association: &mut AsyncServerAssociation<S>,
    presentation_context_id: u8,
    command_bytes: Vec<u8>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let pdu = Pdu::PData {
        data: vec![PDataValue {
            presentation_context_id,
            value_type: PDataValueType::Command,
            is_last: true,
            data: command_bytes,
        }],
    };
    association
        .send(&pdu)
        .await
        .map_err(|source| Error::Ul { source })?;
    Ok(())
}
