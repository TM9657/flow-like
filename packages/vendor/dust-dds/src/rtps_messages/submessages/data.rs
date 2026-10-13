use super::super::{
    error::{RtpsMessageError, RtpsMessageResult},
    overall_structure::{
        Submessage, SubmessageHeaderRead, SubmessageHeaderWrite, TryReadFromBytes, Write,
        WriteIntoBytes,
    },
    submessage_elements::{Data, ParameterList, ParameterListWrite},
    types::{SubmessageFlag, SubmessageKind},
};
use crate::transport::types::{EntityId, SequenceNumber};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataSubmessageWrite<'a> {
    inline_qos_flag: bool,
    data_flag: bool,
    key_flag: bool,
    non_standard_payload_flag: SubmessageFlag,
    reader_id: EntityId,
    writer_id: EntityId,
    writer_sn: SequenceNumber,
    inline_qos: ParameterListWrite<'a>,
    serialized_payload: &'a [u8],
}

impl<'a> DataSubmessageWrite<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inline_qos_flag: SubmessageFlag,
        data_flag: SubmessageFlag,
        key_flag: SubmessageFlag,
        non_standard_payload_flag: SubmessageFlag,
        reader_id: EntityId,
        writer_id: EntityId,
        writer_sn: SequenceNumber,
        inline_qos: ParameterListWrite<'a>,
        serialized_payload: &'a [u8],
    ) -> Self {
        Self {
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        }
    }

    pub fn inline_qos(&self) -> &ParameterListWrite<'a> {
        &self.inline_qos
    }

    pub fn serialized_payload(&self) -> &'a [u8] {
        self.serialized_payload
    }
}

impl Submessage for DataSubmessageWrite<'_> {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(
            SubmessageKind::DATA,
            &[
                self.inline_qos_flag,
                self.data_flag,
                self.key_flag,
                self.non_standard_payload_flag,
            ],
            octets_to_next_header,
        )
        .write_into_bytes(buf)
    }

    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write) {
        const EXTRA_FLAGS: u16 = 0;
        const OCTETS_TO_INLINE_QOS: u16 = 16;
        EXTRA_FLAGS.write_into_bytes(buf);
        OCTETS_TO_INLINE_QOS.write_into_bytes(buf);
        self.reader_id.write_into_bytes(buf);
        self.writer_id.write_into_bytes(buf);
        self.writer_sn.write_into_bytes(buf);
        if self.inline_qos_flag {
            self.inline_qos.write_into_bytes(buf);
        }
        if self.data_flag || self.key_flag {
            self.serialized_payload.write_into_bytes(buf);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataSubmessage {
    inline_qos_flag: bool,
    data_flag: bool,
    key_flag: bool,
    non_standard_payload_flag: SubmessageFlag,
    reader_id: EntityId,
    writer_id: EntityId,
    writer_sn: SequenceNumber,
    inline_qos: ParameterList,
    serialized_payload: Data,
}

impl DataSubmessage {
    pub fn try_from_bytes(
        submessage_header: &SubmessageHeaderRead,
        data: &[u8],
    ) -> RtpsMessageResult<Self> {
        if submessage_header.submessage_length() as usize > data.len() {
            return Err(RtpsMessageError::InvalidData);
        }
        let mut slice = data;
        let endianness = submessage_header.endianness();
        let inline_qos_flag = submessage_header.flags()[1];
        let data_flag = submessage_header.flags()[2];
        let key_flag = submessage_header.flags()[3];
        let non_standard_payload_flag = submessage_header.flags()[4];

        let _extra_flags = u16::try_read_from_bytes(&mut slice, endianness)?;
        let octets_to_inline_qos = u16::try_read_from_bytes(&mut slice, endianness)? as usize + 4;
        let reader_id = EntityId::try_read_from_bytes(&mut slice, endianness)?;
        let writer_id = EntityId::try_read_from_bytes(&mut slice, endianness)?;
        let writer_sn = SequenceNumber::try_read_from_bytes(&mut slice, endianness)?;

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
            ParameterList::try_read_from_bytes(
                &mut data_starting_at_inline_qos,
                submessage_header.endianness(),
            )?
        } else {
            ParameterList::empty()
        };

        let serialized_payload = if data_flag || key_flag {
            Data::new(data_starting_at_inline_qos.into())
        } else {
            Data::default()
        };

        Ok(Self {
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inline_qos_flag: SubmessageFlag,
        data_flag: SubmessageFlag,
        key_flag: SubmessageFlag,
        non_standard_payload_flag: SubmessageFlag,
        reader_id: EntityId,
        writer_id: EntityId,
        writer_sn: SequenceNumber,
        inline_qos: ParameterList,
        serialized_payload: Data,
    ) -> Self {
        Self {
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        }
    }

    pub fn _inline_qos_flag(&self) -> bool {
        self.inline_qos_flag
    }

    pub fn _data_flag(&self) -> bool {
        self.data_flag
    }

    pub fn _key_flag(&self) -> bool {
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

    pub fn inline_qos(&self) -> &ParameterList {
        &self.inline_qos
    }

    pub fn serialized_payload(&self) -> &Data {
        &self.serialized_payload
    }
}

impl Submessage for DataSubmessage {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(
            SubmessageKind::DATA,
            &[
                self.inline_qos_flag,
                self.data_flag,
                self.key_flag,
                self.non_standard_payload_flag,
            ],
            octets_to_next_header,
        )
        .write_into_bytes(buf)
    }

    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write) {
        const EXTRA_FLAGS: u16 = 0;
        const OCTETS_TO_INLINE_QOS: u16 = 16;
        EXTRA_FLAGS.write_into_bytes(buf);
        OCTETS_TO_INLINE_QOS.write_into_bytes(buf);
        self.reader_id.write_into_bytes(buf);
        self.writer_id.write_into_bytes(buf);
        self.writer_sn.write_into_bytes(buf);
        if self.inline_qos_flag {
            self.inline_qos.write_into_bytes(buf);
        }
        if self.data_flag || self.key_flag {
            self.serialized_payload.write_into_bytes(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rtps_messages::{
            overall_structure::write_submessage_into_bytes_vec,
            submessage_elements::{Parameter, ParameterWrite},
        },
        transport::types::{USER_DEFINED_READER_GROUP, USER_DEFINED_READER_NO_KEY},
    };

    #[test]
    fn serialize_data_write_no_inline_qos_with_serialized_payload() {
        let inline_qos_flag = false;
        let data_flag = true;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let writer_sn = 5;
        let inline_qos = ParameterListWrite::empty();
        let payload = [1, 2, 3];
        let submessage = DataSubmessageWrite::new(
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            &payload,
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x15, 0b_0000_0101, 23, 0, // Submessage header
                0, 0, 16, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
                1, 2, 3, // serialized payload
            ]
        );
    }

    #[test]
    fn serialize_data_write_with_inline_qos() {
        let inline_qos_flag = true;
        let data_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let writer_sn = 5;
        let parameter_1 = ParameterWrite::new(6, &[10, 11, 12, 13]);
        let parameter_2 = ParameterWrite::new(7, &[20, 21, 22, 23]);
        let parameters = [parameter_1, parameter_2];
        let inline_qos = ParameterListWrite::new(&parameters);
        let submessage = DataSubmessageWrite::new(
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            &[],
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x15, 0b_0000_0011, 40, 0, // Submessage header
                0, 0, 16, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
                6, 0, 4, 0, // inlineQos: parameterId_1, length_1
                10, 11, 12, 13, // inlineQos: value_1[length_1]
                7, 0, 4, 0, // inlineQos: parameterId_2, length_2
                20, 21, 22, 23, // inlineQos: value_2[length_2]
                1, 0, 0, 0, // inlineQos: Sentinel
            ]
        );
    }

    #[test]
    fn serialize_no_inline_qos_no_serialized_payload() {
        let inline_qos_flag = false;
        let data_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let writer_sn = 5;
        let inline_qos = ParameterList::empty();
        let serialized_payload = Data::new(vec![].into());
        let submessage = DataSubmessage::new(
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x15_u8, 0b_0000_0001, 20, 0, // Submessage header
                0, 0, 16, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
            ]
        );
    }

    #[test]
    fn serialize_with_inline_qos_no_serialized_payload() {
        let inline_qos_flag = true;
        let data_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let writer_sn = 5;
        let parameter_1 = Parameter::new(6, vec![10, 11, 12, 13].into());
        let parameter_2 = Parameter::new(7, vec![20, 21, 22, 23].into());
        let inline_qos = ParameterList::new(vec![parameter_1, parameter_2]);
        let serialized_payload = Data::default();

        let submessage = DataSubmessage::new(
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x15, 0b_0000_0011, 40, 0, // Submessage header
                0, 0, 16, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
                6, 0, 4, 0, // inlineQos: parameterId_1, length_1
                10, 11, 12, 13, // inlineQos: value_1[length_1]
                7, 0, 4, 0, // inlineQos: parameterId_2, length_2
                20, 21, 22, 23, // inlineQos: value_2[length_2]
                1, 0, 0, 0, // inlineQos: Sentinel
            ]
        );
    }

    #[test]
    fn serialize_no_inline_qos_with_serialized_payload() {
        let inline_qos_flag = false;
        let data_flag = true;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let writer_sn = 5;
        let inline_qos = ParameterList::empty();
        let serialized_payload = Data::new(vec![1, 2, 3].into());
        let submessage = DataSubmessage::new(
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x15, 0b_0000_0101, 23, 0, // Submessage header
                0, 0, 16, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
                1, 2, 3, // serialized payload (Note: padding is not added by the submessage, it should have been added by the CDR data itself; hence 3 bytes only)
            ]
        );
    }

    #[test]
    fn deserialize_no_inline_qos_no_serialized_payload() {
        let inline_qos_flag = false;
        let data_flag = false;
        let key_flag = false;
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let writer_sn = 5;
        let inline_qos = ParameterList::empty();
        let serialized_payload = Data::new(vec![].into());

        #[rustfmt::skip]
        let mut data = &[
            0x15, 0b_0000_0001, 20, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let data_submessage = DataSubmessage::try_from_bytes(&submessage_header, data).unwrap();

        assert_eq!(inline_qos_flag, data_submessage._inline_qos_flag());
        assert_eq!(data_flag, data_submessage._data_flag());
        assert_eq!(key_flag, data_submessage._key_flag());
        assert_eq!(reader_id, data_submessage.reader_id());
        assert_eq!(writer_id, data_submessage.writer_id());
        assert_eq!(writer_sn, data_submessage.writer_sn());
        assert_eq!(&inline_qos, data_submessage.inline_qos());
        assert_eq!(&serialized_payload, data_submessage.serialized_payload());
    }

    #[test]
    fn deserialize_no_inline_qos_with_serialized_payload() {
        let expected_inline_qos = ParameterList::empty();
        let expected_serialized_payload = Data::new(vec![1, 2, 3, 4].into());

        #[rustfmt::skip]
        let mut data = &[
            0x15, 0b_0000_0101, 24, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            1, 2, 3, 4, // SerializedPayload
            123, 123, 123 // Following data
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let data_submessage = DataSubmessage::try_from_bytes(&submessage_header, data).unwrap();
        assert_eq!(&expected_inline_qos, data_submessage.inline_qos());
        assert_eq!(
            &expected_serialized_payload,
            data_submessage.serialized_payload()
        );
    }

    #[test]
    fn deserialize_with_inline_qos_no_serialized_payload() {
        let inline_qos = ParameterList::new(vec![
            Parameter::new(6, vec![10, 11, 12, 13].into()),
            Parameter::new(7, vec![20, 21, 22, 23].into()),
        ]);
        let serialized_payload = Data::new(vec![].into());

        #[rustfmt::skip]
        let mut data = &[
            0x15, 0b_0000_0011, 40, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            6, 0, 4, 0, // inlineQos: parameterId_1, length_1
            10, 11, 12, 13, // inlineQos: value_1[length_1]
            7, 0, 4, 0, // inlineQos: parameterId_2, length_2
            20, 21, 22, 23, // inlineQos: value_2[length_2]
            1, 0, 1, 0, // inlineQos: Sentinel
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let data_submessage = DataSubmessage::try_from_bytes(&submessage_header, data).unwrap();
        assert_eq!(&inline_qos, data_submessage.inline_qos());
        assert_eq!(&serialized_payload, data_submessage.serialized_payload());
    }

    #[test]
    fn deserialize_with_inline_qos_with_serialized_payload() {
        let expected_inline_qos = ParameterList::new(vec![
            Parameter::new(6, vec![10, 11, 12, 13].into()),
            Parameter::new(7, vec![20, 21, 22, 23].into()),
        ]);
        let expected_serialized_payload = Data::new(vec![1, 2, 3, 4].into());

        #[rustfmt::skip]
        let mut data = &[
            0x15, 0b_0000_0111, 44, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            6, 0, 4, 0, // inlineQos: parameterId_1, length_1
            10, 11, 12, 13, // inlineQos: value_1[length_1]
            7, 0, 4, 0, // inlineQos: parameterId_2, length_2
            20, 21, 22, 23, // inlineQos: value_2[length_2]
            1, 0, 1, 0, // inlineQos: Sentinel
            1, 2, 3, 4, // SerializedPayload
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let data_submessage = DataSubmessage::try_from_bytes(&submessage_header, data).unwrap();

        assert_eq!(&expected_inline_qos, data_submessage.inline_qos());
        assert_eq!(
            &expected_serialized_payload,
            data_submessage.serialized_payload()
        );
    }

    #[test]
    fn deserialize_with_inline_qos_with_serialized_payload_unspecified_message_length() {
        let expected_inline_qos = ParameterList::new(vec![
            Parameter::new(6, vec![10, 11, 12, 13].into()),
            Parameter::new(7, vec![20, 21, 22, 23].into()),
        ]);
        let expected_serialized_payload = Data::new(vec![1, 2, 3, 4].into());

        #[rustfmt::skip]
        let mut data = &[
            0x15, 0b_0000_0111, 0, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            6, 0, 4, 0, // inlineQos: parameterId_1, length_1
            10, 11, 12, 13, // inlineQos: value_1[length_1]
            7, 0, 4, 0, // inlineQos: parameterId_2, length_2
            20, 21, 22, 23, // inlineQos: value_2[length_2]
            1, 0, 1, 0, // inlineQos: Sentinel
            1, 2, 3, 4, // SerializedPayload
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let data_submessage = DataSubmessage::try_from_bytes(&submessage_header, data).unwrap();

        assert_eq!(&expected_inline_qos, data_submessage.inline_qos());
        assert_eq!(
            &expected_serialized_payload,
            data_submessage.serialized_payload()
        );
    }

    #[test]
    fn deserialize_octets_to_inline_qos_non_16() {
        let expected_inline_qos = ParameterList::new(vec![
            Parameter::new(6, vec![10, 11, 12, 13].into()),
            Parameter::new(7, vec![20, 21, 22, 23].into()),
        ]);
        #[rustfmt::skip]
        let mut data = &[
            0x15, 0b_0000_0011, 44, 0, // Submessage header
            123, 123, 20, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            123, 123, 123, 123, // Unknown data
            6, 0, 4, 0, // inlineQos: parameterId_1, length_1
            10, 11, 12, 13, // inlineQos: value_1[length_1]
            7, 0, 4, 0, // inlineQos: parameterId_2, length_2
            20, 21, 22, 23, // inlineQos: value_2[length_2]
            1, 0, 1, 0, // inlineQos: Sentinel
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let data_submessage = DataSubmessage::try_from_bytes(&submessage_header, data).unwrap();

        assert_eq!(&expected_inline_qos, data_submessage.inline_qos());
    }

    #[test]
    fn fuzz_test_input_1() {
        let mut data = &[
            0x00, 0x32, 0x00, 0x00, // Submessage header
            0xa2, 0xa2, 0xa2, 0x0a, // extraFlags, octetsToInlineQos
            0x00, 0x00, 0x00, 0x10, // readerId: value[4]
            0x00, 0x00, 0x00, 0x00, // writerId: value[4]
            0xa2, 0xa2, 0xa2, 0xa2, // writerSN: high
            0xa2, 0xa2, 0xa2, 0xa2, // writerSN: low
            0x0a,
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        // Should not panic with this input
        let _ = DataSubmessage::try_from_bytes(&submessage_header, data);
    }

    #[test]
    fn fuzz_test_input_2() {
        let mut data = &[
            0, 6, 0, 8, // Submessage header
            1, 0, 0, 0, // extraFlags, octetsToInlineQos
            0, 0, 0, 16, // readerId: value[4]
            122, 0, 0, 0, // writerId: value[4]
            1, 0, 0, 0, // writerSN: high
            0, 0, 36, 0, // writerSN: low
            45, 0, 0, 3, // Other data
            32, 0, 0, 0, // Other data
            0, 0, 189, 0, // Other data
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        // Should not panic with this input
        let _ = DataSubmessage::try_from_bytes(&submessage_header, data);
    }

    #[test]
    fn fuzz_test_input_3() {
        let mut data = &[
            0, 6, 0, 51, 9, 0, 0, 0, 0, 45, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 252, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 9, 0,
            0, 0, 45, 110, 0, 0, 8, 0, 2, 0, 0, 0, 1, 0,
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        // Should not panic with this input
        let _ = DataSubmessage::try_from_bytes(&submessage_header, data);
    }
}
