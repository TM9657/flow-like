use crate::transport::types::{EntityId, SequenceNumber};

use super::super::{
    error::RtpsMessageResult,
    overall_structure::{
        Submessage, SubmessageHeaderRead, SubmessageHeaderWrite, TryReadFromBytes, Write,
        WriteIntoBytes,
    },
    types::{Count, SubmessageFlag, SubmessageKind},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HeartbeatSubmessage {
    final_flag: SubmessageFlag,
    liveliness_flag: SubmessageFlag,
    reader_id: EntityId,
    writer_id: EntityId,
    first_sn: SequenceNumber,
    last_sn: SequenceNumber,
    count: Count,
}

impl HeartbeatSubmessage {
    pub fn try_from_bytes(
        submessage_header: &SubmessageHeaderRead,
        mut data: &[u8],
    ) -> RtpsMessageResult<Self> {
        let endianness = submessage_header.endianness();
        Ok(Self {
            final_flag: submessage_header.flags()[1],
            liveliness_flag: submessage_header.flags()[2],
            reader_id: EntityId::try_read_from_bytes(&mut data, endianness)?,
            writer_id: EntityId::try_read_from_bytes(&mut data, endianness)?,
            first_sn: SequenceNumber::try_read_from_bytes(&mut data, endianness)?,
            last_sn: SequenceNumber::try_read_from_bytes(&mut data, endianness)?,
            count: Count::try_read_from_bytes(&mut data, endianness)?,
        })
    }

    pub fn final_flag(&self) -> bool {
        self.final_flag
    }

    pub fn liveliness_flag(&self) -> bool {
        self.liveliness_flag
    }

    pub fn _reader_id(&self) -> EntityId {
        self.reader_id
    }

    pub fn writer_id(&self) -> EntityId {
        self.writer_id
    }

    pub fn first_sn(&self) -> SequenceNumber {
        self.first_sn
    }

    pub fn last_sn(&self) -> SequenceNumber {
        self.last_sn
    }

    pub fn count(&self) -> Count {
        self.count
    }
}

impl HeartbeatSubmessage {
    pub fn new(
        final_flag: SubmessageFlag,
        liveliness_flag: SubmessageFlag,
        reader_id: EntityId,
        writer_id: EntityId,
        first_sn: SequenceNumber,
        last_sn: SequenceNumber,
        count: Count,
    ) -> Self {
        Self {
            final_flag,
            liveliness_flag,
            reader_id,
            writer_id,
            first_sn,
            last_sn,
            count,
        }
    }
}

impl Submessage for HeartbeatSubmessage {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(
            SubmessageKind::HEARTBEAT,
            &[self.final_flag, self.liveliness_flag],
            octets_to_next_header,
        )
        .write_into_bytes(buf);
    }

    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write) {
        self.reader_id.write_into_bytes(buf);
        self.writer_id.write_into_bytes(buf);
        self.first_sn.write_into_bytes(buf);
        self.last_sn.write_into_bytes(buf);
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
    fn serialize_heart_beat() {
        let final_flag = false;
        let liveliness_flag = true;
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let first_sn = 5;
        let last_sn = 7;
        let count = 2;
        let submessage = HeartbeatSubmessage::new(
            final_flag,
            liveliness_flag,
            reader_id,
            writer_id,
            first_sn,
            last_sn,
            count,
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x07_u8, 0b_0000_0101, 28, 0, // Submessage header
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // firstSN: SequenceNumber: high
                5, 0, 0, 0, // firstSN: SequenceNumber: low
                0, 0, 0, 0, // lastSN: SequenceNumberSet: high
                7, 0, 0, 0, // lastSN: SequenceNumberSet: low
                2, 0, 0, 0, // count: Count: value (long)
            ]
        );
    }

    #[test]
    fn deserialize_heart_beat() {
        let expected_final_flag = false;
        let expected_liveliness_flag = true;
        let expected_reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let expected_writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let expected_first_sn = 5;
        let expected_last_sn = 7;
        let expected_count = 2;
        #[rustfmt::skip]
        let mut data = &[
            0x07, 0b_0000_0101, 28, 0, // Submessage header
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // firstSN: SequenceNumber: high
            5, 0, 0, 0, // firstSN: SequenceNumber: low
            0, 0, 0, 0, // lastSN: SequenceNumberSet: high
            7, 0, 0, 0, // lastSN: SequenceNumberSet: low
            2, 0, 0, 0, // count: Count: value (long)
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let submessage = HeartbeatSubmessage::try_from_bytes(&submessage_header, data).unwrap();
        assert_eq!(expected_final_flag, submessage.final_flag());
        assert_eq!(expected_liveliness_flag, submessage.liveliness_flag());
        assert_eq!(expected_reader_id, submessage._reader_id());
        assert_eq!(expected_writer_id, submessage.writer_id());
        assert_eq!(expected_first_sn, submessage.first_sn());
        assert_eq!(expected_last_sn, submessage.last_sn());
        assert_eq!(expected_count, submessage.count());
    }
}
