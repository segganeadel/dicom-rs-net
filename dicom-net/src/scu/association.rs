//! Established SCU association wrapper.

use dicom_ul::association::Association;
use dicom_ul::association::client::AsyncClientAssociation;
use dicom_ul::pdu::{PDataValue, PDataValueType, Pdu};

use crate::dimse::rsp::parse_response;
use crate::error::{Error, Result};

enum ScuStream {
    Plain(AsyncClientAssociation<tokio::net::TcpStream>),
    #[cfg(feature = "tls")]
    Tls(AsyncClientAssociation<dicom_ul::association::client::AsyncTlsStream>),
}

/// An established DICOM association from the SCU perspective.
pub struct ScuAssociation {
    stream: ScuStream,
    next_message_id: u16,
}

macro_rules! with_inner {
    ($self:expr, |$inner:ident| $body:expr) => {
        match &mut $self.stream {
            ScuStream::Plain($inner) => $body,
            #[cfg(feature = "tls")]
            ScuStream::Tls($inner) => $body,
        }
    };
    ($self:expr, |$inner:ident| $body:expr, read_only) => {
        match &$self.stream {
            ScuStream::Plain($inner) => $body,
            #[cfg(feature = "tls")]
            ScuStream::Tls($inner) => $body,
        }
    };
}

impl ScuAssociation {
    pub(crate) fn new(inner: AsyncClientAssociation<tokio::net::TcpStream>) -> Self {
        Self {
            stream: ScuStream::Plain(inner),
            next_message_id: 1,
        }
    }

    #[cfg(feature = "tls")]
    pub(crate) fn new_tls(
        inner: AsyncClientAssociation<dicom_ul::association::client::AsyncTlsStream>,
    ) -> Self {
        Self {
            stream: ScuStream::Tls(inner),
            next_message_id: 1,
        }
    }

    /// Returns the peer AE title.
    pub fn peer_ae_title(&self) -> &str {
        with_inner!(self, |inner| inner.peer_ae_title(), read_only)
    }

    /// Returns negotiated presentation contexts.
    pub fn presentation_contexts(&self) -> &[dicom_ul::pdu::PresentationContextNegotiated] {
        with_inner!(self, |inner| inner.presentation_contexts(), read_only)
    }

    pub(crate) fn acceptor_max_pdu_length(&self) -> u32 {
        with_inner!(self, |inner| inner.acceptor_max_pdu_length(), read_only)
    }

    pub(crate) fn verification_context(
        &self,
    ) -> Option<dicom_ul::pdu::PresentationContextNegotiated> {
        self.find_context(dicom_dictionary_std::uids::VERIFICATION)
    }

    pub(crate) fn find_context(
        &self,
        abstract_syntax: &str,
    ) -> Option<dicom_ul::pdu::PresentationContextNegotiated> {
        self.presentation_contexts()
            .iter()
            .find(|pc| pc.abstract_syntax == abstract_syntax)
            .cloned()
    }

    pub(crate) async fn send_data(&mut self, pc_id: u8, data: &[u8]) -> Result<()> {
        let pdu = Pdu::PData {
            data: vec![PDataValue {
                presentation_context_id: pc_id,
                value_type: PDataValueType::Data,
                is_last: true,
                data: data.to_vec(),
            }],
        };
        self.send_pdu(pdu).await
    }

    pub(crate) async fn receive_with_optional_data(
        &mut self,
    ) -> Result<(crate::dimse::rsp::DimseResponse, Option<Vec<u8>>)> {
        loop {
            let pdu = with_inner!(self, |inner| inner
                .receive()
                .await
                .map_err(|source| Error::Ul { source }))?;

            match pdu {
                Pdu::PData { data } => {
                    let mut response = None;
                    let mut dataset = None;
                    for pdv in data {
                        match pdv.value_type {
                            PDataValueType::Command => {
                                response = Some(parse_response(&pdv.data)?);
                            }
                            PDataValueType::Data if pdv.is_last => {
                                dataset = Some(pdv.data);
                            }
                            _ => {}
                        }
                    }
                    if let Some(rsp) = response {
                        return Ok((rsp, dataset));
                    }
                }
                Pdu::ReleaseRQ | Pdu::AbortRQ { .. } => {
                    return Err(Error::InvalidCommand {
                        message: "association closed while awaiting response".to_string(),
                    });
                }
                _ => continue,
            }
        }
    }

    pub(crate) fn next_message_id(&mut self) -> u16 {
        let id = self.next_message_id;
        self.next_message_id = self.next_message_id.wrapping_add(1).max(1);
        id
    }

    pub(crate) async fn send_command(&mut self, pc_id: u8, cmd: Vec<u8>) -> Result<()> {
        let pdu = Pdu::PData {
            data: vec![PDataValue {
                presentation_context_id: pc_id,
                value_type: PDataValueType::Command,
                is_last: true,
                data: cmd,
            }],
        };
        self.send_pdu(pdu).await
    }

    pub(crate) async fn send_pdu(&mut self, pdu: Pdu) -> Result<()> {
        with_inner!(self, |inner| inner
            .send(&pdu)
            .await
            .map_err(|source| Error::Ul { source }))
    }

    pub(crate) async fn receive_response(&mut self) -> Result<crate::dimse::rsp::DimseResponse> {
        loop {
            let pdu = with_inner!(self, |inner| inner
                .receive()
                .await
                .map_err(|source| Error::Ul { source }))?;

            match pdu {
                Pdu::PData { data } => {
                    let cmd = data
                        .iter()
                        .find(|v| v.value_type == PDataValueType::Command && v.is_last)
                        .ok_or_else(|| Error::InvalidCommand {
                            message: "missing command in response".to_string(),
                        })?;
                    return parse_response(&cmd.data);
                }
                Pdu::ReleaseRQ | Pdu::AbortRQ { .. } => {
                    return Err(Error::InvalidCommand {
                        message: "association closed while awaiting response".to_string(),
                    });
                }
                _ => continue,
            }
        }
    }

    pub(crate) async fn receive_raw_pdu(&mut self) -> Result<Pdu> {
        loop {
            let pdu = with_inner!(self, |inner| inner
                .receive()
                .await
                .map_err(|source| Error::Ul { source }))?;
            match pdu {
                Pdu::ReleaseRQ | Pdu::AbortRQ { .. } => {
                    return Err(Error::InvalidCommand {
                        message: "association closed while awaiting response".to_string(),
                    });
                }
                Pdu::PData { .. } => return Ok(pdu),
                _ => continue,
            }
        }
    }

    /// Releases the association gracefully.
    pub async fn release(mut self) -> Result<()> {
        with_inner!(self, |inner| inner
            .send(&Pdu::ReleaseRQ)
            .await
            .map_err(|source| Error::Ul { source }))?;
        let _ = with_inner!(self, |inner| inner.receive().await);
        Ok(())
    }
}
