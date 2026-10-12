use crate::{
    infrastructure::time::{Duration, Time},
    rtps_messages::{
        submessages::{heartbeat::HeartbeatSubmessage, heartbeat_frag::HeartbeatFragSubmessage},
        types::{Count, FragmentNumber},
    },
    transport::types::{
        CacheChange, DurabilityKind, EntityId, Guid, Locator, ReliabilityKind, SequenceNumber,
    },
};
use alloc::vec::Vec;

#[derive(Debug, PartialEq, Eq)]
pub struct HeartbeatMachine {
    count: Count,
    reader_id: EntityId,
    last_heartbeat_time: Time,
}
impl HeartbeatMachine {
    fn new(reader_id: EntityId) -> Self {
        HeartbeatMachine {
            count: 0,
            reader_id,
            last_heartbeat_time: Time::new(0, 0),
        }
    }
    pub fn is_time_for_heartbeat(&self, now: Time, heartbeat_period: Duration) -> bool {
        now - self.last_heartbeat_time >= heartbeat_period
    }

    pub fn reset_heartbeat_time(&mut self) {
        self.last_heartbeat_time = Time::new(0, 0);
    }

    pub fn time_until_heartbeat(&self, now: Time, heartbeat_period: Duration) -> Duration {
        let elapsed = now - self.last_heartbeat_time;
        if elapsed >= heartbeat_period {
            Duration::new(0, 0)
        } else {
            heartbeat_period - elapsed
        }
    }

    pub fn generate_new_heartbeat(
        &mut self,
        writer_id: EntityId,
        first_sn: SequenceNumber,
        last_sn: SequenceNumber,
        heartbeat_time: Time,
        final_flag: bool,
    ) -> HeartbeatSubmessage {
        self.count = self.count.wrapping_add(1);
        self.last_heartbeat_time = heartbeat_time;
        HeartbeatSubmessage::new(
            final_flag,
            false,
            self.reader_id,
            writer_id,
            first_sn,
            last_sn,
            self.count,
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct HeartbeatFragMachine {
    count: Count,
    reader_id: EntityId,
}

impl HeartbeatFragMachine {
    fn new(reader_id: EntityId) -> Self {
        HeartbeatFragMachine {
            count: 0,
            reader_id,
        }
    }
    pub fn _submessage(
        &mut self,
        writer_id: EntityId,
        writer_sn: SequenceNumber,
        last_fragment_num: FragmentNumber,
    ) -> HeartbeatFragSubmessage {
        self.count = self.count.wrapping_add(1);
        HeartbeatFragSubmessage::_new(
            self.reader_id,
            writer_id,
            writer_sn,
            last_fragment_num,
            self.count,
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RtpsReaderProxy {
    remote_reader_guid: Guid,
    remote_group_entity_id: EntityId,
    unicast_locator_list: Vec<Locator>,
    multicast_locator_list: Vec<Locator>,
    highest_sent_seq_num: SequenceNumber,
    highest_acked_seq_num: SequenceNumber,
    requested_changes: Vec<SequenceNumber>,
    expects_inline_qos: bool,
    is_active: bool,
    last_received_acknack_count: Count,
    last_received_nack_frag_count: Count,
    heartbeat_machine: HeartbeatMachine,
    heartbeat_frag_machine: HeartbeatFragMachine,
    reliability: ReliabilityKind,
    first_relevant_sample_seq_num: SequenceNumber,
    durability: DurabilityKind,
}

impl RtpsReaderProxy {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        remote_reader_guid: Guid,
        remote_group_entity_id: EntityId,
        unicast_locator_list: &[Locator],
        multicast_locator_list: &[Locator],
        expects_inline_qos: bool,
        is_active: bool,
        reliability: ReliabilityKind,
        first_relevant_sample_seq_num: SequenceNumber,
        durability: DurabilityKind,
        highest_sent_seq_num: SequenceNumber,
    ) -> Self {
        let heartbeat_machine = HeartbeatMachine::new(remote_reader_guid.entity_id());
        let heartbeat_frag_machine = HeartbeatFragMachine::new(remote_reader_guid.entity_id());
        Self {
            remote_reader_guid,
            remote_group_entity_id,
            unicast_locator_list: unicast_locator_list.to_vec(),
            multicast_locator_list: multicast_locator_list.to_vec(),
            highest_sent_seq_num,
            highest_acked_seq_num: 0,
            requested_changes: Vec::new(),
            expects_inline_qos,
            is_active,
            last_received_acknack_count: 0,
            last_received_nack_frag_count: 0,
            heartbeat_machine,
            heartbeat_frag_machine,
            reliability,
            first_relevant_sample_seq_num,
            durability,
        }
    }

    pub fn remote_reader_guid(&self) -> Guid {
        self.remote_reader_guid
    }

    pub fn unicast_locator_list(&self) -> &[Locator] {
        self.unicast_locator_list.as_slice()
    }

    pub fn reliability(&self) -> ReliabilityKind {
        self.reliability
    }

    pub fn heartbeat_machine(&mut self) -> &mut HeartbeatMachine {
        &mut self.heartbeat_machine
    }

    pub fn _heartbeat_frag_machine(&mut self) -> &mut HeartbeatFragMachine {
        &mut self.heartbeat_frag_machine
    }

    pub fn durability(&self) -> DurabilityKind {
        self.durability
    }

    // //////////////   ReaderProxy operations defined in the Rtps Standard

    pub fn acked_changes_set(&mut self, committed_seq_num: SequenceNumber) {
        if committed_seq_num > self.highest_acked_seq_num {
            self.highest_acked_seq_num = committed_seq_num
        }
    }

    pub fn next_requested_change(&mut self) -> Option<SequenceNumber> {
        let next_requested_change = self.requested_changes.iter().min().cloned();

        if let Some(next_sn) = &next_requested_change {
            self.requested_changes.retain(|sn| sn != next_sn);
        }

        next_requested_change
    }

    pub fn next_unsent_change(
        &self,
        writer_history_cache: &[CacheChange],
    ) -> Option<SequenceNumber> {
        //         unsent_changes :=
        // { changes SUCH_THAT change.sequenceNumber > this.highestSentChangeSN }
        //
        // IF unsent_changes == <empty> return SEQUENCE_NUMBER_INVALID
        // ELSE return MIN { unsent_changes.sequenceNumber }

        // Iterate from the end since the last sequence number is like
        writer_history_cache
            .iter()
            .rev()
            .map(|cc| cc.sequence_number)
            .filter(|cc_sn| cc_sn > &self.highest_sent_seq_num)
            .filter(|cc_sn| cc_sn >= &self.first_relevant_sample_seq_num)
            .min()
    }

    pub fn unsent_changes(&self, writer_history_cache: &[CacheChange]) -> bool {
        // return this.next_unsent_change() != SEQUENCE_NUMBER_INVALID;
        self.next_unsent_change(writer_history_cache).is_some()
    }

    pub fn requested_changes(&self) -> &[SequenceNumber] {
        &self.requested_changes
    }

    pub fn requested_changes_set(&mut self, req_seq_num_set: impl Iterator<Item = SequenceNumber>) {
        // "FOR_EACH seq_num IN req_seq_num_set DO
        //     FIND change_for_reader IN this.changes_for_reader
        //          SUCH-THAT (change_for_reader.sequenceNumber==seq_num)
        //     change_for_reader.status := REQUESTED;
        // END"
        for seq_num in req_seq_num_set {
            if !self.requested_changes.contains(&seq_num) {
                self.requested_changes.push(seq_num);
            }
        }
    }

    pub fn unacked_changes(&self, highest_available_seq_num: Option<SequenceNumber>) -> bool {
        // highest_available_seq_num := MAX { change.sequenceNumber }
        // highest_acked_seq_num := MAX { this.acknowledged_changes }
        // return ( highest_available_seq_num > highest_acked_seq_num )

        match highest_available_seq_num {
            Some(highest_available_seq_num) => {
                highest_available_seq_num > self.highest_acked_seq_num
            }
            None => false,
        }
    }

    pub fn time_until_heartbeat(
        &self,
        now: Time,
        heartbeat_period: Duration,
        highest_available_seq_num: Option<SequenceNumber>,
    ) -> Option<Duration> {
        if self.reliability == ReliabilityKind::Reliable
            && self.unacked_changes(highest_available_seq_num)
        {
            Some(
                self.heartbeat_machine
                    .time_until_heartbeat(now, heartbeat_period),
            )
        } else {
            None
        }
    }

    pub fn highest_acked_seq_num(&self) -> SequenceNumber {
        self.highest_acked_seq_num
    }

    pub fn highest_sent_seq_num(&self) -> SequenceNumber {
        self.highest_sent_seq_num
    }

    pub fn set_highest_sent_seq_num(&mut self, seq_num: SequenceNumber) {
        if seq_num > self.highest_sent_seq_num {
            self.highest_sent_seq_num = seq_num;
        }
    }

    pub fn first_relevant_sample_seq_num(&self) -> SequenceNumber {
        self.first_relevant_sample_seq_num
    }

    pub fn last_received_acknack_count(&self) -> Count {
        self.last_received_acknack_count
    }

    pub fn set_last_received_acknack_count(&mut self, count: Count) {
        self.last_received_acknack_count = count;
    }

    pub fn last_received_nack_frag_count(&self) -> Count {
        self.last_received_nack_frag_count
    }

    pub fn set_last_received_nack_frag_count(&mut self, count: Count) {
        self.last_received_nack_frag_count = count;
    }
}
