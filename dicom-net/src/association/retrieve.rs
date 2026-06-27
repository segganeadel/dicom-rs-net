//! C-MOVE / C-GET retrieve engine.

use std::path::PathBuf;

use dicom_core::{DataElement, VR, dicom_value};
use dicom_dictionary_std::tags;
use dicom_encoding::TransferSyntaxIndex;
use dicom_object::{InMemDicomObject, OpenFileOptions};
use dicom_transfer_syntax_registry::TransferSyntaxRegistry;
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;
use dicom_ul::association::client::AsyncClientAssociation;
use dicom_ul::association::client::ClientAssociationOptions;
use dicom_ul::association::server::AsyncServerAssociation;
use dicom_ul::pdu::{PDataValue, PDataValueType, Pdu};

use crate::device::{AssociationRegistry, SharedAssociationRegistry};
use crate::dimse::request::build_cstore_rq;
use crate::dimse::response::SubOperationCounts;
use crate::dimse::rsp::parse_response;
use crate::error::{Error, Result};
use crate::qr::QueryRetrieveLevel;
use crate::status::Status;

/// Source data for a retrieve sub-operation.
#[derive(Debug, Clone)]
pub enum RetrieveSource {
    /// DICOM file on disk.
    File(PathBuf),
    /// Raw dataset bytes.
    Bytes(Vec<u8>),
}

/// Locator for a storage instance to retrieve.
#[derive(Debug, Clone)]
pub struct InstanceLocator {
    /// Storage SOP class UID.
    pub sop_class_uid: String,
    /// Storage SOP instance UID.
    pub sop_instance_uid: String,
    /// Transfer syntax UID.
    pub transfer_syntax_uid: String,
    /// Where to read the dataset from.
    pub source: RetrieveSource,
}

/// Resolves C-MOVE / C-GET identifier keys to instances.
#[async_trait::async_trait]
pub trait CRetrieveSink: Send + Sync {
    /// Locates instances matching the query identifier.
    async fn locate(
        &self,
        identifier: &[u8],
        transfer_syntax: &str,
    ) -> Result<Vec<InstanceLocator>>;
}

/// Runs C-STORE sub-operations for retrieve on the same association (C-GET).
pub async fn run_cget_subops<S>(
    association: &mut AsyncServerAssociation<S>,
    instances: &[InstanceLocator],
    message_id: u16,
    move_originator_ae: Option<&str>,
    move_originator_message_id: Option<u16>,
) -> Result<SubOperationCounts>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let total = instances.len() as u16;
    let mut completed = 0u16;
    let mut failed = 0u16;

    for inst in instances.iter() {
        let remaining = total.saturating_sub(completed + failed + 1);
        match send_cstore_subop(
            association,
            inst,
            message_id,
            move_originator_ae,
            move_originator_message_id,
        )
        .await
        {
            Ok(()) => completed += 1,
            Err(e) => {
                tracing::warn!("C-GET sub-op failed for {}: {e}", inst.sop_instance_uid);
                failed += 1;
            }
        }
        let _ = remaining;
    }

    Ok(SubOperationCounts {
        remaining: 0,
        completed,
        failed,
        warning: 0,
    })
}

/// Runs C-STORE sub-operations to a move destination AE (C-MOVE).
pub async fn run_cmove_subops(
    instances: &[InstanceLocator],
    move_destination: &str,
    remote_addr: &str,
    scp_ae_title: &str,
    move_originator_ae: &str,
    originator_message_id: u16,
    registry: Option<&SharedAssociationRegistry>,
    ae_id: &str,
    connection_id: &str,
    connection_index: usize,
) -> Result<SubOperationCounts> {
    let mut completed = 0u16;
    let mut failed = 0u16;

    if instances.is_empty() {
        return Ok(SubOperationCounts::default());
    }

    let mut options = ClientAssociationOptions::new()
        .calling_ae_title(scp_ae_title.to_string())
        .called_ae_title(move_destination.to_string());

    let mut by_abstract: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for inst in instances {
        by_abstract
            .entry(inst.sop_class_uid.clone())
            .or_default()
            .push(inst.transfer_syntax_uid.clone());
    }
    for (abstract_syntax, transfer_syntaxes) in by_abstract {
        options = options.with_presentation_context(abstract_syntax, transfer_syntaxes);
    }

    let mut assoc = options
        .establish_with_async(remote_addr)
        .await
        .map_err(|source| Error::Ul { source })?;

    let _guard = registry.map(|registry| {
        let guard = AssociationRegistry::register_outbound(
            registry,
            ae_id,
            scp_ae_title,
            move_destination,
            connection_id,
            connection_index,
            remote_addr,
        );
        registry.set_active(guard.id());
        guard
    });

    for (idx, inst) in instances.iter().enumerate() {
        let dataset = read_instance_dataset(inst)?;
        let pc = assoc
            .presentation_contexts()
            .iter()
            .find(|pc| pc.abstract_syntax == inst.sop_class_uid)
            .ok_or_else(|| Error::MissingPresentationContext {
                abstract_syntax: inst.sop_class_uid.clone(),
            })?;

        let msg_id = (idx as u16) + 1;
        let cmd = {
            let obj = InMemDicomObject::command_from_element_iter([
                DataElement::new(
                    tags::AFFECTED_SOP_CLASS_UID,
                    VR::UI,
                    dicom_value!(Str, inst.sop_class_uid.as_str()),
                ),
                DataElement::new(tags::COMMAND_FIELD, VR::US, dicom_value!(U16, [0x0001])),
                DataElement::new(tags::MESSAGE_ID, VR::US, dicom_value!(U16, [msg_id])),
                DataElement::new(tags::PRIORITY, VR::US, dicom_value!(U16, [0x0000])),
                DataElement::new(
                    tags::COMMAND_DATA_SET_TYPE,
                    VR::US,
                    dicom_value!(U16, [0x0000]),
                ),
                DataElement::new(
                    tags::AFFECTED_SOP_INSTANCE_UID,
                    VR::UI,
                    dicom_value!(Str, inst.sop_instance_uid.as_str()),
                ),
                DataElement::new(
                    tags::MOVE_ORIGINATOR_APPLICATION_ENTITY_TITLE,
                    VR::AE,
                    dicom_value!(Str, move_originator_ae),
                ),
                DataElement::new(
                    tags::MOVE_ORIGINATOR_MESSAGE_ID,
                    VR::US,
                    dicom_value!(U16, [originator_message_id]),
                ),
            ]);
            let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
            let mut data = Vec::new();
            obj.write_dataset_with_ts(&mut data, &ts)
                .map_err(|e| Error::InvalidCommand {
                    message: e.to_string(),
                })?;
            data
        };

        let pdu = Pdu::PData {
            data: vec![
                PDataValue {
                    presentation_context_id: pc.id,
                    value_type: PDataValueType::Command,
                    is_last: false,
                    data: cmd,
                },
                PDataValue {
                    presentation_context_id: pc.id,
                    value_type: PDataValueType::Data,
                    is_last: true,
                    data: dataset,
                },
            ],
        };
        if assoc.send(&pdu).await.is_err() {
            failed += 1;
            continue;
        }
        match receive_cstore_rsp_client(&mut assoc).await {
            Ok(status) if status.is_success() || status.is_warning() => completed += 1,
            _ => failed += 1,
        }
    }

    let _ = assoc.send(&Pdu::ReleaseRQ).await;
    let _ = assoc.receive().await;

    Ok(SubOperationCounts {
        remaining: 0,
        completed,
        failed,
        warning: 0,
    })
}

async fn send_cstore_subop<S>(
    association: &mut AsyncServerAssociation<S>,
    inst: &InstanceLocator,
    _message_id: u16,
    move_originator_ae: Option<&str>,
    move_originator_message_id: Option<u16>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let dataset = read_instance_dataset(inst)?;
    let pc = association
        .presentation_contexts()
        .iter()
        .find(|pc| pc.abstract_syntax == inst.sop_class_uid)
        .ok_or_else(|| Error::MissingPresentationContext {
            abstract_syntax: inst.sop_class_uid.clone(),
        })?;

    let msg_id = 1u16;
    let cmd = if let (Some(ae), Some(mid)) = (move_originator_ae, move_originator_message_id) {
        let obj = InMemDicomObject::command_from_element_iter([
            DataElement::new(
                tags::AFFECTED_SOP_CLASS_UID,
                VR::UI,
                dicom_value!(Str, inst.sop_class_uid.as_str()),
            ),
            DataElement::new(tags::COMMAND_FIELD, VR::US, dicom_value!(U16, [0x0001])),
            DataElement::new(tags::MESSAGE_ID, VR::US, dicom_value!(U16, [msg_id])),
            DataElement::new(tags::PRIORITY, VR::US, dicom_value!(U16, [0x0000])),
            DataElement::new(
                tags::COMMAND_DATA_SET_TYPE,
                VR::US,
                dicom_value!(U16, [0x0000]),
            ),
            DataElement::new(
                tags::AFFECTED_SOP_INSTANCE_UID,
                VR::UI,
                dicom_value!(Str, inst.sop_instance_uid.as_str()),
            ),
            DataElement::new(
                tags::MOVE_ORIGINATOR_APPLICATION_ENTITY_TITLE,
                VR::AE,
                dicom_value!(Str, ae),
            ),
            DataElement::new(
                tags::MOVE_ORIGINATOR_MESSAGE_ID,
                VR::US,
                dicom_value!(U16, [mid]),
            ),
        ]);
        let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
        let mut data = Vec::new();
        obj.write_dataset_with_ts(&mut data, &ts)
            .map_err(|e| Error::InvalidCommand {
                message: e.to_string(),
            })?;
        data
    } else {
        build_cstore_rq(&inst.sop_class_uid, &inst.sop_instance_uid, msg_id)?
    };

    let pdu = Pdu::PData {
        data: vec![
            PDataValue {
                presentation_context_id: pc.id,
                value_type: PDataValueType::Command,
                is_last: false,
                data: cmd,
            },
            PDataValue {
                presentation_context_id: pc.id,
                value_type: PDataValueType::Data,
                is_last: true,
                data: dataset,
            },
        ],
    };
    association
        .send(&pdu)
        .await
        .map_err(|source| Error::Ul { source })?;
    let rsp = receive_cstore_rsp_assoc(association).await?;
    if rsp.is_success() || rsp.is_warning() {
        Ok(())
    } else {
        Err(Error::DimseFailure { status: rsp.0 })
    }
}

async fn receive_cstore_rsp_client(
    association: &mut AsyncClientAssociation<tokio::net::TcpStream>,
) -> Result<Status> {
    loop {
        let pdu = association
            .receive()
            .await
            .map_err(|source| Error::Ul { source })?;
        match pdu {
            Pdu::PData { data } => {
                if let Some(cmd) = data
                    .iter()
                    .find(|v| v.value_type == PDataValueType::Command)
                {
                    return Ok(parse_response(&cmd.data)?.status);
                }
            }
            Pdu::ReleaseRQ | Pdu::AbortRQ { .. } => {
                return Err(Error::InvalidCommand {
                    message: "association closed awaiting C-STORE response".to_string(),
                });
            }
            _ => {}
        }
    }
}

async fn receive_cstore_rsp_assoc<S>(association: &mut AsyncServerAssociation<S>) -> Result<Status>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    loop {
        let pdu = association
            .receive()
            .await
            .map_err(|source| Error::Ul { source })?;
        if let Pdu::PData { data } = pdu {
            if let Some(cmd) = data
                .iter()
                .find(|v| v.value_type == PDataValueType::Command && v.is_last)
            {
                return Ok(parse_response(&cmd.data)?.status);
            }
        }
    }
}

fn read_instance_dataset(inst: &InstanceLocator) -> Result<Vec<u8>> {
    match &inst.source {
        RetrieveSource::Bytes(b) => Ok(b.clone()),
        RetrieveSource::File(path) => {
            let obj = OpenFileOptions::new()
                .open_file(path)
                .map_err(|e| Error::Io {
                    source: std::io::Error::other(e.to_string()),
                })?;
            let ts_uid = obj.meta().transfer_syntax();
            let ts = TransferSyntaxRegistry
                .get(ts_uid)
                .ok_or_else(|| Error::InvalidCommand {
                    message: format!("unknown transfer syntax {ts_uid}"),
                })?;
            let mut data = Vec::new();
            obj.write_dataset_with_ts(&mut data, ts)
                .map_err(|e| Error::InvalidCommand {
                    message: e.to_string(),
                })?;
            Ok(data)
        }
    }
}

/// Parses query retrieve level from an identifier dataset.
#[allow(dead_code)]
pub fn parse_query_level(identifier: &[u8], transfer_syntax: &str) -> Result<QueryRetrieveLevel> {
    use dicom_transfer_syntax_registry::TransferSyntaxRegistry;
    let ts = TransferSyntaxRegistry
        .get(transfer_syntax)
        .ok_or_else(|| Error::InvalidCommand {
            message: format!("unknown transfer syntax {transfer_syntax}"),
        })?;
    let obj = InMemDicomObject::read_dataset_with_ts(identifier, ts).map_err(|e| {
        Error::InvalidCommand {
            message: e.to_string(),
        }
    })?;
    let level_str = obj
        .element(tags::QUERY_RETRIEVE_LEVEL)
        .map_err(|_| Error::InvalidCommand {
            message: "missing QueryRetrieveLevel".to_string(),
        })?
        .to_str()
        .map_err(|_| Error::InvalidCommand {
            message: "QueryRetrieveLevel is not a string".to_string(),
        })?;
    level_str.parse().map_err(|_| Error::InvalidCommand {
        message: format!("unsupported QueryRetrieveLevel: {level_str}"),
    })
}

/// File-backed retrieve sink for tests.
pub struct FileRetrieveSink {
    files: Vec<PathBuf>,
}

impl FileRetrieveSink {
    /// Creates a sink that can retrieve the given files.
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self { files }
    }
}

#[async_trait::async_trait]
impl CRetrieveSink for FileRetrieveSink {
    async fn locate(
        &self,
        _identifier: &[u8],
        _transfer_syntax: &str,
    ) -> Result<Vec<InstanceLocator>> {
        let mut out = Vec::new();
        for path in &self.files {
            let obj = OpenFileOptions::new()
                .open_file(path)
                .map_err(|e| Error::Io {
                    source: std::io::Error::other(e.to_string()),
                })?;
            let meta = obj.meta();
            out.push(InstanceLocator {
                sop_class_uid: meta
                    .media_storage_sop_class_uid
                    .trim_end_matches('\0')
                    .to_string(),
                sop_instance_uid: meta
                    .media_storage_sop_instance_uid
                    .trim_end_matches('\0')
                    .to_string(),
                transfer_syntax_uid: meta.transfer_syntax().to_string(),
                source: RetrieveSource::File(path.clone()),
            });
        }
        Ok(out)
    }
}
