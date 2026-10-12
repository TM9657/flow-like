use crate::transport::types::{EntityId, SequenceNumber};

use super::super::{
    error::RtpsMessageResult,
    overall_structure::{
        Submessage, SubmessageHeaderRead, SubmessageHeaderWrite, TryReadFromBytes, Write,
        WriteIntoBytes,
    },
    submessage_elements::FragmentNumberSet,
    types::{Count, SubmessageKind},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NackFragSubmessage {
    reader_id: EntityId,
    writer_id: EntityId,
    writer_sn: SequenceNumber,
    fragment_number_state: FragmentNumberSet,
    count: Count,
}

impl NackFragSubmessage {
    pub fn try_from_bytes(
        submessage_header: &SubmessageHeaderRead,
        mut data: &[u8],
    ) -> RtpsMessageResult<Self> {
        let endianness = submessage_header.endianness();
        Ok(Self {
            reader_id: EntityId::try_read_from_bytes(&mut data, endianness)?,
            writer_id: EntityId::try_read_from_bytes(&mut data, endianness)?,
            writer_sn: SequenceNumber::try_read_from_bytes(&mut data, endianness)?,
            fragment_number_state: FragmentNumberSet::try_read_from_bytes(&mut data, endianness)?,
            count: Count::try_read_from_bytes(&mut data, endianness)?,
        })
    }

    pub fn reader_id(&self) -> EntityId {
        self.reader_id
    }

    pub fn _writer_id(&self) -> EntityId {
        self.writer_id
    }

    pub fn writer_sn(&self) -> SequenceNumber {
        self.writer_sn
    }

    pub fn fragment_number_state(&self) -> &FragmentNumberSet {
        &self.fragment_number_state
    }

    pub fn count(&self) -> Count {
        self.count
    }
}

impl NackFragSubmessage {
    pub fn new(
        reader_id: EntityId,
        writer_id: EntityId,
        writer_sn: SequenceNumber,
        fragment_number_state: FragmentNumberSet,
        count: Count,
    ) -> Self {
        Self {
            reader_id,
            writer_id,
            writer_sn,
            fragment_number_state,
            count,
        }
    }
}

impl Submessage for NackFragSubmessage {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(SubmessageKind::NACK_FRAG, &[], octets_to_next_header)
            .write_into_bytes(buf);
    }

    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write) {
        self.reader_id.write_into_bytes(buf);
        self.writer_id.write_into_bytes(buf);
        self.writer_sn.write_into_bytes(buf);
        self.fragment_number_state.write_into_bytes(buf);
        self.count.write_into_bytes(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rtps_messages::overall_structure::write_submessage_into_bytes_vec,
        transport::types::{USER_DEFINED_READER_GROUP, USER_DEFINED_READER_NO_KEY},
    };

    #[test]
    fn serialize_nack_frag() {
        let submessage = NackFragSubmessage::new(
            EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY),
            EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP),
            4,
            FragmentNumberSet::new(10, []),
            6,
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x12_u8, 0b_0000_0001, 28, 0, // Submessage header
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN
                4, 0, 0, 0, // writerSN
               10, 0, 0, 0, // fragmentNumberState.base
                0, 0, 0, 0, // fragmentNumberState.numBits
                6, 0, 0, 0, // count
            ]
        );
    }

    #[test]
    fn deserialize_nack_frag() {
        #[rustfmt::skip]
        let mut data = &[
            0x12_u8, 0b_0000_0001, 28, 0, // Submessage header
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN
            4, 0, 0, 0, // writerSN
           10, 0, 0, 0, // fragmentNumberState.base
            0, 0, 0, 0, // fragmentNumberState.numBits
            6, 0, 0, 0, // count
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let submessage = NackFragSubmessage::try_from_bytes(&submessage_header, data).unwrap();

        let expected_reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let expected_writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let expected_writer_sn = 4;
        let expected_fragment_number_state = FragmentNumberSet::new(10, []);
        let expected_count = 6;

        assert_eq!(expected_reader_id, submessage.reader_id());
        assert_eq!(expected_writer_id, submessage._writer_id());
        assert_eq!(expected_writer_sn, submessage.writer_sn());
        assert_eq!(
            &expected_fragment_number_state,
            submessage.fragment_number_state()
        );
        assert_eq!(expected_count, submessage.count());
    }
}
