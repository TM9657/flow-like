use crate::transport::types::{EntityId, SequenceNumber};

use super::super::{
    error::RtpsMessageResult,
    overall_structure::{
        Submessage, SubmessageHeaderRead, SubmessageHeaderWrite, TryReadFromBytes, Write,
        WriteIntoBytes,
    },
    submessage_elements::SequenceNumberSet,
    types::SubmessageKind,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GapSubmessage {
    reader_id: EntityId,
    writer_id: EntityId,
    gap_start: SequenceNumber,
    gap_list: SequenceNumberSet,
}

impl GapSubmessage {
    pub fn try_from_bytes(
        submessage_header: &SubmessageHeaderRead,
        mut data: &[u8],
    ) -> RtpsMessageResult<Self> {
        let endianness = submessage_header.endianness();
        Ok(Self {
            reader_id: EntityId::try_read_from_bytes(&mut data, endianness)?,
            writer_id: EntityId::try_read_from_bytes(&mut data, endianness)?,
            gap_start: SequenceNumber::try_read_from_bytes(&mut data, endianness)?,
            gap_list: SequenceNumberSet::try_read_from_bytes(&mut data, endianness)?,
        })
    }

    pub fn _reader_id(&self) -> EntityId {
        self.reader_id
    }

    pub fn writer_id(&self) -> EntityId {
        self.writer_id
    }

    pub fn gap_start(&self) -> SequenceNumber {
        self.gap_start
    }

    pub fn gap_list(&self) -> &SequenceNumberSet {
        &self.gap_list
    }
}

impl GapSubmessage {
    pub fn new(
        reader_id: EntityId,
        writer_id: EntityId,
        gap_start: SequenceNumber,
        gap_list: SequenceNumberSet,
    ) -> Self {
        Self {
            reader_id,
            writer_id,
            gap_start,
            gap_list,
        }
    }
}

impl Submessage for GapSubmessage {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(SubmessageKind::GAP, &[], octets_to_next_header)
            .write_into_bytes(buf)
    }

    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write) {
        self.reader_id.write_into_bytes(buf);
        self.writer_id.write_into_bytes(buf);
        self.gap_start.write_into_bytes(buf);
        self.gap_list.write_into_bytes(buf);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        rtps_messages::overall_structure::write_submessage_into_bytes_vec,
        transport::types::{USER_DEFINED_READER_GROUP, USER_DEFINED_READER_NO_KEY},
    };

    #[test]
    fn serialize_gap() {
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let gap_start = 5;
        let gap_list = SequenceNumberSet::new(10, []);
        let submessage = GapSubmessage::new(reader_id, writer_id, gap_start, gap_list);
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x08_u8, 0b_0000_0001, 28, 0, // Submessage header
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // gapStart: SequenceNumber: high
                5, 0, 0, 0, // gapStart: SequenceNumber: low
                0, 0, 0, 0, // gapList: SequenceNumberSet: bitmapBase: high
               10, 0, 0, 0, // gapList: SequenceNumberSet: bitmapBase: low
                0, 0, 0, 0, // gapList: SequenceNumberSet: numBits (ULong)
            ]
        );
    }

    use super::*;
    #[test]
    fn deserialize_gap() {
        let expected_reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let expected_writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let expected_gap_start = 5;
        let expected_gap_list = SequenceNumberSet::new(10, []);
        #[rustfmt::skip]
        let mut data = &[
            0x08, 0b_0000_0001, 28, 0, // Submessage header
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // gapStart: SequenceNumber: high
            5, 0, 0, 0, // gapStart: SequenceNumber: low
            0, 0, 0, 0, // gapList: SequenceNumberSet: bitmapBase: high
           10, 0, 0, 0, // gapList: SequenceNumberSet: bitmapBase: low
            0, 0, 0, 0, // gapList: SequenceNumberSet: numBits (ULong)
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let submessage = GapSubmessage::try_from_bytes(&submessage_header, data).unwrap();
        assert_eq!(expected_reader_id, submessage._reader_id());
        assert_eq!(expected_writer_id, submessage.writer_id());
        assert_eq!(expected_gap_start, submessage.gap_start());
        assert_eq!(&expected_gap_list, submessage.gap_list());
    }
}
