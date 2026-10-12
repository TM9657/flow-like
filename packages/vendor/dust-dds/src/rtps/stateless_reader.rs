use crate::{
    rtps_messages::{submessages::data::DataSubmessage, types::Time},
    transport::types::{CacheChange, ENTITYID_UNKNOWN, Guid, GuidPrefix},
};
use alloc::vec::Vec;

pub struct RtpsStatelessReader {
    guid: Guid,
    changes: Vec<CacheChange>,
}

impl RtpsStatelessReader {
    pub const fn new(guid: Guid) -> Self {
        Self {
            guid,
            changes: Vec::new(),
        }
    }

    pub const fn guid(&self) -> Guid {
        self.guid
    }

    #[tracing::instrument(skip(self))]
    pub fn on_data_submessage(
        &mut self,
        data_submessage: &DataSubmessage,
        source_guid_prefix: GuidPrefix,
        source_timestamp: Option<Time>,
    ) {
        if data_submessage.reader_id() == ENTITYID_UNKNOWN
            || data_submessage.reader_id() == self.guid.entity_id()
        {
            if let Ok(change) = CacheChange::try_from_data_submessage(
                data_submessage,
                source_guid_prefix,
                source_timestamp,
            ) {
                tracing::trace!(guid=?self.guid, "Change pushed into history cache");
                self.changes.push(change);
            }
        }
    }

    pub fn changes_mut(&mut self) -> &mut Vec<CacheChange> {
        &mut self.changes
    }
}
