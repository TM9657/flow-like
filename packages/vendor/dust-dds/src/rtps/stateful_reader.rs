use super::writer_proxy::RtpsWriterProxy;
use crate::{
    rtps_messages::{
        submessages::{data::DataSubmessage, data_frag::DataFragSubmessage},
        types::Time,
    },
    transport::types::{CacheChange, Guid, GuidPrefix, ReliabilityKind, WriterProxy},
};
use alloc::vec::Vec;

pub struct RtpsStatefulReader {
    guid: Guid,
    changes: Vec<CacheChange>,
    matched_writers: Vec<RtpsWriterProxy>,
    reliability: ReliabilityKind,
}

impl RtpsStatefulReader {
    pub const fn new(guid: Guid, reliability: ReliabilityKind) -> Self {
        Self {
            guid,
            changes: Vec::new(),
            matched_writers: Vec::new(),
            reliability,
        }
    }

    pub const fn guid(&self) -> Guid {
        self.guid
    }

    pub fn add_matched_writer(&mut self, writer_proxy: &WriterProxy) {
        let rtps_writer_proxy = RtpsWriterProxy::new(
            writer_proxy.remote_writer_guid,
            &writer_proxy.unicast_locator_list,
            &writer_proxy.multicast_locator_list,
            writer_proxy.remote_group_entity_id,
            writer_proxy.reliability_kind,
        );
        if let Some(wp) = self
            .matched_writers
            .iter_mut()
            .find(|wp| wp.remote_writer_guid() == writer_proxy.remote_writer_guid)
        {
            *wp = rtps_writer_proxy;
        } else {
            self.matched_writers.push(rtps_writer_proxy);
        }
    }

    pub fn delete_matched_writer(&mut self, writer_guid: Guid) {
        self.matched_writers
            .retain(|writer_proxy| writer_proxy.remote_writer_guid() != writer_guid)
    }

    pub fn matched_writer_lookup(&mut self, a_writer_guid: Guid) -> Option<&mut RtpsWriterProxy> {
        self.matched_writers
            .iter_mut()
            .find(|x| x.remote_writer_guid() == a_writer_guid)
    }

    pub fn reliability(&self) -> ReliabilityKind {
        self.reliability
    }

    pub fn on_data_submessage(
        &mut self,
        data_submessage: &DataSubmessage,
        source_guid_prefix: GuidPrefix,
        source_timestamp: Option<Time>,
    ) {
        let writer_guid = Guid::new(source_guid_prefix, data_submessage.writer_id());
        let sequence_number = data_submessage.writer_sn();
        if let Some(writer_proxy) = self
            .matched_writers
            .iter_mut()
            .find(|x| x.remote_writer_guid() == writer_guid)
        {
            match self.reliability {
                ReliabilityKind::BestEffort => {
                    let expected_seq_num = writer_proxy.available_changes_max() + 1;
                    if sequence_number >= expected_seq_num {
                        writer_proxy.received_change_set(sequence_number);
                        if sequence_number > expected_seq_num {
                            writer_proxy.lost_changes_update(sequence_number);
                        }

                        if let Ok(change) = CacheChange::try_from_data_submessage(
                            data_submessage,
                            source_guid_prefix,
                            source_timestamp,
                        ) {
                            self.changes.push(change);
                        }
                    }
                }
                ReliabilityKind::Reliable => {
                    let expected_seq_num = writer_proxy.available_changes_max() + 1;
                    if sequence_number == expected_seq_num {
                        writer_proxy.received_change_set(sequence_number);

                        if let Ok(change) = CacheChange::try_from_data_submessage(
                            data_submessage,
                            source_guid_prefix,
                            source_timestamp,
                        ) {
                            self.changes.push(change);
                        }
                    }
                }
            }
        }
    }

    pub fn on_data_frag_submessage(
        &mut self,
        data_frag_submessage: &DataFragSubmessage,
        source_guid_prefix: GuidPrefix,
        source_timestamp: Option<Time>,
    ) {
        let writer_guid = Guid::new(source_guid_prefix, data_frag_submessage.writer_id());
        let sequence_number = data_frag_submessage.writer_sn();
        if let Some(writer_proxy) = self
            .matched_writers
            .iter_mut()
            .find(|x| x.remote_writer_guid() == writer_guid)
        {
            match self.reliability {
                ReliabilityKind::BestEffort => {
                    let expected_seq_num = writer_proxy.available_changes_max() + 1;
                    if sequence_number >= expected_seq_num {
                        writer_proxy.push_data_frag(data_frag_submessage.clone());
                    }
                }
                ReliabilityKind::Reliable => {
                    let expected_seq_num = writer_proxy.available_changes_max() + 1;
                    if sequence_number == expected_seq_num {
                        writer_proxy.push_data_frag(data_frag_submessage.clone());
                    }
                }
            }

            if let Some(data_submessage) = writer_proxy.reconstruct_data_from_frag(sequence_number)
            {
                self.on_data_submessage(&data_submessage, source_guid_prefix, source_timestamp);
            }
        };
    }

    pub fn changes_mut(&mut self) -> &mut Vec<CacheChange> {
        &mut self.changes
    }
}

// The methods in this impl block are not defined by the standard
impl RtpsStatefulReader {
    pub fn is_historical_data_received(&self) -> bool {
        !self
            .matched_writers
            .iter()
            .any(|p| !p.is_historical_data_received())
    }
}
