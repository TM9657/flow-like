use crate::{
    rtps_messages::{
        overall_structure::RtpsMessageWrite,
        submessage_elements::{Data, FragmentNumberSet, SequenceNumberSet},
        submessages::{
            ack_nack::AckNackSubmessage, data::DataSubmessage, data_frag::DataFragSubmessage,
            info_destination::InfoDestinationSubmessage, nack_frag::NackFragSubmessage,
        },
        types::Count,
    },
    transport::{
        interface::WriteMessage,
        types::{EntityId, Guid, Locator, ReliabilityKind, SequenceNumber},
    },
};
use alloc::{sync::Arc, vec::Vec};

use core::cmp::max;

fn total_fragments_expected(data_frag_submessage: &DataFragSubmessage) -> u32 {
    let data_size = data_frag_submessage.data_size();
    let fragment_size = data_frag_submessage.fragment_size() as u32;
    let total_fragments_correction = if data_size.is_multiple_of(fragment_size) {
        0
    } else {
        1
    };
    data_size / fragment_size + total_fragments_correction
}

#[derive(Debug, PartialEq, Eq)]
pub struct RtpsWriterProxy {
    remote_writer_guid: Guid,
    unicast_locator_list: Vec<Locator>,
    multicast_locator_list: Vec<Locator>,
    remote_group_entity_id: EntityId,
    first_available_seq_num: SequenceNumber,
    last_available_seq_num: SequenceNumber,
    highest_received_change_sn: SequenceNumber,
    must_send_acknacks: bool,
    last_received_heartbeat_count: Count,
    last_received_heartbeat_frag_count: Count,
    acknack_count: Count,
    nack_frag_count: Count,
    frag_buffer: Vec<DataFragSubmessage>,
    reliability: ReliabilityKind,
}

impl RtpsWriterProxy {
    pub fn new(
        remote_writer_guid: Guid,
        unicast_locator_list: &[Locator],
        multicast_locator_list: &[Locator],
        remote_group_entity_id: EntityId,
        reliability: ReliabilityKind,
    ) -> Self {
        Self {
            remote_writer_guid,
            unicast_locator_list: unicast_locator_list.to_vec(),
            multicast_locator_list: multicast_locator_list.to_vec(),
            remote_group_entity_id,
            first_available_seq_num: 1,
            last_available_seq_num: 0,
            highest_received_change_sn: 0,
            must_send_acknacks: false,
            last_received_heartbeat_count: 0,
            last_received_heartbeat_frag_count: 0,
            acknack_count: 0,
            nack_frag_count: 0,
            frag_buffer: Vec::new(),
            reliability,
        }
    }

    pub fn push_data_frag(&mut self, submessage: DataFragSubmessage) {
        if !self.frag_buffer.iter().any(|f| {
            f.writer_sn() == submessage.writer_sn()
                && f.fragment_starting_num() == submessage.fragment_starting_num()
        }) {
            self.frag_buffer.push(submessage);
        }
    }

    pub fn delete_data_fragments(&mut self, sequence_number: SequenceNumber) {
        self.frag_buffer.retain(|f| f.writer_sn() > sequence_number);
    }

    pub fn reconstruct_data_from_frag(
        &mut self,
        seq_num: SequenceNumber,
    ) -> Option<DataSubmessage> {
        let frag_submessage = self.frag_buffer.iter().find(|f| f.writer_sn() == seq_num)?;
        let total_fragments_expected = total_fragments_expected(frag_submessage);

        let total_fragments: u32 = self
            .frag_buffer
            .iter()
            .filter(|f| f.writer_sn() == seq_num)
            .map(|f| f.fragments_in_submessage() as u32)
            .sum();

        if total_fragments == total_fragments_expected {
            let mut matching_frags: Vec<&DataFragSubmessage> = self
                .frag_buffer
                .iter()
                .filter(|f| f.writer_sn() == seq_num)
                .collect();
            matching_frags.sort_unstable_by_key(|f| f.fragment_starting_num());

            let mut data = Vec::with_capacity(frag_submessage.data_size() as usize);
            for frag in &matching_frags {
                data.extend_from_slice(frag.serialized_payload().as_ref());
            }

            let first_frag = matching_frags.first().copied()?;

            let inline_qos_flag = first_frag.inline_qos_flag();
            let data_flag = !first_frag.key_flag();
            let key_flag = first_frag.key_flag();
            let non_standard_payload_flag = false;
            let writer_id = self.remote_writer_guid.entity_id();
            let reader_id = first_frag.reader_id();
            let writer_sn = seq_num;
            let inline_qos = first_frag.inline_qos().clone();

            self.frag_buffer.retain(|f| f.writer_sn() != seq_num);

            Some(DataSubmessage::new(
                inline_qos_flag,
                data_flag,
                key_flag,
                non_standard_payload_flag,
                reader_id,
                writer_id,
                writer_sn,
                inline_qos,
                Data::new(Arc::from(data)),
            ))
        } else {
            None
        }
    }

    pub fn remote_writer_guid(&self) -> Guid {
        self.remote_writer_guid
    }

    pub fn unicast_locator_list(&self) -> &[Locator] {
        self.unicast_locator_list.as_ref()
    }

    pub fn reliability(&self) -> ReliabilityKind {
        self.reliability
    }

    pub fn available_changes_max(&self) -> SequenceNumber {
        // The condition to make any CacheChange 'a_change' available for 'access' by the DDS DataReader is that there are no changes
        // from the RTPS Writer with SequenceNumber_t smaller than or equal to a_change.sequenceNumber that have status MISSING or UNKNOWN.

        max(
            self.first_available_seq_num - 1,
            self.highest_received_change_sn,
        )
    }

    pub fn irrelevant_change_set(&mut self, a_seq_num: SequenceNumber) {
        // This operation modifies the status of a ChangeFromWriter to indicate that the CacheChange with the
        // SequenceNumber_t 'a_seq_num' is irrelevant to the RTPS Reader. Logical action in the virtual machine:
        // FIND change FROM this.changes_from_writer SUCH-THAT
        // (change.sequenceNumber == a_seq_num);
        // change.status := RECEIVED; change.is_relevant := FALSE;
        if a_seq_num > self.highest_received_change_sn {
            self.highest_received_change_sn = a_seq_num;
        }
    }

    pub fn lost_changes_update(&mut self, first_available_seq_num: SequenceNumber) {
        // FOREACH change IN this.changes_from_writer
        // SUCH-THAT ( change.status == UNKNOWN OR change.status == MISSING
        // AND seq_num < first_available_seq_num ) DO {
        // change.status := LOST;
        // }
        self.first_available_seq_num = first_available_seq_num;
    }

    pub fn missing_changes(&self) -> impl Iterator<Item = SequenceNumber> {
        // The changes with status 'MISSING' represent the set of changes available in the HistoryCache of the RTPS Writer
        // represented by the RTPS WriterProxy that have not been received by the RTPS Reader.
        // return { change IN this.changes_from_writer SUCH-THAT change.status == MISSING};

        // The highest sequence number of all present
        let highest_number = max(self.last_available_seq_num, self.highest_received_change_sn);

        // Changes below first_available_seq_num are LOST (or RECEIVED, but in any case not MISSING) and above last_available_seq_num are unknown.
        // In between those two numbers, every change that is not RECEIVED or IRRELEVANT is MISSING
        let first_missing_change = max(
            self.first_available_seq_num,
            self.highest_received_change_sn + 1,
        );
        first_missing_change..=highest_number
    }

    pub fn missing_changes_update(&mut self, last_available_seq_num: SequenceNumber) {
        // FOREACH change IN this.changes_from_writer
        // SUCH-THAT ( change.status == UNKNOWN
        // AND seq_num <= last_available_seq_num ) DO {
        // change.status := MISSING;
        // }
        self.last_available_seq_num = last_available_seq_num;
    }

    pub fn received_change_set(&mut self, a_seq_num: SequenceNumber) {
        // FIND change FROM this.changes_from_writer
        //     SUCH-THAT change.sequenceNumber == a_seq_num;
        // change.status := RECEIVED
        if a_seq_num > self.highest_received_change_sn {
            self.highest_received_change_sn = a_seq_num;
        }

        // Make sure all the fragments that are older than the received sample are deleted
        // since they are not useful anymore
        self.frag_buffer.retain(|x| x.writer_sn() > a_seq_num);
    }

    pub fn set_must_send_acknacks(&mut self, must_send_acknacks: bool) {
        self.must_send_acknacks = must_send_acknacks;
    }

    pub fn must_send_acknacks(&self) -> bool {
        self.must_send_acknacks
    }

    pub fn last_received_heartbeat_count(&self) -> Count {
        self.last_received_heartbeat_count
    }

    pub fn set_last_received_heartbeat_count(&mut self, last_received_heartbeat_count: Count) {
        self.last_received_heartbeat_count = last_received_heartbeat_count;
    }

    pub fn last_received_heartbeat_frag_count(&self) -> Count {
        self.last_received_heartbeat_frag_count
    }

    pub fn set_last_received_heartbeat_frag_count(
        &mut self,
        last_received_heartbeat_frag_count: Count,
    ) {
        self.last_received_heartbeat_frag_count = last_received_heartbeat_frag_count;
    }

    pub fn acknack_count(&self) -> Count {
        self.acknack_count
    }

    pub fn increment_acknack_count(&mut self) {
        self.acknack_count = self.acknack_count.wrapping_add(1);
    }

    pub fn increment_nack_frag_count(&mut self) {
        self.nack_frag_count = self.nack_frag_count.wrapping_add(1);
    }

    pub fn write_message(
        &mut self,
        reader_guid: &Guid,
        message_writer: &mut (impl WriteMessage + ?Sized),
    ) {
        if self.must_send_acknacks() || !self.missing_changes().count() == 0 {
            self.set_must_send_acknacks(false);
            self.increment_acknack_count();

            let info_dst_submessage =
                InfoDestinationSubmessage::new(self.remote_writer_guid().prefix());

            // We report missing changes excluding those where we have received at least one fragment
            let missing_changes = self
                .missing_changes()
                .take(256)
                .filter(|x| !self.frag_buffer.iter().any(|f| &f.writer_sn() == x));
            let acknack_submessage = AckNackSubmessage::new(
                true,
                reader_guid.entity_id(),
                self.remote_writer_guid().entity_id(),
                SequenceNumberSet::new(self.available_changes_max() + 1, missing_changes),
                self.acknack_count(),
            );

            let missing_change_fragments_seq_num = self
                .missing_changes()
                .take(256)
                .find(|s| self.frag_buffer.iter().any(|x| &x.writer_sn() == s));

            let len = if let Some(missing_change_fragments_seq_num) =
                missing_change_fragments_seq_num
            {
                let frag = self
                    .frag_buffer
                    .iter()
                    .find(|x| x.writer_sn() == missing_change_fragments_seq_num)
                    .expect("Must exist");
                let total_fragments_expected =
                    frag.data_size().div_ceil(frag.fragment_size() as u32);
                let mut missing_fragments_iter = (1..=total_fragments_expected)
                    .filter(|frag_num| {
                        !self.frag_buffer.iter().any(|f| {
                            f.writer_sn() == missing_change_fragments_seq_num
                                && &f.fragment_starting_num() == frag_num
                        })
                    })
                    .peekable();

                let base = *missing_fragments_iter
                    .peek()
                    .expect("At least a fragment must be missing");
                let fragment_number_state = FragmentNumberSet::new(base, missing_fragments_iter);
                self.increment_nack_frag_count();
                let nack_frag_submessage = NackFragSubmessage::new(
                    reader_guid.entity_id(),
                    self.remote_writer_guid().entity_id(),
                    missing_change_fragments_seq_num,
                    fragment_number_state,
                    self.nack_frag_count,
                );

                RtpsMessageWrite::from_submessages(
                    message_writer.write_buffer_mut(),
                    &[
                        &info_dst_submessage,
                        &acknack_submessage,
                        &nack_frag_submessage,
                    ],
                    reader_guid.prefix(),
                )
                .buffer()
                .len()
            } else {
                RtpsMessageWrite::from_submessages(
                    message_writer.write_buffer_mut(),
                    &[&info_dst_submessage, &acknack_submessage],
                    reader_guid.prefix(),
                )
                .buffer()
                .len()
            };

            message_writer.write_message(len, self.unicast_locator_list());
        }
    }

    pub fn is_historical_data_received(&self) -> bool {
        let at_least_one_heartbeat_received = self.last_received_heartbeat_count > 0;
        at_least_one_heartbeat_received && self.missing_changes().count() == 0
    }
}
