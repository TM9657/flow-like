use crate::transport::types::EntityId;

use super::super::{
    error::RtpsMessageResult,
    overall_structure::{
        Submessage, SubmessageHeaderRead, SubmessageHeaderWrite, TryReadFromBytes, Write,
        WriteIntoBytes,
    },
    submessage_elements::SequenceNumberSet,
    types::{Count, SubmessageFlag, SubmessageKind},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AckNackSubmessage {
    final_flag: SubmessageFlag,
    reader_id: EntityId,
    writer_id: EntityId,
    reader_sn_state: SequenceNumberSet,
    count: Count,
}

impl AckNackSubmessage {
    pub fn try_from_bytes(
        submessage_header: &SubmessageHeaderRead,
        mut data: &[u8],
    ) -> RtpsMessageResult<Self> {
        let endianness = submessage_header.endianness();
        Ok(Self {
            final_flag: submessage_header.flags()[1],
            reader_id: EntityId::try_read_from_bytes(&mut data, endianness)?,
            writer_id: EntityId::try_read_from_bytes(&mut data, endianness)?,
            reader_sn_state: SequenceNumberSet::try_read_from_bytes(&mut data, endianness)?,
            count: Count::try_read_from_bytes(&mut data, endianness)?,
        })
    }

    pub fn _final_flag(&self) -> bool {
        self.final_flag
    }

    pub fn reader_id(&self) -> &EntityId {
        &self.reader_id
    }

    pub fn writer_id(&self) -> &EntityId {
        &self.writer_id
    }

    pub fn reader_sn_state(&self) -> &SequenceNumberSet {
        &self.reader_sn_state
    }

    pub fn count(&self) -> Count {
        self.count
    }
}

impl AckNackSubmessage {
    pub fn new(
        final_flag: SubmessageFlag,
        reader_id: EntityId,
        writer_id: EntityId,
        reader_sn_state: SequenceNumberSet,
        count: Count,
    ) -> Self {
        Self {
            final_flag,
            reader_id,
            writer_id,
            reader_sn_state,
            count,
        }
    }
}

impl Submessage for AckNackSubmessage {
    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write) {
        self.reader_id.write_into_bytes(buf);
        self.writer_id.write_into_bytes(buf);
        self.reader_sn_state.write_into_bytes(buf);
        self.count.write_into_bytes(buf);
    }

    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(
            SubmessageKind::ACKNACK,
            &[self.final_flag],
            octets_to_next_header,
        )
        .write_into_bytes(buf);
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
    fn serialize_acknack() {
        let final_flag = false;
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let submessage = AckNackSubmessage::new(
            final_flag,
            reader_id,
            writer_id,
            SequenceNumberSet::new(10, []),
            14,
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x06_u8, 0b_0000_0001, 24, 0, // Submessage header
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // reader_sn_state.base
               10, 0, 0, 0, // reader_sn_state.base
                0, 0, 0, 0, // reader_sn_state.set: numBits (ULong)
                14, 0, 0, 0, // count
            ]
        );
    }

    #[test]
    fn deserialize_acknack() {
        #[rustfmt::skip]
        let mut data = &[
            0x06_u8, 0b_0000_0001, 24, 0, // Submessage header
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // reader_sn_state.base
           10, 0, 0, 0, // reader_sn_state.base
            0, 0, 0, 0, // reader_sn_state.set: numBits (ULong)
            2, 0, 0, 0, // count
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let submessage = AckNackSubmessage::try_from_bytes(&submessage_header, data).unwrap();
        let expected_final_flag = false;
        let expected_reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let expected_writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let expected_reader_sn_state = SequenceNumberSet::new(10, []);
        let expected_count = 2;

        assert_eq!(expected_final_flag, submessage._final_flag());
        assert_eq!(&expected_reader_id, submessage.reader_id());
        assert_eq!(&expected_writer_id, submessage.writer_id());
        assert_eq!(&expected_reader_sn_state, submessage.reader_sn_state());
        assert_eq!(expected_count, submessage.count());
    }
}
