//! C-GET SCU operations.

use dicom_ul::pdu::{PDataValueType, Pdu};

use crate::dimse::parse::{command_field_raw, parse_command};
use crate::dimse::request::build_cget_rq;
use crate::dimse::response::{SubOperationCounts, build_cstore_rsp};
use crate::dimse::rsp::{ensure_success, parse_response};
use crate::dimse::{CommandField, Dimse};
use crate::error::{Error, Result};
use crate::qr::STUDY_ROOT_GET;
use crate::scu::association::ScuAssociation;
use crate::status::Status;

impl ScuAssociation {
    /// Performs C-GET and waits for completion, accepting C-STORE sub-operations.
    pub async fn get_instances(&mut self, identifier: &[u8]) -> Result<SubOperationCounts> {
        let pc = self
            .find_context(STUDY_ROOT_GET)
            .ok_or_else(|| Error::NoPresentationContext {
                sop_class: STUDY_ROOT_GET.to_string(),
            })?;

        let message_id = self.next_message_id();
        let cmd = build_cget_rq(STUDY_ROOT_GET, message_id)?;
        self.send_command(pc.id, cmd).await?;
        self.send_data(pc.id, identifier).await?;

        let mut stored = 0u16;
        loop {
            match self.receive_cget_event().await? {
                CGetEvent::Pending => {}
                CGetEvent::CStore {
                    pc_id,
                    command,
                    data,
                } => {
                    let sop_class = command.affected_sop_class_uid.as_deref().unwrap_or("");
                    let sop_instance = command.affected_sop_instance_uid.as_deref().unwrap_or("");
                    let rsp = build_cstore_rsp(
                        command.message_id,
                        sop_class,
                        sop_instance,
                        Status::SUCCESS,
                    )?;
                    self.send_command(pc_id, rsp).await?;
                    let _ = data;
                    stored += 1;
                }
                CGetEvent::Final(response) => {
                    ensure_success(response)?;
                    break;
                }
            }
        }

        Ok(SubOperationCounts {
            remaining: 0,
            completed: stored,
            failed: 0,
            warning: 0,
        })
    }

    async fn receive_cget_event(&mut self) -> Result<CGetEvent> {
        loop {
            let pdu = self.receive_raw_pdu().await?;
            if let Pdu::PData { data } = pdu {
                let mut command_pdu = None;
                let mut dataset = None;
                let mut pc_id = 0u8;
                for pdv in data {
                    pc_id = pdv.presentation_context_id;
                    match pdv.value_type {
                        PDataValueType::Command => {
                            command_pdu = Some(pdv.data);
                        }
                        PDataValueType::Data if pdv.is_last => {
                            dataset = Some(pdv.data);
                        }
                        _ => {}
                    }
                }
                if let Some(bytes) = command_pdu {
                    let field = command_field_raw(&bytes)?;
                    if CommandField::from_u16(field)
                        .map(|f| f.is_response())
                        .unwrap_or(false)
                    {
                        let response = parse_response(&bytes)?;
                        if response.status.is_pending() {
                            return Ok(CGetEvent::Pending);
                        }
                        return Ok(CGetEvent::Final(response));
                    }
                    let cmd = parse_command(&bytes, pc_id)?;
                    if cmd.dimse == Dimse::CStore {
                        return Ok(CGetEvent::CStore {
                            pc_id,
                            command: cmd,
                            data: dataset.unwrap_or_default(),
                        });
                    }
                }
            }
        }
    }
}

enum CGetEvent {
    Pending,
    CStore {
        pc_id: u8,
        command: crate::dimse::DimseMessage,
        data: Vec<u8>,
    },
    Final(crate::dimse::rsp::DimseResponse),
}
