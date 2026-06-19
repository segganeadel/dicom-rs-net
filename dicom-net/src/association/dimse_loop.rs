//! DIMSE request handling loop for an established association.

use std::sync::Arc;

use dicom_ul::association::server::AsyncServerAssociation;
use dicom_ul::association::Association;
use dicom_ul::pdu::{PDataValue, PDataValueType, Pdu};
use tracing::{debug, info, warn};

use crate::association::dataset_stream::DatasetReader;
use crate::association::AssociationContext;
use crate::dimse::{parse::parse_command, response, Dimse, DimseMessage};
use crate::error::{Error, Result};
use crate::service::ServiceRegistry;
use crate::status::Status;

struct AssociationState {
    pending_cstore: Option<DimseMessage>,
    dataset_buffer: Vec<u8>,
}

impl AssociationState {
    fn new() -> Self {
        Self {
            pending_cstore: None,
            dataset_buffer: Vec::new(),
        }
    }
}

/// Handles DIMSE traffic on an established server association.
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
            (PDataValueType::Command, true) => {
                let command = parse_command(&data_value.data, data_value.presentation_context_id)?;

                match command.dimse {
                    Dimse::CEcho => {
                        let status = handle_cecho(services)?;
                        send_command_response(
                            association,
                            data_value.presentation_context_id,
                            response::build_cecho_rsp(command.message_id, status)?,
                        )
                        .await?;
                    }
                    Dimse::CStore => {
                        state.pending_cstore = Some(command);
                        state.dataset_buffer.clear();
                    }
                    _ => {
                        return Err(Error::UnexpectedDimse {
                            expected: Dimse::CStore,
                            got: command.command_field.as_u16(),
                        });
                    }
                }
            }
            (PDataValueType::Data, is_last) if state.pending_cstore.is_some() => {
                state.dataset_buffer.extend(data_value.data);

                if is_last {
                    let command = state.pending_cstore.take().expect("pending C-STORE");
                    let status = process_cstore(
                        association,
                        services,
                        ctx,
                        &command,
                        data_value.presentation_context_id,
                        std::mem::take(&mut state.dataset_buffer),
                        true,
                    )
                    .await?;

                    send_cstore_response(
                        association,
                        &command,
                        data_value.presentation_context_id,
                        status,
                    )
                    .await?;
                }
            }
            (PDataValueType::Command, false) => {
                return Err(Error::InvalidCommand {
                    message: "fragmented command PDV not supported".to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(())
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
