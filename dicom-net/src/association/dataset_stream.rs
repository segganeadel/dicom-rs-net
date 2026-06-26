//! Streaming reader for C-STORE dataset P-DATA PDVs.

use std::collections::VecDeque;

use async_trait::async_trait;
use dicom_ul::association::server::AsyncServerAssociation;
use dicom_ul::pdu::{PDataValueType, Pdu};

use crate::error::{Error, Result};

/// Object-safe async byte stream for incoming C-STORE datasets.
#[async_trait]
pub trait DatasetStream: Send {
    /// Reads up to `buf.len()` bytes from the dataset stream.
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}

/// Reads C-STORE dataset bytes from P-DATA PDVs (Data type only).
pub struct DatasetReader<'a, S> {
    association: &'a mut AsyncServerAssociation<S>,
    buffer: VecDeque<u8>,
    presentation_context_id: u8,
    finished: bool,
}

impl<'a, S> DatasetReader<'a, S> {
    /// Creates a reader for the dataset following a C-STORE-RQ command.
    ///
    /// `data_complete` is `true` when `initial` already contains the full dataset.
    pub fn new(
        association: &'a mut AsyncServerAssociation<S>,
        presentation_context_id: u8,
        initial: Vec<u8>,
        data_complete: bool,
    ) -> Self {
        Self {
            association,
            buffer: VecDeque::from(initial),
            presentation_context_id,
            finished: data_complete,
        }
    }

    /// Reads up to `buf.len()` bytes from the dataset stream.
    ///
    /// Returns `0` at end-of-stream.
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
    {
        if self.finished && self.buffer.is_empty() {
            return Ok(0);
        }

        while self.buffer.is_empty() && !self.finished {
            self.fill_buffer().await?;
        }

        let to_copy = buf.len().min(self.buffer.len());
        for (i, byte) in self.buffer.drain(..to_copy).enumerate() {
            buf[i] = byte;
        }
        Ok(to_copy)
    }

    async fn fill_buffer(&mut self) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
    {
        let pdu = self
            .association
            .receive()
            .await
            .map_err(|source| Error::Ul { source })?;
        match pdu {
            Pdu::PData { data } => {
                for data_value in data {
                    if data_value.presentation_context_id != self.presentation_context_id {
                        return Err(Error::InvalidCommand {
                            message: format!(
                                "presentation context mismatch: expected {}, got {}",
                                self.presentation_context_id, data_value.presentation_context_id
                            ),
                        });
                    }
                    if data_value.value_type != PDataValueType::Data {
                        return Err(Error::InvalidCommand {
                            message: "unexpected command PDV in dataset stream".to_string(),
                        });
                    }
                    self.buffer.extend(data_value.data);
                    if data_value.is_last {
                        self.finished = true;
                    }
                }
            }
            _ => {
                return Err(Error::InvalidCommand {
                    message: "unexpected PDU type while reading dataset".to_string(),
                });
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<S> DatasetStream for DatasetReader<'_, S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        DatasetReader::read(self, buf).await
    }
}
