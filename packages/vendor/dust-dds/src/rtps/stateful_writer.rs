use super::{behavior_types::Duration, reader_proxy::RtpsReaderProxy};
use crate::{
    infrastructure::time::Time,
    rtps_messages::{
        overall_structure::RtpsMessageWrite,
        submessage_elements::SequenceNumberSet,
        submessages::{
            ack_nack::AckNackSubmessage, gap::GapSubmessage,
            info_destination::InfoDestinationSubmessage, info_timestamp::InfoTimestampSubmessage,
            nack_frag::NackFragSubmessage,
        },
        types::TIME_INVALID,
    },
    transport::{
        interface::WriteMessage,
        types::{
            CacheChange, ChangeKind, DurabilityKind, ENTITYID_UNKNOWN, EntityId, Guid, GuidPrefix,
            ReaderProxy, ReliabilityKind, SequenceNumber,
        },
    },
};
use alloc::vec::Vec;

pub struct RtpsStatefulWriter {
    guid: Guid,
    changes: Vec<CacheChange>,
    matched_readers: Vec<RtpsReaderProxy>,
    heartbeat_period: Duration,
    data_max_size_serialized: usize,
}

impl RtpsStatefulWriter {
    pub fn new(guid: Guid, data_max_size_serialized: usize) -> Self {
        Self {
            guid,
            changes: Vec::new(),
            matched_readers: Vec::new(),
            heartbeat_period: Duration::from_millis(200),
            data_max_size_serialized,
        }
    }

    pub fn guid(&self) -> Guid {
        self.guid
    }

    pub fn data_max_size_serialized(&self) -> usize {
        self.data_max_size_serialized
    }

    pub fn add_change(&mut self, cache_change: CacheChange) {
        self.changes.push(cache_change);
    }

    pub fn remove_change(&mut self, sequence_number: SequenceNumber) {
        self.changes
            .retain(|cc| cc.sequence_number != sequence_number);
    }

    pub fn is_change_acknowledged(&self, sequence_number: SequenceNumber) -> bool {
        !self
            .matched_readers
            .iter()
            .filter(|rp| rp.reliability() == ReliabilityKind::Reliable)
            .any(|rp| rp.unacked_changes(Some(sequence_number)))
    }

    pub fn time_until_next_heartbeat(
        &self,
        now: crate::infrastructure::time::Time,
    ) -> Option<crate::infrastructure::time::Duration> {
        if self.changes.is_empty() || self.matched_readers.is_empty() {
            return None;
        }
        let seq_num_max = self.changes.last()?.sequence_number;
        let mut min_time: Option<crate::infrastructure::time::Duration> = None;
        for rp in &self.matched_readers {
            if rp.reliability() == ReliabilityKind::Reliable
                && seq_num_max > rp.highest_acked_seq_num()
            {
                if let Some(d) =
                    rp.time_until_heartbeat(now, self.heartbeat_period.into(), Some(seq_num_max))
                {
                    min_time = Some(min_time.map_or(d, |min| min.min(d)));
                }
            }
        }
        min_time
    }

    pub fn add_matched_reader(&mut self, reader_proxy: ReaderProxy) {
        let first_relevant_sample_seq_num = match reader_proxy.durability_kind {
            DurabilityKind::Volatile => self
                .changes
                .last()
                .map(|cc| cc.sequence_number + 1)
                .unwrap_or(1),
            DurabilityKind::TransientLocal
            | DurabilityKind::Transient
            | DurabilityKind::Persistent => 1,
        };
        let highest_sent_seq_num = match reader_proxy.reliability_kind {
            ReliabilityKind::BestEffort => 0,
            ReliabilityKind::Reliable => self
                .changes
                .last()
                .map(|cc| cc.sequence_number)
                .unwrap_or(0),
        };
        let rtps_reader_proxy = RtpsReaderProxy::new(
            reader_proxy.remote_reader_guid,
            reader_proxy.remote_group_entity_id,
            &reader_proxy.unicast_locator_list,
            &reader_proxy.multicast_locator_list,
            reader_proxy.expects_inline_qos,
            true,
            reader_proxy.reliability_kind,
            first_relevant_sample_seq_num,
            reader_proxy.durability_kind,
            highest_sent_seq_num,
        );
        if let Some(rp) = self
            .matched_readers
            .iter_mut()
            .find(|rp| rp.remote_reader_guid() == reader_proxy.remote_reader_guid)
        {
            *rp = rtps_reader_proxy;
        } else {
            self.matched_readers.push(rtps_reader_proxy);
        }
    }

    pub fn delete_matched_reader(&mut self, reader_guid: Guid) {
        self.matched_readers
            .retain(|reader_proxy| reader_proxy.remote_reader_guid() != reader_guid);
    }

    pub fn write_message(&mut self, message_writer: &mut (impl WriteMessage + ?Sized), now: Time) {
        if self.changes.is_empty() || self.matched_readers.is_empty() {
            return;
        }
        for reader_proxy in &mut self.matched_readers {
            reader_proxy.write_message(
                self.guid.entity_id(),
                &self.changes,
                self.data_max_size_serialized,
                self.heartbeat_period,
                message_writer,
                now,
                self.guid.prefix(),
            )
        }
    }

    /// Process the received AckNack RTPS submessage. This method return an Option indicating the sequence number of the acknowledged change
    /// or None if no change has been acknowledged.
    pub fn on_acknack_submessage_received(
        &mut self,
        acknack_submessage: &AckNackSubmessage,
        source_guid_prefix: GuidPrefix,
        message_writer: &mut (impl WriteMessage + ?Sized),
        now: Time,
    ) -> Option<SequenceNumber> {
        if &self.guid.entity_id() == acknack_submessage.writer_id() {
            let reader_guid = Guid::new(source_guid_prefix, *acknack_submessage.reader_id());

            if let Some(reader_proxy) = self
                .matched_readers
                .iter_mut()
                .find(|x| x.remote_reader_guid() == reader_guid)
            {
                if reader_proxy.reliability() == ReliabilityKind::Reliable
                    && acknack_submessage.count() > reader_proxy.last_received_acknack_count()
                {
                    let acked_changes = acknack_submessage.reader_sn_state().base() - 1;
                    reader_proxy.acked_changes_set(acked_changes);
                    reader_proxy.requested_changes_set(acknack_submessage.reader_sn_state().set());

                    reader_proxy.set_last_received_acknack_count(acknack_submessage.count());

                    let is_preemptive = acknack_submessage.reader_sn_state().base() <= 0
                        || (acknack_submessage.reader_sn_state().base() == 1
                            && reader_proxy.highest_acked_seq_num() == 0
                            && acknack_submessage.reader_sn_state().set().next().is_none());
                    if is_preemptive {
                        reader_proxy.heartbeat_machine().reset_heartbeat_time();
                    }

                    reader_proxy.write_message_reliable(
                        self.guid.entity_id(),
                        &self.changes,
                        self.data_max_size_serialized,
                        self.heartbeat_period,
                        message_writer,
                        now,
                        self.guid.prefix(),
                    );
                    return Some(acked_changes);
                }
            }
        }
        None
    }

    pub fn on_nack_frag_submessage_received(
        &mut self,
        nackfrag_submessage: &NackFragSubmessage,
        source_guid_prefix: GuidPrefix,
        message_writer: &mut (impl WriteMessage + ?Sized),
    ) {
        let reader_guid = Guid::new(source_guid_prefix, nackfrag_submessage.reader_id());

        if let Some(reader_proxy) = self
            .matched_readers
            .iter_mut()
            .find(|x| x.remote_reader_guid() == reader_guid)
        {
            if reader_proxy.reliability() == ReliabilityKind::Reliable
                && nackfrag_submessage.count() > reader_proxy.last_received_nack_frag_count()
            {
                reader_proxy.set_last_received_nack_frag_count(nackfrag_submessage.count());
                let change_seq_num = nackfrag_submessage.writer_sn();
                if let Some(cache_change) = self
                    .changes
                    .iter()
                    .find(|cc| cc.sequence_number == change_seq_num)
                {
                    let number_of_fragments = cache_change
                        .data_value
                        .len()
                        .div_ceil(self.data_max_size_serialized);

                    for request_fragment_number in
                        core::iter::once(nackfrag_submessage.fragment_number_state().base())
                            .chain(nackfrag_submessage.fragment_number_state().set())
                    {
                        let request_fragment_number = request_fragment_number as usize;
                        // Either send a DATAFRAG submessages or send a single DATA submessage
                        if (1..=number_of_fragments).contains(&request_fragment_number)
                            && cache_change.kind == ChangeKind::Alive
                        {
                            let writer_id = self.guid.entity_id();
                            let reader_id = reader_proxy.remote_reader_guid().entity_id();
                            let data_frag = cache_change.as_data_frag_submessage(
                                reader_id,
                                writer_id,
                                self.data_max_size_serialized,
                                request_fragment_number - 1,
                            );

                            let info_dst = InfoDestinationSubmessage::new(
                                reader_proxy.remote_reader_guid().prefix(),
                            );
                            let info_timestamp =
                                if let Some(timestamp) = cache_change.source_timestamp {
                                    InfoTimestampSubmessage::new(false, timestamp.into())
                                } else {
                                    InfoTimestampSubmessage::new(true, TIME_INVALID)
                                };

                            let len = RtpsMessageWrite::from_submessages(
                                message_writer.write_buffer_mut(),
                                &[&info_dst, &info_timestamp, &data_frag],
                                self.guid.prefix(),
                            )
                            .buffer()
                            .len();
                            message_writer.write_message(len, reader_proxy.unicast_locator_list())
                        }
                    }
                } else {
                    let writer_id = self.guid.entity_id();
                    let info_dst =
                        InfoDestinationSubmessage::new(reader_proxy.remote_reader_guid().prefix());
                    let gap_submessage = GapSubmessage::new(
                        ENTITYID_UNKNOWN,
                        writer_id,
                        change_seq_num,
                        SequenceNumberSet::new(change_seq_num + 1, []),
                    );

                    let len = RtpsMessageWrite::from_submessages(
                        message_writer.write_buffer_mut(),
                        &[&info_dst, &gap_submessage],
                        self.guid.prefix(),
                    )
                    .buffer()
                    .len();
                    message_writer.write_message(len, reader_proxy.unicast_locator_list())
                }
            }
        }
    }

    pub fn changes(&self) -> &[CacheChange] {
        &self.changes
    }

    pub fn changes_mut(&mut self) -> &mut Vec<CacheChange> {
        &mut self.changes
    }
}

impl RtpsReaderProxy {
    #[allow(clippy::too_many_arguments)]
    fn write_message(
        &mut self,
        writer_id: EntityId,
        changes: &[CacheChange],
        data_max_size_serialized: usize,
        heartbeat_period: Duration,
        message_writer: &mut (impl WriteMessage + ?Sized),
        now: Time,
        guid_prefix: GuidPrefix,
    ) {
        match self.reliability() {
            ReliabilityKind::BestEffort => self.write_message_best_effort(
                writer_id,
                changes,
                data_max_size_serialized,
                message_writer,
                guid_prefix,
            ),
            ReliabilityKind::Reliable => self.write_message_reliable(
                writer_id,
                changes,
                data_max_size_serialized,
                heartbeat_period,
                message_writer,
                now,
                guid_prefix,
            ),
        }
    }

    fn write_message_best_effort(
        &mut self,
        writer_id: EntityId,
        changes: &[CacheChange],
        data_max_size_serialized: usize,
        message_writer: &mut (impl WriteMessage + ?Sized),
        guid_prefix: GuidPrefix,
    ) {
        // a_change_seq_num := the_reader_proxy.next_unsent_change();
        // a_change := the_writer.writer_cache.get_change(a_change_seq_num );
        // if ( DDS_FILTER(the_reader_proxy, a_change) ) {
        //      DATA = new DATA(a_change);
        //      IF (the_reader_proxy.expectsInlineQos) {
        //          DATA.inlineQos := the_rtps_writer.related_dds_writer.qos;
        //          DATA.inlineQos += a_change.inlineQos;
        //      }
        //      DATA.readerId := the_reader_proxy.remoteReaderGuid.entityId;
        //      send DATA;
        // }
        // the_reader_proxy.highest_sent_seq_num := a_change_seq_num;
        while let Some(next_unsent_change_seq_num) = self.next_unsent_change(changes) {
            if let Some(cache_change) = changes.iter().find(|cc| {
                cc.sequence_number == next_unsent_change_seq_num
                    && next_unsent_change_seq_num >= self.first_relevant_sample_seq_num()
            }) {
                let number_of_fragments = cache_change
                    .data_value
                    .len()
                    .div_ceil(data_max_size_serialized);

                let info_dst = InfoDestinationSubmessage::new(self.remote_reader_guid().prefix());

                let info_timestamp = if let Some(timestamp) = cache_change.source_timestamp {
                    InfoTimestampSubmessage::new(false, timestamp.into())
                } else {
                    InfoTimestampSubmessage::new(true, TIME_INVALID)
                };
                // Either send a DATAFRAG submessages or send a single DATA submessage
                if number_of_fragments > 1 {
                    for fragment_number in 0..number_of_fragments {
                        let reader_id = self.remote_reader_guid().entity_id();

                        let data_frag = cache_change.as_data_frag_submessage(
                            reader_id,
                            writer_id,
                            data_max_size_serialized,
                            fragment_number,
                        );
                        let len = RtpsMessageWrite::from_submessages(
                            message_writer.write_buffer_mut(),
                            &[&info_dst, &info_timestamp, &data_frag],
                            guid_prefix,
                        )
                        .buffer()
                        .len();
                        message_writer.write_message(len, self.unicast_locator_list())
                    }
                } else {
                    let inline_qos = match (
                        cache_change.status_info_parameter(),
                        cache_change.key_hash_parameter(),
                    ) {
                        (Some(s), Some(k)) => &[s, k][..],
                        (Some(s), None) => &[s][..],
                        (None, Some(k)) => &[k][..],
                        (None, None) => &[],
                    };
                    let data_submessage = cache_change.as_data_submessage(
                        self.remote_reader_guid().entity_id(),
                        writer_id,
                        inline_qos,
                    );

                    let len = RtpsMessageWrite::from_submessages(
                        message_writer.write_buffer_mut(),
                        &[&info_dst, &info_timestamp, &data_submessage],
                        guid_prefix,
                    )
                    .buffer()
                    .len();
                    message_writer.write_message(len, self.unicast_locator_list())
                }
            }

            self.set_highest_sent_seq_num(next_unsent_change_seq_num);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn write_message_reliable(
        &mut self,
        writer_id: EntityId,
        changes: &[CacheChange],
        data_max_size_serialized: usize,
        heartbeat_period: Duration,
        message_writer: &mut (impl WriteMessage + ?Sized),
        now: Time,
        guid_prefix: GuidPrefix,
    ) {
        let seq_num_min = changes.first().map(|cc| cc.sequence_number);
        let seq_num_max = changes.last().map(|cc| cc.sequence_number);
        // Top part of the state machine - Figure 8.19 RTPS standard
        if self.unsent_changes(changes) {
            while let Some(next_unsent_change_seq_num) = self.next_unsent_change(changes) {
                if next_unsent_change_seq_num > self.highest_sent_seq_num() + 1 {
                    let gap_start_sequence_number = self.highest_sent_seq_num() + 1;
                    let gap_end_sequence_number = next_unsent_change_seq_num - 1;
                    let gap_submessage = GapSubmessage::new(
                        self.remote_reader_guid().entity_id(),
                        writer_id,
                        gap_start_sequence_number,
                        SequenceNumberSet::new(gap_end_sequence_number + 1, []),
                    );
                    let info_dst =
                        InfoDestinationSubmessage::new(self.remote_reader_guid().prefix());
                    let len = RtpsMessageWrite::from_submessages(
                        message_writer.write_buffer_mut(),
                        &[&info_dst, &gap_submessage],
                        guid_prefix,
                    )
                    .buffer()
                    .len();
                    message_writer.write_message(len, self.unicast_locator_list());
                    self.set_highest_sent_seq_num(gap_end_sequence_number);
                }

                if let Some(cache_change) = changes.iter().find(|cc| {
                    cc.sequence_number == next_unsent_change_seq_num
                        && next_unsent_change_seq_num >= self.first_relevant_sample_seq_num()
                }) {
                    let number_of_fragments = cache_change
                        .data_value
                        .len()
                        .div_ceil(data_max_size_serialized);

                    // Either send a DATAFRAG submessages or send a single DATA submessage
                    if number_of_fragments > 1 && cache_change.kind == ChangeKind::Alive {
                        for fragment_number in 0..number_of_fragments {
                            let reader_id = self.remote_reader_guid().entity_id();
                            let data_frag = cache_change.as_data_frag_submessage(
                                reader_id,
                                writer_id,
                                data_max_size_serialized,
                                fragment_number,
                            );

                            let info_dst =
                                InfoDestinationSubmessage::new(self.remote_reader_guid().prefix());
                            let info_timestamp =
                                if let Some(timestamp) = cache_change.source_timestamp {
                                    InfoTimestampSubmessage::new(false, timestamp.into())
                                } else {
                                    InfoTimestampSubmessage::new(true, TIME_INVALID)
                                };

                            let len = if fragment_number == number_of_fragments - 1 {
                                let first_sn = seq_num_min
                                    .unwrap_or(1)
                                    .max(self.first_relevant_sample_seq_num());
                                let last_sn = seq_num_max.unwrap_or(0).max(first_sn - 1);
                                let heartbeat = self.heartbeat_machine().generate_new_heartbeat(
                                    writer_id, first_sn, last_sn, now, false,
                                );
                                RtpsMessageWrite::from_submessages(
                                    message_writer.write_buffer_mut(),
                                    &[&info_dst, &info_timestamp, &data_frag, &heartbeat],
                                    guid_prefix,
                                )
                            } else {
                                RtpsMessageWrite::from_submessages(
                                    message_writer.write_buffer_mut(),
                                    &[&info_dst, &info_timestamp, &data_frag],
                                    guid_prefix,
                                )
                            }
                            .buffer()
                            .len();
                            message_writer.write_message(len, self.unicast_locator_list())
                        }
                    } else {
                        let info_dst =
                            InfoDestinationSubmessage::new(self.remote_reader_guid().prefix());

                        let info_timestamp = if let Some(timestamp) = cache_change.source_timestamp
                        {
                            InfoTimestampSubmessage::new(false, timestamp.into())
                        } else {
                            InfoTimestampSubmessage::new(true, TIME_INVALID)
                        };

                        let inline_qos = match (
                            cache_change.status_info_parameter(),
                            cache_change.key_hash_parameter(),
                        ) {
                            (Some(s), Some(k)) => &[s, k][..],
                            (Some(s), None) => &[s][..],
                            (None, Some(k)) => &[k][..],
                            (None, None) => &[],
                        };
                        let data_submessage = cache_change.as_data_submessage(
                            self.remote_reader_guid().entity_id(),
                            writer_id,
                            inline_qos,
                        );

                        let first_sn = seq_num_min
                            .unwrap_or(1)
                            .max(self.first_relevant_sample_seq_num());
                        let last_sn = seq_num_max.unwrap_or(0).max(first_sn - 1);
                        let heartbeat = self
                            .heartbeat_machine()
                            .generate_new_heartbeat(writer_id, first_sn, last_sn, now, false);

                        let len = RtpsMessageWrite::from_submessages(
                            message_writer.write_buffer_mut(),
                            &[&info_dst, &info_timestamp, &data_submessage, &heartbeat],
                            guid_prefix,
                        )
                        .buffer()
                        .len();
                        message_writer.write_message(len, self.unicast_locator_list())
                    }
                } else {
                    let info_dst =
                        InfoDestinationSubmessage::new(self.remote_reader_guid().prefix());

                    let gap_submessage = GapSubmessage::new(
                        ENTITYID_UNKNOWN,
                        writer_id,
                        next_unsent_change_seq_num,
                        SequenceNumberSet::new(next_unsent_change_seq_num + 1, []),
                    );

                    let len = RtpsMessageWrite::from_submessages(
                        message_writer.write_buffer_mut(),
                        &[&info_dst, &gap_submessage],
                        guid_prefix,
                    )
                    .buffer()
                    .len();
                    message_writer.write_message(len, self.unicast_locator_list())
                }

                self.set_highest_sent_seq_num(next_unsent_change_seq_num);
            }
        } else if !self.unacked_changes(seq_num_max) {
            // Idle
        } else if self
            .heartbeat_machine()
            .is_time_for_heartbeat(now, heartbeat_period.into())
        {
            let first_sn = seq_num_min
                .unwrap_or(1)
                .max(self.first_relevant_sample_seq_num());
            let last_sn = seq_num_max.unwrap_or(0).max(first_sn - 1);
            let heartbeat_submessage = self
                .heartbeat_machine()
                .generate_new_heartbeat(writer_id, first_sn, last_sn, now, false);

            let info_dst = InfoDestinationSubmessage::new(self.remote_reader_guid().prefix());

            let len = RtpsMessageWrite::from_submessages(
                message_writer.write_buffer_mut(),
                &[&info_dst, &heartbeat_submessage],
                guid_prefix,
            )
            .buffer()
            .len();
            message_writer.write_message(len, self.unicast_locator_list())
        }

        // Middle-part of the state-machine - Figure 8.19 RTPS standard
        if !self.requested_changes().is_empty() {
            while let Some(next_requested_change_seq_num) = self.next_requested_change() {
                // "a_change.status := UNDERWAY;" should be done by next_requested_change() as
                // it's not done here to avoid the change being a mutable reference
                // Also the post-condition:
                // a_change BELONGS-TO the_reader_proxy.requested_changes() ) == FALSE
                // should be full-filled by next_requested_change()
                if let Some(cache_change) = changes.iter().find(|cc| {
                    cc.sequence_number == next_requested_change_seq_num
                        && next_requested_change_seq_num >= self.first_relevant_sample_seq_num()
                }) {
                    let number_of_fragments = cache_change
                        .data_value
                        .len()
                        .div_ceil(data_max_size_serialized);

                    // Either send a DATAFRAG submessages or send a single DATA submessage
                    if number_of_fragments > 1 && cache_change.kind == ChangeKind::Alive {
                        for fragment_number in 0..number_of_fragments {
                            let reader_id = self.remote_reader_guid().entity_id();
                            let data_frag = cache_change.as_data_frag_submessage(
                                reader_id,
                                writer_id,
                                data_max_size_serialized,
                                fragment_number,
                            );

                            let info_dst =
                                InfoDestinationSubmessage::new(self.remote_reader_guid().prefix());
                            let info_timestamp =
                                if let Some(timestamp) = cache_change.source_timestamp {
                                    InfoTimestampSubmessage::new(false, timestamp.into())
                                } else {
                                    InfoTimestampSubmessage::new(true, TIME_INVALID)
                                };
                            let len = if fragment_number == number_of_fragments - 1 {
                                let first_sn = seq_num_min
                                    .unwrap_or(1)
                                    .max(self.first_relevant_sample_seq_num());
                                let last_sn = seq_num_max.unwrap_or(0).max(first_sn - 1);
                                let heartbeat = self.heartbeat_machine().generate_new_heartbeat(
                                    writer_id, first_sn, last_sn, now, false,
                                );

                                RtpsMessageWrite::from_submessages(
                                    message_writer.write_buffer_mut(),
                                    &[&info_dst, &info_timestamp, &data_frag, &heartbeat],
                                    guid_prefix,
                                )
                            } else {
                                RtpsMessageWrite::from_submessages(
                                    message_writer.write_buffer_mut(),
                                    &[&info_dst, &info_timestamp, &data_frag],
                                    guid_prefix,
                                )
                            }
                            .buffer()
                            .len();
                            message_writer.write_message(len, self.unicast_locator_list());
                        }
                    } else {
                        let info_dst =
                            InfoDestinationSubmessage::new(self.remote_reader_guid().prefix());

                        let info_timestamp = if let Some(timestamp) = cache_change.source_timestamp
                        {
                            InfoTimestampSubmessage::new(false, timestamp.into())
                        } else {
                            InfoTimestampSubmessage::new(true, TIME_INVALID)
                        };

                        let inline_qos = match (
                            cache_change.status_info_parameter(),
                            cache_change.key_hash_parameter(),
                        ) {
                            (Some(s), Some(k)) => &[s, k][..],
                            (Some(s), None) => &[s][..],
                            (None, Some(k)) => &[k][..],
                            (None, None) => &[],
                        };
                        let data_submessage = cache_change.as_data_submessage(
                            self.remote_reader_guid().entity_id(),
                            writer_id,
                            inline_qos,
                        );

                        let first_sn = seq_num_min
                            .unwrap_or(1)
                            .max(self.first_relevant_sample_seq_num());
                        let last_sn = seq_num_max.unwrap_or(0).max(first_sn - 1);
                        let heartbeat = self
                            .heartbeat_machine()
                            .generate_new_heartbeat(writer_id, first_sn, last_sn, now, false);

                        let len = RtpsMessageWrite::from_submessages(
                            message_writer.write_buffer_mut(),
                            &[&info_dst, &info_timestamp, &data_submessage, &heartbeat],
                            guid_prefix,
                        )
                        .buffer()
                        .len();
                        message_writer.write_message(len, self.unicast_locator_list());
                    }
                } else {
                    let info_dst =
                        InfoDestinationSubmessage::new(self.remote_reader_guid().prefix());

                    let gap_submessage = GapSubmessage::new(
                        ENTITYID_UNKNOWN,
                        writer_id,
                        next_requested_change_seq_num,
                        SequenceNumberSet::new(next_requested_change_seq_num + 1, []),
                    );

                    let len = RtpsMessageWrite::from_submessages(
                        message_writer.write_buffer_mut(),
                        &[&info_dst, &gap_submessage],
                        guid_prefix,
                    )
                    .buffer()
                    .len();
                    message_writer.write_message(len, self.unicast_locator_list());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::{
        infrastructure::time::Time,
        rtps_messages::{
            overall_structure::{RtpsMessageRead, RtpsSubmessageReadKind},
            submessage_elements::FragmentNumberSet,
        },
    };

    use super::*;

    #[test]
    fn test_all_fragments_sent() {
        struct MockWriter {
            total_fragments_sent: Mutex<usize>,
            buffer: [u8; 65535],
        }
        impl WriteMessage for MockWriter {
            fn write_buffer_mut(&mut self) -> &mut [u8] {
                &mut self.buffer
            }
            fn write_message(
                &mut self,
                len: usize,
                _locator_list: &[crate::transport::types::Locator],
            ) {
                let message = RtpsMessageRead::try_from(&self.buffer[..len]).unwrap();
                assert!(matches!(
                    message.submessages()[2],
                    RtpsSubmessageReadKind::DataFrag(_)
                ));
                *self.total_fragments_sent.lock().unwrap() += 1;
            }
        }

        let data_max_size_serialized = 500;
        let guid = Guid::new([1; 12], EntityId::new([1; 3], 1));
        let mut writer = RtpsStatefulWriter::new(guid, data_max_size_serialized);

        let remote_reader_guid = Guid::new([2; 12], EntityId::new([2; 3], 2));
        writer.add_matched_reader(ReaderProxy {
            remote_reader_guid,
            remote_group_entity_id: ENTITYID_UNKNOWN,
            reliability_kind: ReliabilityKind::Reliable,
            durability_kind: DurabilityKind::Volatile,
            unicast_locator_list: vec![],
            multicast_locator_list: vec![],
            expects_inline_qos: false,
        });

        let mut message_writer = MockWriter {
            total_fragments_sent: Mutex::new(0),
            buffer: [0; 65535],
        };
        writer.add_change(CacheChange {
            kind: ChangeKind::Alive,
            writer_guid: guid,
            sequence_number: 1,
            source_timestamp: None,
            instance_handle: Some([10; 16]),
            data_value: vec![8; 1300].into(),
        });
        writer.write_message(&mut message_writer, Time::new(1, 0));
        assert_eq!(*message_writer.total_fragments_sent.lock().unwrap(), 3);
    }

    #[test]
    fn test_single_fragment_sent_after_acknack_frag() {
        struct MockWriter {
            total_fragments_sent: Mutex<usize>,
            buffer: [u8; 65535],
        }
        impl WriteMessage for MockWriter {
            fn write_buffer_mut(&mut self) -> &mut [u8] {
                &mut self.buffer
            }
            fn write_message(
                &mut self,
                len: usize,
                _locator_list: &[crate::transport::types::Locator],
            ) {
                let message = RtpsMessageRead::try_from(&self.buffer[..len]).unwrap();
                assert!(matches!(
                    message.submessages()[2],
                    RtpsSubmessageReadKind::DataFrag(_)
                ));
                *self.total_fragments_sent.lock().unwrap() += 1;
            }
        }

        let data_max_size_serialized = 500;
        let writer_id = EntityId::new([1; 3], 1);
        let guid = Guid::new([1; 12], writer_id);
        let mut writer = RtpsStatefulWriter::new(guid, data_max_size_serialized);

        let remote_reader_id = EntityId::new([2; 3], 2);
        let remote_reader_guid_prefix = [2; 12];
        let remote_reader_guid = Guid::new(remote_reader_guid_prefix, remote_reader_id);
        writer.add_matched_reader(ReaderProxy {
            remote_reader_guid,
            remote_group_entity_id: ENTITYID_UNKNOWN,
            reliability_kind: ReliabilityKind::Reliable,
            durability_kind: DurabilityKind::Volatile,
            unicast_locator_list: vec![],
            multicast_locator_list: vec![],
            expects_inline_qos: false,
        });
        let mut message_writer = MockWriter {
            total_fragments_sent: Mutex::new(0),
            buffer: [0; 65535],
        };
        writer.add_change(CacheChange {
            kind: ChangeKind::Alive,
            writer_guid: guid,
            sequence_number: 1,
            source_timestamp: None,
            instance_handle: Some([10; 16]),
            data_value: vec![8; 1300].into(),
        });
        writer.write_message(&mut message_writer, Time::new(1, 0));

        let nackfrag_submessage = NackFragSubmessage::new(
            remote_reader_id,
            writer_id,
            1,
            FragmentNumberSet::new(1, []),
            1,
        );
        let mut message_writer = MockWriter {
            total_fragments_sent: Mutex::new(0),
            buffer: [0; 65535],
        };
        writer.on_nack_frag_submessage_received(
            &nackfrag_submessage,
            remote_reader_guid_prefix,
            &mut message_writer,
        );

        assert_eq!(*message_writer.total_fragments_sent.lock().unwrap(), 1);
    }

    #[test]
    fn test_best_effort_reader_no_gap_submessage_on_non_contiguous_sequence_numbers() {
        struct MockWriter {
            gap_count: Mutex<usize>,
            data_count: Mutex<usize>,
            buffer: [u8; 65535],
        }
        impl WriteMessage for MockWriter {
            fn write_buffer_mut(&mut self) -> &mut [u8] {
                &mut self.buffer
            }
            fn write_message(
                &mut self,
                len: usize,
                _locator_list: &[crate::transport::types::Locator],
            ) {
                let message = RtpsMessageRead::try_from(&self.buffer[..len]).unwrap();
                for submessage in message.submessages() {
                    match submessage {
                        RtpsSubmessageReadKind::Gap(_) => *self.gap_count.lock().unwrap() += 1,
                        RtpsSubmessageReadKind::Data(_) => *self.data_count.lock().unwrap() += 1,
                        _ => (),
                    }
                }
            }
        }

        let data_max_size_serialized = 500;
        let writer_id = EntityId::new([1; 3], 1);
        let guid = Guid::new([1; 12], writer_id);
        let mut writer = RtpsStatefulWriter::new(guid, data_max_size_serialized);

        let remote_reader_id = EntityId::new([2; 3], 2);
        let remote_reader_guid = Guid::new([2; 12], remote_reader_id);
        writer.add_matched_reader(ReaderProxy {
            remote_reader_guid,
            remote_group_entity_id: ENTITYID_UNKNOWN,
            reliability_kind: ReliabilityKind::BestEffort,
            durability_kind: DurabilityKind::Volatile,
            unicast_locator_list: vec![],
            multicast_locator_list: vec![],
            expects_inline_qos: false,
        });

        // Add non-contiguous sequence numbers (e.g. 2 and 5)
        writer.add_change(CacheChange {
            kind: ChangeKind::Alive,
            writer_guid: guid,
            sequence_number: 2,
            source_timestamp: None,
            instance_handle: Some([10; 16]),
            data_value: vec![1, 2, 3].into(),
        });
        writer.add_change(CacheChange {
            kind: ChangeKind::Alive,
            writer_guid: guid,
            sequence_number: 5,
            source_timestamp: None,
            instance_handle: Some([10; 16]),
            data_value: vec![4, 5, 6].into(),
        });

        let mut message_writer = MockWriter {
            gap_count: Mutex::new(0),
            data_count: Mutex::new(0),
            buffer: [0; 65535],
        };

        writer.write_message(&mut message_writer, Time::new(1, 0));

        assert_eq!(
            *message_writer.gap_count.lock().unwrap(),
            0,
            "Best-effort reader proxy should not receive any GAP submessages"
        );
        assert_eq!(*message_writer.data_count.lock().unwrap(), 2);
    }

    #[test]
    fn test_reliable_reader_receives_heartbeat_first_for_historical_data() {
        struct MockWriter {
            heartbeat_count: Mutex<usize>,
            data_count: Mutex<usize>,
            buffer: [u8; 65535],
        }
        impl WriteMessage for MockWriter {
            fn write_buffer_mut(&mut self) -> &mut [u8] {
                &mut self.buffer
            }
            fn write_message(
                &mut self,
                len: usize,
                _locator_list: &[crate::transport::types::Locator],
            ) {
                let message = RtpsMessageRead::try_from(&self.buffer[..len]).unwrap();
                for submessage in message.submessages() {
                    match submessage {
                        RtpsSubmessageReadKind::Heartbeat(_) => {
                            *self.heartbeat_count.lock().unwrap() += 1
                        }
                        RtpsSubmessageReadKind::Data(_) => *self.data_count.lock().unwrap() += 1,
                        _ => (),
                    }
                }
            }
        }

        let data_max_size_serialized = 500;
        let writer_id = EntityId::new([1; 3], 1);
        let guid = Guid::new([1; 12], writer_id);
        let mut writer = RtpsStatefulWriter::new(guid, data_max_size_serialized);

        // Add historical data before reader matches
        writer.add_change(CacheChange {
            kind: ChangeKind::Alive,
            writer_guid: guid,
            sequence_number: 1,
            source_timestamp: None,
            instance_handle: Some([10; 16]),
            data_value: vec![1, 2, 3].into(),
        });

        // Now match a Reliable reader
        let remote_reader_id = EntityId::new([2; 3], 2);
        let remote_reader_guid_prefix = [2; 12];
        let remote_reader_guid = Guid::new(remote_reader_guid_prefix, remote_reader_id);
        writer.add_matched_reader(ReaderProxy {
            remote_reader_guid,
            remote_group_entity_id: ENTITYID_UNKNOWN,
            reliability_kind: ReliabilityKind::Reliable,
            durability_kind: DurabilityKind::TransientLocal,
            unicast_locator_list: vec![],
            multicast_locator_list: vec![],
            expects_inline_qos: false,
        });

        let mut message_writer = MockWriter {
            heartbeat_count: Mutex::new(0),
            data_count: Mutex::new(0),
            buffer: [0; 65535],
        };

        // write_message should send HEARTBEAT, not DATA
        writer.write_message(&mut message_writer, Time::new(1, 0));

        assert_eq!(*message_writer.heartbeat_count.lock().unwrap(), 1);
        assert_eq!(*message_writer.data_count.lock().unwrap(), 0);

        // Simulate receiving an AckNack requesting sequence number 1
        let acknack_submessage = AckNackSubmessage::new(
            true,
            remote_reader_id,
            writer_id,
            SequenceNumberSet::new(1, [1]),
            1,
        );
        let mut ack_response_writer = MockWriter {
            heartbeat_count: Mutex::new(0),
            data_count: Mutex::new(0),
            buffer: [0; 65535],
        };
        writer.on_acknack_submessage_received(
            &acknack_submessage,
            remote_reader_guid_prefix,
            &mut ack_response_writer,
            Time::new(1, 0),
        );

        // Now DATA should have been sent in response to the AckNack
        assert_eq!(*ack_response_writer.data_count.lock().unwrap(), 1);
    }

    #[test]
    fn test_best_effort_reader_receives_historical_data_immediately() {
        struct MockWriter {
            data_count: Mutex<usize>,
            buffer: [u8; 65535],
        }
        impl WriteMessage for MockWriter {
            fn write_buffer_mut(&mut self) -> &mut [u8] {
                &mut self.buffer
            }
            fn write_message(
                &mut self,
                len: usize,
                _locator_list: &[crate::transport::types::Locator],
            ) {
                let message = RtpsMessageRead::try_from(&self.buffer[..len]).unwrap();
                for submessage in message.submessages() {
                    if let RtpsSubmessageReadKind::Data(_) = submessage {
                        *self.data_count.lock().unwrap() += 1;
                    }
                }
            }
        }

        let data_max_size_serialized = 500;
        let writer_id = EntityId::new([1; 3], 1);
        let guid = Guid::new([1; 12], writer_id);
        let mut writer = RtpsStatefulWriter::new(guid, data_max_size_serialized);

        // Add historical data before reader matches
        writer.add_change(CacheChange {
            kind: ChangeKind::Alive,
            writer_guid: guid,
            sequence_number: 1,
            source_timestamp: None,
            instance_handle: Some([10; 16]),
            data_value: vec![1, 2, 3].into(),
        });

        // Match a BestEffort reader
        let remote_reader_id = EntityId::new([2; 3], 2);
        let remote_reader_guid = Guid::new([2; 12], remote_reader_id);
        writer.add_matched_reader(ReaderProxy {
            remote_reader_guid,
            remote_group_entity_id: ENTITYID_UNKNOWN,
            reliability_kind: ReliabilityKind::BestEffort,
            durability_kind: DurabilityKind::TransientLocal,
            unicast_locator_list: vec![],
            multicast_locator_list: vec![],
            expects_inline_qos: false,
        });

        let mut message_writer = MockWriter {
            data_count: Mutex::new(0),
            buffer: [0; 65535],
        };

        // write_message should send DATA immediately for BestEffort
        writer.write_message(&mut message_writer, Time::new(1, 0));

        assert_eq!(*message_writer.data_count.lock().unwrap(), 1);
    }
}
