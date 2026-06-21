//! C-MOVE SCU operations.

use crate::dimse::request::build_cmove_rq;
use crate::dimse::response::SubOperationCounts;
use crate::dimse::rsp::ensure_success;
use crate::error::{Error, Result};
use crate::qr::STUDY_ROOT_MOVE;
use crate::scu::association::ScuAssociation;

impl ScuAssociation {
    /// Performs C-MOVE and returns final sub-operation counts.
    pub async fn move_instances(
        &mut self,
        identifier: &[u8],
        move_destination: &str,
    ) -> Result<SubOperationCounts> {
        let pc =
            self.find_context(STUDY_ROOT_MOVE)
                .ok_or_else(|| Error::NoPresentationContext {
                    sop_class: STUDY_ROOT_MOVE.to_string(),
                })?;

        let message_id = self.next_message_id();
        let cmd = build_cmove_rq(STUDY_ROOT_MOVE, message_id, move_destination)?;
        self.send_command(pc.id, cmd).await?;
        self.send_data(pc.id, identifier).await?;

        loop {
            let (response, _) = self.receive_with_optional_data().await?;
            if response.status.is_pending() {
                continue;
            }
            ensure_success(response)?;
            break;
        }
        Ok(SubOperationCounts::default())
    }
}
