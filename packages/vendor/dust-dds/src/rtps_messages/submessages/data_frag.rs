use crate::transport::types::{EntityId, SequenceNumber};

use super::super::{
    error::{RtpsMessageError, RtpsMessageResult},
    overall_structure::{
        Submessage, SubmessageHeaderRead, SubmessageHeaderWrite, TryReadFromBytes, Write,
        WriteIntoBytes,
    },
    submessage_elements::{ParameterList, ParameterListWrite, SerializedDataFragment},
    types::{FragmentNumber, SubmessageFlag, SubmessageKind},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataFragSubmessageWrite<'a> {
    inline_qos_flag: bool,
    non_standard_payload_flag: SubmessageFlag,
    key_flag: bool,
    reader_id: EntityId,
    writer_id: EntityId,
    writer_sn: SequenceNumber,
    fragment_starting_num: FragmentNumber,
    fragments_in_submessage: u16,
    fragment_size: u16,
    data_size: u32,
    inline_qos: ParameterListWrite<'a>,
    serialized_payload: &'a [u8],
}

impl<'a> DataFragSubmessageWrite<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inline_qos_flag: SubmessageFlag,
        non_standard_payload_flag: SubmessageFlag,
        key_flag: SubmessageFlag,
        reader_id: EntityId,
        writer_id: EntityId,
        writer_sn: SequenceNumber,
        fragment_starting_num: FragmentNumber,
        fragments_in_submessage: u16,
        fragment_size: u16,
        data_size: u32,
        inline_qos: ParameterListWrite<'a>,
        serialized_payload: &'a [u8],
    ) -> Self {
        Self {
            inline_qos_flag,
            non_standard_payload_flag,
            key_flag,
            reader_id,
            writer_id,
            writer_sn,
            fragment_starting_num,
            fragments_in_submessage,
            fragment_size,
            data_size,
            inline_qos,
            serialized_payload,
        }
    }
}

impl Submessage for DataFragSubmessageWrite<'_> {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(
            SubmessageKind::DATA_FRAG,
            &[
                self.inline_qos_flag,
                self.key_flag,
                self.non_standard_payload_flag,
            ],
            octets_to_next_header,
        )
        .write_into_bytes(buf);
    }

    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write) {
        const EXTRA_FLAGS: u16 = 0;
        const OCTETS_TO_INLINE_QOS: u16 = 28;
        EXTRA_FLAGS.write_into_bytes(buf);
        OCTETS_TO_INLINE_QOS.write_into_bytes(buf);
        self.reader_id.write_into_bytes(buf);
        self.writer_id.write_into_bytes(buf);
        self.writer_sn.write_into_bytes(buf);
        self.fragment_starting_num.write_into_bytes(buf);
        self.fragments_in_submessage.write_into_bytes(buf);
        self.fragment_size.write_into_bytes(buf);
        self.data_size.write_into_bytes(buf);
        if self.inline_qos_flag {
            self.inline_qos.write_into_bytes(buf);
        }
        self.serialized_payload.write_into_bytes(buf);
    }
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataFragSubmessage {
    inline_qos_flag: bool,
    non_standard_payload_flag: SubmessageFlag,
    key_flag: bool,
    reader_id: EntityId,
    writer_id: EntityId,
    writer_sn: SequenceNumber,
    fragment_starting_num: FragmentNumber,
    fragments_in_submessage: u16,
    fragment_size: u16,
    data_size: u32,
    inline_qos: ParameterList,
    serialized_payload: SerializedDataFragment,
}

impl DataFragSubmessage {
    pub fn try_from_bytes(
        submessage_header: &SubmessageHeaderRead,
        data: &[u8],
    ) -> RtpsMessageResult<Self> {
        if submessage_header.submessage_length() as usize > data.len() {
            return Err(RtpsMessageError::InvalidData);
        }

        let mut slice = data;
        if data.len() >= 32 {
            let endianness = submessage_header.endianness();
            let inline_qos_flag = submessage_header.flags()[1];
            let key_flag = submessage_header.flags()[2];
            let non_standard_payload_flag = submessage_header.flags()[3];

            let _extra_flags = u16::try_read_from_bytes(&mut slice, endianness)?;
            let octets_to_inline_qos =
                u16::try_read_from_bytes(&mut slice, endianness)? as usize + 4;
            let reader_id = EntityId::try_read_from_bytes(&mut slice, endianness)?;
            let writer_id = EntityId::try_read_from_bytes(&mut slice, endianness)?;
            let writer_sn = SequenceNumber::try_read_from_bytes(&mut slice, endianness)?;
            let fragment_starting_num =
                FragmentNumber::try_read_from_bytes(&mut slice, endianness)?;
            let fragments_in_submessage = u16::try_read_from_bytes(&mut slice, endianness)?;
            let fragment_size = u16::try_read_from_bytes(&mut slice, endianness)?;
            let data_size = u32::try_read_from_bytes(&mut slice, endianness)?;

            let end_position = if submessage_header.submessage_length() == 0 {
                data.len()
            } else {
                submessage_header.submessage_length() as usize
            };

            if octets_to_inline_qos > end_position {
                return Err(RtpsMessageError::InvalidData);
            }

            let mut data_starting_at_inline_qos = &data[octets_to_inline_qos..end_position];

            let inline_qos = if inline_qos_flag {
                ParameterList::try_read_from_bytes(&mut data_starting_at_inline_qos, endianness)?
            } else {
                ParameterList::empty()
            };
            let serialized_payload = SerializedDataFragment::from(data_starting_at_inline_qos);

            Ok(Self {
                inline_qos_flag,
                non_standard_payload_flag,
                key_flag,
                reader_id,
                writer_id,
                writer_sn,
                fragment_starting_num,
                fragments_in_submessage,
                fragment_size,
                data_size,
                inline_qos,
                serialized_payload,
            })
        } else {
            Err(RtpsMessageError::NotEnoughData)
        }
    }

    pub fn inline_qos_flag(&self) -> bool {
        self.inline_qos_flag
    }

    pub fn key_flag(&self) -> bool {
        self.key_flag
    }

    pub fn _non_standard_payload_flag(&self) -> bool {
        self.non_standard_payload_flag
    }

    pub fn reader_id(&self) -> EntityId {
        self.reader_id
    }

    pub fn writer_id(&self) -> EntityId {
        self.writer_id
    }

    pub fn writer_sn(&self) -> SequenceNumber {
        self.writer_sn
    }

    pub fn fragment_starting_num(&self) -> FragmentNumber {
        self.fragment_starting_num
    }

    pub fn fragments_in_submessage(&self) -> u16 {
        self.fragments_in_submessage
    }

    pub fn fragment_size(&self) -> u16 {
        self.fragment_size
    }

    pub fn data_size(&self) -> u32 {
        self.data_size
    }

    pub fn inline_qos(&self) -> &ParameterList {
        &self.inline_qos
    }

    pub fn serialized_payload(&self) -> &SerializedDataFragment {
        &self.serialized_payload
    }
}

impl DataFragSubmessage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inline_qos_flag: SubmessageFlag,
        non_standard_payload_flag: SubmessageFlag,
        key_flag: SubmessageFlag,
        reader_id: EntityId,
        writer_id: EntityId,
        writer_sn: SequenceNumber,
        fragment_starting_num: FragmentNumber,
        fragments_in_submessage: u16,
        fragment_size: u16,
        data_size: u32,
        inline_qos: ParameterList,
        serialized_payload: SerializedDataFragment,
    ) -> Self {
        Self {
            inline_qos_flag,
            non_standard_payload_flag,
            key_flag,
            reader_id,
            writer_id,
            writer_sn,
            fragment_starting_num,
            fragments_in_submessage,
            fragment_size,
            data_size,
            inline_qos,
            serialized_payload,
        }
    }
}

impl Submessage for DataFragSubmessage {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(
            SubmessageKind::DATA_FRAG,
            &[
                self.inline_qos_flag,
                self.key_flag,
                self.non_standard_payload_flag,
            ],
            octets_to_next_header,
        )
        .write_into_bytes(buf);
    }

    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write) {
        const EXTRA_FLAGS: u16 = 0;
        const OCTETS_TO_INLINE_QOS: u16 = 28;
        EXTRA_FLAGS.write_into_bytes(buf);
        OCTETS_TO_INLINE_QOS.write_into_bytes(buf);
        self.reader_id.write_into_bytes(buf);
        self.writer_id.write_into_bytes(buf);
        self.writer_sn.write_into_bytes(buf);
        self.fragment_starting_num.write_into_bytes(buf);
        self.fragments_in_submessage.write_into_bytes(buf);
        self.fragment_size.write_into_bytes(buf);
        self.data_size.write_into_bytes(buf);
        if self.inline_qos_flag {
            self.inline_qos.write_into_bytes(buf);
        }
        self.serialized_payload.write_into_bytes(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rtps_messages::{
            overall_structure::write_submessage_into_bytes_vec, submessage_elements::Parameter,
        },
        transport::types::{USER_DEFINED_READER_GROUP, USER_DEFINED_READER_NO_KEY},
    };

    #[test]
    fn serialize_data_frag_write_no_inline_qos_with_payload() {
        let inline_qos = ParameterListWrite::empty();
        let payload = [1, 2, 3];
        let submessage = DataFragSubmessageWrite::new(
            false,
            false,
            false,
            EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY),
            EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP),
            5,
            2,
            3,
            5,
            4,
            inline_qos,
            &payload,
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x16_u8, 0b_0000_0001, 35, 0, // Submessage header
                0, 0, 28, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
                2, 0, 0, 0, // fragmentStartingNum
                3, 0, 5, 0, // fragmentsInSubmessage | fragmentSize
                4, 0, 0, 0, // sampleSize
                1, 2, 3,    // serialized payload
            ]
        );
    }

    #[test]
    fn serialize_no_inline_qos_no_serialized_payload() {
        let inline_qos = ParameterList::empty();
        let serialized_payload = SerializedDataFragment::default();
        let submessage = DataFragSubmessage::new(
            false,
            false,
            false,
            EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY),
            EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP),
            5,
            2,
            3,
            5,
            4,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x16_u8, 0b_0000_0001, 32, 0, // Submessage header
                0, 0, 28, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
                2, 0, 0, 0, // fragmentStartingNum
                3, 0, 5, 0, // fragmentsInSubmessage | fragmentSize
                4, 0, 0, 0, // sampleSize
            ]
        );
    }

    #[test]
    fn serialize_with_inline_qos_with_serialized_payload() {
        let inline_qos = ParameterList::new(vec![Parameter::new(8, vec![71, 72, 73, 74].into())]);
        let serialized_payload = SerializedDataFragment::from([1, 2, 3].as_slice());
        let submessage = DataFragSubmessage::new(
            true,
            false,
            false,
            EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY),
            EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP),
            6,
            2,
            3,
            5,
            8,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x16_u8, 0b_0000_0011, 47, 0, // Submessage header
                0, 0, 28, 0, // extraFlags | octetsToInlineQos
                1, 2, 3, 4, // readerId
                6, 7, 8, 9, // writerId
                0, 0, 0, 0, // writerSN: high
                6, 0, 0, 0, // writerSN: low
                2, 0, 0, 0, // fragmentStartingNum
                3, 0, 5, 0, // fragmentsInSubmessage | fragmentSize
                8, 0, 0, 0, // sampleSize
                8, 0, 4, 0, // inlineQos: parameterId, length
                71, 72, 73, 74, // inlineQos: value[length]
                1, 0, 0, 0, // inlineQos: Sentinel
                1, 2, 3, // serializedPayload
            ]
        );
    }

    #[test]
    fn deserialize_no_inline_qos_no_serialized_payload() {
        #[rustfmt::skip]
        let mut data = &[
            0x16_u8, 0b_0000_0001, 32, 0, // Submessage header
            0, 0, 28, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            2, 0, 0, 0, // fragmentStartingNum
            3, 0, 5, 0, // fragmentsInSubmessage | fragmentSize
            4, 0, 0, 0, // sampleSize
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let submessage = DataFragSubmessage::try_from_bytes(&submessage_header, data).unwrap();

        let expected_inline_qos_flag = false;
        let expected_non_standard_payload_flag = false;
        let expected_key_flag = false;
        let expected_reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let expected_writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let expected_writer_sn = 5;
        let expected_fragment_starting_num = 2;
        let expected_fragments_in_submessage = 3;
        let expected_data_size = 4;
        let expected_fragment_size = 5;
        let expected_inline_qos = ParameterList::empty();
        let expected_serialized_payload = SerializedDataFragment::default();

        assert_eq!(expected_inline_qos_flag, submessage.inline_qos_flag());
        assert_eq!(
            expected_non_standard_payload_flag,
            submessage._non_standard_payload_flag()
        );
        assert_eq!(expected_key_flag, submessage.key_flag());
        assert_eq!(expected_reader_id, submessage.reader_id());
        assert_eq!(expected_writer_id, submessage.writer_id());
        assert_eq!(expected_writer_sn, submessage.writer_sn());
        assert_eq!(
            expected_fragment_starting_num,
            submessage.fragment_starting_num()
        );
        assert_eq!(
            expected_fragments_in_submessage,
            submessage.fragments_in_submessage()
        );
        assert_eq!(expected_data_size, submessage.data_size());
        assert_eq!(expected_fragment_size, submessage.fragment_size());
        assert_eq!(&expected_inline_qos, submessage.inline_qos());
        assert_eq!(
            &expected_serialized_payload,
            submessage.serialized_payload()
        );
    }

    #[test]
    fn deserialize_with_inline_qos_with_serialized_payload() {
        #[rustfmt::skip]
        let mut data = &[
            0x16_u8, 0b_0000_0011, 48, 0, // Submessage header
            0, 0, 28, 0, // extraFlags | octetsToInlineQos
            1, 2, 3, 4, // readerId
            6, 7, 8, 9, // writerId
            0, 0, 0, 0, // writerSN: high
            6, 0, 0, 0, // writerSN: low
            2, 0, 0, 0, // fragmentStartingNum
            3, 0, 5, 0, // fragmentsInSubmessage | fragmentSize
            8, 0, 0, 0, // sampleSize
            8, 0, 4, 0, // inlineQos: parameterId, length
            71, 72, 73, 74, // inlineQos: value[length]
            1, 0, 0, 0, // inlineQos: Sentinel
            1, 2, 3, 0, // serializedPayload
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let submessage = DataFragSubmessage::try_from_bytes(&submessage_header, data).unwrap();

        let expected_inline_qos_flag = true;
        let expected_non_standard_payload_flag = false;
        let expected_key_flag = false;
        let expected_reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let expected_writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let expected_writer_sn = 6;
        let expected_fragment_starting_num = 2;
        let expected_fragments_in_submessage = 3;
        let expected_data_size = 8;
        let expected_fragment_size = 5;
        let expected_inline_qos =
            ParameterList::new(vec![Parameter::new(8, vec![71, 72, 73, 74].into())]);
        let expected_serialized_payload = SerializedDataFragment::from(&[1, 2, 3, 0][..]);

        assert_eq!(expected_inline_qos_flag, submessage.inline_qos_flag());
        assert_eq!(
            expected_non_standard_payload_flag,
            submessage._non_standard_payload_flag()
        );
        assert_eq!(expected_key_flag, submessage.key_flag());
        assert_eq!(expected_reader_id, submessage.reader_id());
        assert_eq!(expected_writer_id, submessage.writer_id());
        assert_eq!(expected_writer_sn, submessage.writer_sn());
        assert_eq!(
            expected_fragment_starting_num,
            submessage.fragment_starting_num()
        );
        assert_eq!(
            expected_fragments_in_submessage,
            submessage.fragments_in_submessage()
        );
        assert_eq!(expected_data_size, submessage.data_size());
        assert_eq!(expected_fragment_size, submessage.fragment_size());
        assert_eq!(&expected_inline_qos, submessage.inline_qos());
        assert_eq!(
            &expected_serialized_payload,
            submessage.serialized_payload()
        );
    }

    #[test]
    fn fuzz_test_input_1() {
        let mut data = &[
            159, 10, 0, 0, 0, 0, 247, 0, 0, 0, 0, 199, 199, 199, 199, 199, 199, 199, 199, 199, 199,
            199, 199, 199, 199, 199, 199, 10, 0, 0, 0, 0, 0, 37, 159, 31, 36, 93, 159, 159, 159,
            10,
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        // Should not panic with this input
        let _ = DataFragSubmessage::try_from_bytes(&submessage_header, data);
    }
}
