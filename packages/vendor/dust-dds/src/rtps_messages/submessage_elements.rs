use super::{
    error::{RtpsMessageError, RtpsMessageResult},
    overall_structure::{BufRead, Endianness, TryReadFromBytes, Write, WriteIntoBytes},
    types::FragmentNumber,
    types::ParameterId,
};
use crate::transport::types::{Locator, SequenceNumber};
use alloc::{sync::Arc, vec::Vec};
use core::ops::Range;

// This files shall only contain the types as listed in the DDS-RTPS Version 2.3
// 8.3.5 RTPS SubmessageElements

const PID_SENTINEL: i16 = 0x0001;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceNumberSet {
    base: SequenceNumber,
    num_bits: u32,
    bitmap: [i32; 8],
}

impl SequenceNumberSet {
    pub fn new(base: SequenceNumber, set: impl IntoIterator<Item = SequenceNumber>) -> Self {
        let mut bitmap = [0; 8];
        let mut num_bits = 0;
        for sequence_number in set {
            let delta_n = (sequence_number - base) as u32;
            let bitmap_num = delta_n / 32;
            bitmap[bitmap_num as usize] |= 1 << (31 - delta_n % 32);
            if delta_n + 1 > num_bits {
                num_bits = delta_n + 1;
            }
        }

        Self {
            base,
            num_bits,
            bitmap,
        }
    }

    pub fn base(&self) -> SequenceNumber {
        self.base
    }

    pub fn set(&self) -> impl Iterator<Item = SequenceNumber> + '_ {
        struct SequenceNumberSetIterator<'a> {
            set: &'a SequenceNumberSet,
            index: usize,
        }

        impl Iterator for SequenceNumberSetIterator<'_> {
            type Item = SequenceNumber;

            fn next(&mut self) -> Option<Self::Item> {
                while self.index < self.set.num_bits as usize {
                    let delta_n = self.index;
                    self.index += 1;
                    let bitmap_num = delta_n / 32;
                    let mask = 1 << (31 - delta_n % 32);
                    if self.set.bitmap[bitmap_num] & mask == mask {
                        return Some(self.set.base + delta_n as i64);
                    }
                }
                None
            }
        }

        SequenceNumberSetIterator {
            set: self,
            index: 0,
        }
    }
}

impl TryReadFromBytes for SequenceNumberSet {
    fn try_read_from_bytes(data: &mut &[u8], endianness: &Endianness) -> RtpsMessageResult<Self> {
        let base = SequenceNumber::try_read_from_bytes(data, endianness)?;
        let num_bits = u32::try_read_from_bytes(data, endianness)?;
        if num_bits > 256 {
            return Err(RtpsMessageError::InvalidData);
        }
        let number_of_bitmap_elements = num_bits.div_ceil(32) as usize; //In standard referred to as "M"
        let mut bitmap = [0; 8];
        for bitmap_i in bitmap.iter_mut().take(number_of_bitmap_elements) {
            *bitmap_i = i32::try_read_from_bytes(data, endianness)?;
        }
        Ok(Self {
            base,
            num_bits,
            bitmap,
        })
    }
}

impl WriteIntoBytes for SequenceNumberSet {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        let number_of_bitmap_elements = self.num_bits.div_ceil(32) as usize; //In standard referred to as "M"

        self.base.write_into_bytes(buf);
        self.num_bits.write_into_bytes(buf);
        for bitmap_element in &self.bitmap[..number_of_bitmap_elements] {
            bitmap_element.write_into_bytes(buf);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentNumberSet {
    base: FragmentNumber,
    num_bits: u32,
    bitmap: [i32; 8],
}

impl FragmentNumberSet {
    pub fn new(base: FragmentNumber, set: impl IntoIterator<Item = FragmentNumber>) -> Self {
        let mut bitmap = [0; 8];
        let mut num_bits = 0;
        for fragment_number in set {
            let delta_n = fragment_number - base;
            let bitmap_num = delta_n / 32;
            bitmap[bitmap_num as usize] |= 1 << (31 - delta_n % 32);
            if delta_n + 1 > num_bits {
                num_bits = delta_n + 1;
            }
        }

        Self {
            base,
            num_bits,
            bitmap,
        }
    }

    pub fn try_read_from_bytes(
        data: &mut &[u8],
        endianness: &Endianness,
    ) -> RtpsMessageResult<Self> {
        let base = FragmentNumber::try_read_from_bytes(data, endianness)?;
        let num_bits = u32::try_read_from_bytes(data, endianness)?;
        let number_of_bitmap_elements = num_bits.div_ceil(32) as usize; //In standard referred to as "M"
        let mut bitmap = [0; 8];

        for bitmap_i in bitmap.iter_mut().take(number_of_bitmap_elements) {
            *bitmap_i = i32::try_read_from_bytes(data, endianness)?;
        }

        let mut set = Vec::with_capacity(256);
        for delta_n in 0..num_bits as usize {
            if (bitmap[delta_n / 32] & (1 << (31 - delta_n % 32))) == (1 << (31 - delta_n % 32)) {
                set.push(base + delta_n as u32);
            }
        }
        Ok(Self::new(base, set))
    }

    pub fn base(&self) -> FragmentNumber {
        self.base
    }

    pub fn set(&self) -> impl Iterator<Item = FragmentNumber> + '_ {
        struct FragmentNumberSetIterator<'a> {
            set: &'a FragmentNumberSet,
            index: usize,
        }

        impl Iterator for FragmentNumberSetIterator<'_> {
            type Item = FragmentNumber;

            fn next(&mut self) -> Option<Self::Item> {
                while self.index < self.set.num_bits as usize {
                    let delta_n = self.index;
                    self.index += 1;
                    let bitmap_num = delta_n / 32;
                    let mask = 1 << (31 - delta_n % 32);
                    if self.set.bitmap[bitmap_num] & mask == mask {
                        return Some(self.set.base + delta_n as u32);
                    }
                }
                None
            }
        }

        FragmentNumberSetIterator {
            set: self,
            index: 0,
        }
    }
}

impl WriteIntoBytes for FragmentNumberSet {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        let number_of_bitmap_elements = self.num_bits.div_ceil(32) as usize; //In standard referred to as "M"

        self.base.write_into_bytes(buf);
        self.num_bits.write_into_bytes(buf);
        for bitmap_element in &self.bitmap[..number_of_bitmap_elements] {
            bitmap_element.write_into_bytes(buf);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocatorList {
    value: Vec<Locator>,
}

impl LocatorList {
    pub fn new(value: Vec<Locator>) -> Self {
        Self { value }
    }

    pub fn value(&self) -> &[Locator] {
        self.value.as_ref()
    }
}

impl TryReadFromBytes for LocatorList {
    fn try_read_from_bytes(data: &mut &[u8], endianness: &Endianness) -> RtpsMessageResult<Self> {
        let num_locators = u32::try_read_from_bytes(data, endianness)?;
        let mut locator_list = Vec::new();
        for _ in 0..num_locators {
            locator_list.push(Locator::try_read_from_bytes(data, endianness)?);
        }
        Ok(Self::new(locator_list))
    }
}

impl WriteIntoBytes for LocatorList {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        let num_locators = self.value().len() as u32;
        num_locators.write_into_bytes(buf);
        for locator in self.value().iter() {
            locator.write_into_bytes(buf);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Parameter {
    parameter_id: ParameterId,
    value: Arc<[u8]>,
}

impl Parameter {
    pub fn new(parameter_id: ParameterId, value: Arc<[u8]>) -> Self {
        Self {
            parameter_id,
            value,
        }
    }

    pub fn parameter_id(&self) -> ParameterId {
        self.parameter_id
    }

    pub fn value(&self) -> &[u8] {
        self.value.as_ref()
    }

    pub fn length(&self) -> i16 {
        self.value.len() as i16
    }

    fn try_read_from_bytes(data: &mut &[u8], endianness: &Endianness) -> RtpsMessageResult<Self> {
        let mut slice = *data;
        let len = slice.len();
        if len < 4 {
            return Err(RtpsMessageError::NotEnoughData);
        }
        let parameter_id = i16::try_read_from_bytes(&mut slice, endianness)?;
        // This value shouldn't be negative otherwise when converting to usize it overflows
        let length = u16::try_read_from_bytes(&mut slice, endianness)?;
        data.consume(4);

        if parameter_id != PID_SENTINEL && length % 4 != 0 {
            return Err(RtpsMessageError::InvalidData);
        }
        let value = if parameter_id == PID_SENTINEL {
            Arc::new([])
        } else {
            if data.len() < length as usize {
                return Err(RtpsMessageError::NotEnoughData);
            }
            let value: Arc<[u8]> = Arc::from(&data[0..length as usize]);
            if length as usize > value.len() {
                return Err(RtpsMessageError::InvalidData);
            }
            data.consume(length as usize);
            value
        };

        Ok(Self::new(parameter_id, value))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParameterList {
    parameter: Vec<Parameter>,
}

impl ParameterList {
    pub fn new(parameter: Vec<Parameter>) -> Self {
        Self { parameter }
    }

    pub fn empty() -> Self {
        Self {
            parameter: Vec::new(),
        }
    }

    pub fn parameter(&self) -> &[Parameter] {
        self.parameter.as_ref()
    }

    pub fn try_read_from_bytes(
        data: &mut &[u8],
        endianness: &Endianness,
    ) -> RtpsMessageResult<Self> {
        const MAX_PARAMETERS: usize = 2_usize.pow(16);

        let mut parameter = Vec::new();
        for _ in 0..MAX_PARAMETERS {
            let parameter_i = Parameter::try_read_from_bytes(data, endianness)?;
            if parameter_i.parameter_id() == PID_SENTINEL {
                break;
            } else {
                parameter.push(parameter_i);
            }
        }
        Ok(Self { parameter })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ParameterWrite<'a> {
    parameter_id: ParameterId,
    value: &'a [u8],
}

impl<'a> ParameterWrite<'a> {
    pub const fn new(parameter_id: ParameterId, value: &'a [u8]) -> Self {
        Self {
            parameter_id,
            value,
        }
    }

    pub fn parameter_id(&self) -> ParameterId {
        self.parameter_id
    }

    pub fn value(&self) -> &'a [u8] {
        self.value
    }

    pub fn length(&self) -> i16 {
        self.value.len() as i16
    }
}

impl WriteIntoBytes for ParameterWrite<'_> {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        let padding = match self.value().len() % 4 {
            1 => &[0_u8; 3][..],
            2 => &[0_u8; 2][..],
            3 => &[0_u8; 1][..],
            _ => &[0_u8; 0][..],
        };
        let length = self.value().len() + padding.len();
        self.parameter_id().write_into_bytes(buf);
        (length as i16).write_into_bytes(buf);
        self.value().write_into_bytes(buf);
        padding.write_into_bytes(buf);
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ParameterListWrite<'a> {
    parameter: &'a [ParameterWrite<'a>],
}

impl<'a> ParameterListWrite<'a> {
    pub const fn new(parameter: &'a [ParameterWrite<'a>]) -> Self {
        Self { parameter }
    }

    pub const fn empty() -> Self {
        Self { parameter: &[] }
    }

    pub fn parameter(&self) -> &'a [ParameterWrite<'a>] {
        self.parameter
    }
}

impl WriteIntoBytes for ParameterListWrite<'_> {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        for parameter in self.parameter() {
            parameter.write_into_bytes(buf);
        }
        PID_SENTINEL.write_into_bytes(buf);
        [0_u8; 2].write_into_bytes(buf);
    }
}

impl WriteIntoBytes for Parameter {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        let padding = match self.value().len() % 4 {
            1 => &[0_u8; 3][..],
            2 => &[0_u8; 2][..],
            3 => &[0_u8; 1][..],
            _ => &[0_u8; 0][..],
        };
        let length = self.value().len() + padding.len();
        self.parameter_id().write_into_bytes(buf);
        (length as i16).write_into_bytes(buf);
        self.value().write_into_bytes(buf);
        padding.write_into_bytes(buf);
    }
}

impl WriteIntoBytes for ParameterList {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        for parameter in self.parameter().iter() {
            parameter.write_into_bytes(buf);
        }
        PID_SENTINEL.write_into_bytes(buf);
        [0_u8; 2].write_into_bytes(buf);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SerializedDataFragment {
    data: Data,
    range: Range<usize>,
}

impl SerializedDataFragment {
    pub fn new(data: Data, range: Range<usize>) -> Self {
        Self { data, range }
    }
}

impl Default for SerializedDataFragment {
    fn default() -> Self {
        Self {
            data: Data::default(),
            range: 0..0,
        }
    }
}

impl AsRef<[u8]> for SerializedDataFragment {
    fn as_ref(&self) -> &[u8] {
        &self.data.as_ref()[self.range.start..self.range.end]
    }
}

impl From<&[u8]> for SerializedDataFragment {
    fn from(value: &[u8]) -> Self {
        Self {
            data: Data::new(Arc::from(value)),
            range: 0..value.len(),
        }
    }
}

impl WriteIntoBytes for SerializedDataFragment {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        self.data.as_ref()[self.range.start..self.range.end]
            .as_ref()
            .write_into_bytes(buf);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Data(Arc<[u8]>);

impl Data {
    pub fn new(data: Arc<[u8]>) -> Self {
        Self(data)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for Data {
    fn default() -> Self {
        Self(Arc::new([]))
    }
}

impl From<Vec<u8>> for Data {
    fn from(value: Vec<u8>) -> Self {
        Self(value.into_boxed_slice().into())
    }
}

impl From<Arc<[u8]>> for Data {
    fn from(value: Arc<[u8]>) -> Self {
        Self(value)
    }
}

impl From<Data> for Arc<[u8]> {
    fn from(value: Data) -> Self {
        value.0.clone()
    }
}

impl AsRef<[u8]> for Data {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl WriteIntoBytes for Data {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        self.0.as_ref().write_into_bytes(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rtps_messages::{overall_structure::write_into_bytes_vec, types::Count},
        transport::types::{GuidPrefix, ProtocolVersion, VendorId},
    };

    #[test]
    fn sequence_number_set_methods() {
        let base = 100;
        let set = [102, 200, 355];
        let seq_num_set = SequenceNumberSet::new(base, set);

        assert_eq!(seq_num_set.base(), base);
        assert!(
            seq_num_set.set().eq(set),
            "{:?} not equal to {:?}",
            seq_num_set.set().collect::<Vec<_>>(),
            set
        );
    }

    #[test]
    fn serialize_fragment_number_max_gap() {
        let fragment_number_set = FragmentNumberSet::new(2, vec![2, 257]);
        #[rustfmt::skip]
        assert_eq!(write_into_bytes_vec(fragment_number_set), vec![
            2, 0, 0, 0, // bitmapBase: (unsigned long)
            0, 1, 0, 0, // numBits (unsigned long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_1000_0000, // bitmap[0] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[1] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[2] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[3] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[4] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[5] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[6] (long)
            0b000_0001, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[7] (long)
        ]);
    }

    #[test]
    fn serialize_locator_list() {
        let locator_1 = Locator::new(1, 2, [3; 16]);
        let locator_2 = Locator::new(2, 2, [3; 16]);
        let locator_list = LocatorList::new(vec![locator_1, locator_2]);
        assert_eq!(
            write_into_bytes_vec(locator_list),
            vec![
                2, 0, 0, 0, // numLocators (unsigned long)
                1, 0, 0, 0, // kind (long)
                2, 0, 0, 0, // port (unsigned long)
                3, 3, 3, 3, // address (octet[16])
                3, 3, 3, 3, // address (octet[16])
                3, 3, 3, 3, // address (octet[16])
                3, 3, 3, 3, // address (octet[16])
                2, 0, 0, 0, // kind (long)
                2, 0, 0, 0, // port (unsigned long)
                3, 3, 3, 3, // address (octet[16])
                3, 3, 3, 3, // address (octet[16])
                3, 3, 3, 3, // address (octet[16])
                3, 3, 3, 3, // address (octet[16])
            ]
        );
    }

    #[test]
    fn deserialize_count() {
        let expected = 7;
        assert_eq!(
            expected,
            Count::try_read_from_bytes(
                &mut &[
                    7, 0, 0, 0, //value (long)
                ][..],
                &Endianness::LittleEndian
            )
            .unwrap()
        );
    }

    #[test]
    fn deserialize_locator_list() {
        let locator_1 = Locator::new(1, 2, [3; 16]);
        let locator_2 = Locator::new(2, 2, [3; 16]);
        let expected = LocatorList::new(vec![locator_1, locator_2]);
        #[rustfmt::skip]
        let result = LocatorList::try_read_from_bytes(&mut &[
            2, 0, 0, 0,  // numLocators (unsigned long)
            1, 0, 0, 0, // kind (long)
            2, 0, 0, 0, // port (unsigned long)
            3, 3, 3, 3, // address (octet[16])
            3, 3, 3, 3, // address (octet[16])
            3, 3, 3, 3, // address (octet[16])
            3, 3, 3, 3, // address (octet[16])
            2, 0, 0, 0, // kind (long)
            2, 0, 0, 0, // port (unsigned long)
            3, 3, 3, 3, // address (octet[16])
            3, 3, 3, 3, // address (octet[16])
            3, 3, 3, 3, // address (octet[16])
            3, 3, 3, 3, // address (octet[16])
        ][..], &Endianness::LittleEndian).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_fragment_number() {
        let expected = 7;
        assert_eq!(
            expected,
            FragmentNumber::try_read_from_bytes(
                &mut &[
                    7, 0, 0, 0, // (unsigned long)
                ][..],
                &Endianness::LittleEndian
            )
            .unwrap()
        );
    }

    #[test]
    fn deserialize_fragment_number_set_max_gap() {
        let expected = FragmentNumberSet::new(2, vec![2, 257]);
        #[rustfmt::skip]
        let result = FragmentNumberSet::try_read_from_bytes(&mut &[
            2, 0, 0, 0, // bitmapBase: (unsigned long)
            0, 1, 0, 0, // numBits (unsigned long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_1000_0000, // bitmap[0] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[1] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[2] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[3] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[4] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[5] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[6] (long)
            0b000_0001, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[7] (long)

        ][..], &Endianness::LittleEndian).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_guid_prefix() {
        let expected = [1; 12];
        #[rustfmt::skip]
        assert_eq!(expected, GuidPrefix::try_read_from_bytes(&mut &[
            1, 1, 1, 1,
            1, 1, 1, 1,
            1, 1, 1, 1,
        ][..], &Endianness::LittleEndian).unwrap());
    }

    #[test]
    fn deserialize_protocol_version() {
        let expected = ProtocolVersion::new(2, 3);
        assert_eq!(
            expected,
            ProtocolVersion::try_read_from_bytes(&mut &[2, 3][..], &Endianness::LittleEndian)
                .unwrap()
        );
    }

    #[test]
    fn deserialize_vendor_id() {
        let expected = [1, 2];
        assert_eq!(
            expected,
            VendorId::try_read_from_bytes(&mut &[1, 2][..], &Endianness::LittleEndian).unwrap()
        );
    }

    #[test]
    fn serialize_sequence_number() {
        let sequence_number = i64::MAX;
        #[rustfmt::skip]
        assert_eq!(write_into_bytes_vec(sequence_number), vec![
            0xff, 0xff, 0xff, 0x7f, // bitmapBase: high (long)
            0xff, 0xff, 0xff, 0xff, // bitmapBase: low (unsigned long)
        ]);
    }

    #[test]
    fn deserialize_sequence_number() {
        let expected = 7;
        assert_eq!(
            expected,
            SequenceNumber::try_read_from_bytes(
                &mut &[
                    0, 0, 0, 0, // high (long)
                    7, 0, 0, 0, // low (unsigned long)
                ][..],
                &Endianness::LittleEndian
            )
            .unwrap()
        );
    }

    #[test]
    fn deserialize_sequence_number_largest_old() {
        let expected = i64::MAX;
        assert_eq!(
            expected,
            SequenceNumber::try_read_from_bytes(
                &mut &[
                    0xff, 0xff, 0xff, 0x7f, // bitmapBase: high (long)
                    0xff, 0xff, 0xff, 0xff, // bitmapBase: low (unsigned long)
                ][..],
                &Endianness::LittleEndian
            )
            .unwrap()
        );
    }

    #[test]
    fn deserialize_sequence_number_largest() {
        let expected = i64::MAX;
        assert_eq!(
            expected,
            SequenceNumber::try_read_from_bytes(
                &mut &[
                    0xff, 0xff, 0xff, 0x7f, // bitmapBase: high (long)
                    0xff, 0xff, 0xff, 0xff, // bitmapBase: low (unsigned long)
                ][..],
                &Endianness::LittleEndian
            )
            .unwrap()
        );
    }

    #[test]
    fn serialize_sequence_number_set_max_gap() {
        let sequence_number_set = SequenceNumberSet::new(2, [2, 257]);
        #[rustfmt::skip]
        assert_eq!(write_into_bytes_vec(sequence_number_set), vec![
            0, 0, 0, 0, // bitmapBase: high (long)
            2, 0, 0, 0, // bitmapBase: low (unsigned long)
            0, 1, 0, 0, // numBits (unsigned long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_1000_0000, // bitmap[0] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[1] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[2] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[3] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[4] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[5] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[6] (long)
            0b000_0001, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[7] (long)
        ]);
    }

    #[test]
    fn deserialize_sequence_number_set_empty() {
        let expected = SequenceNumberSet::new(2, []);
        #[rustfmt::skip]
        let result = SequenceNumberSet::try_read_from_bytes(&mut &[
            0, 0, 0, 0, // bitmapBase: high (long)
            2, 0, 0, 0, // bitmapBase: low (unsigned long)
            0, 0, 0, 0, // numBits (unsigned long)
        ][..], &Endianness::LittleEndian).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_sequence_number_set_too_many_bits() {
        #[rustfmt::skip]
        let result = SequenceNumberSet::try_read_from_bytes(&mut &[
            0, 0, 0, 0, // bitmapBase: high (long)
            2, 0, 0, 0, // bitmapBase: low (unsigned long)
            0xff, 0xff, 0xff, 0xff, // numBits (unsigned long)
        ][..], &Endianness::LittleEndian);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_sequence_number_set_with_gaps() {
        let expected = SequenceNumberSet::new(7, [9, 11]);
        #[rustfmt::skip]
        let result = SequenceNumberSet::try_read_from_bytes(&mut &[
            0, 0, 0, 0, // bitmapBase: high (long)
            7, 0, 0, 0, // bitmapBase: low (unsigned long)
            5, 0, 0, 0, // numBits (unsigned long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0010_1000, // bitmap[0] (long)
        ][..], &Endianness::LittleEndian).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_sequence_number_set_max_gap() {
        let expected = SequenceNumberSet::new(2, [2, 257]);
        #[rustfmt::skip]
        let result = SequenceNumberSet::try_read_from_bytes(&mut &[
            0, 0, 0, 0, // bitmapBase: high (long)
            2, 0, 0, 0, // bitmapBase: low (unsigned long)
            0, 1, 0, 0, // numBits (unsigned long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_1000_0000, // bitmap[0] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[1] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[2] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[3] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[4] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[5] (long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[6] (long)
            0b000_0001, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[7] (long)
        ][..], &Endianness::LittleEndian).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_sequence_number_set_faulty_num_bitmaps() {
        let expected = SequenceNumberSet::new(2, []);
        #[rustfmt::skip]
        let result = SequenceNumberSet::try_read_from_bytes(&mut &[
            0, 0, 0, 0, // bitmapBase: high (long)
            2, 0, 0, 0, // bitmapBase: low (unsigned long)
            0, 0, 0, 0, // numBits (unsigned long)
            0b000_0000, 0b_0000_0000, 0b_0000_0000, 0b_1000_0000, // bitmap[0] (long)
        ][..], &Endianness::LittleEndian).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_parameter() {
        let parameter = Parameter::new(2, vec![5, 6, 7, 8].into());
        #[rustfmt::skip]
        assert_eq!(write_into_bytes_vec(parameter), vec![
            0x02, 0x00, 4, 0, // Parameter | length
            5, 6, 7, 8,       // value
        ]);
    }

    #[test]
    fn serialize_parameter_non_multiple_4() {
        let parameter = Parameter::new(2, vec![5, 6, 7].into());
        #[rustfmt::skip]
        assert_eq!(write_into_bytes_vec(parameter), vec![
            0x02, 0x00, 4, 0, // Parameter | length
            5, 6, 7, 0,       // value
        ]);
    }

    #[test]
    fn serialize_parameter_zero_size() {
        let parameter = Parameter::new(2, vec![].into());
        assert_eq!(
            write_into_bytes_vec(parameter),
            vec![
                0x02, 0x00, 0, 0, // Parameter | length
            ]
        );
    }

    #[test]
    fn serialize_parameter_list() {
        let parameter_1 = Parameter::new(2, vec![51, 61, 71, 81].into());
        let parameter_2 = Parameter::new(3, vec![52, 62, 0, 0].into());
        let parameter_list_submessage_element = ParameterList::new(vec![parameter_1, parameter_2]);
        #[rustfmt::skip]
        assert_eq!(write_into_bytes_vec(parameter_list_submessage_element), vec![
            0x02, 0x00, 4, 0, // Parameter ID | length
            51, 61, 71, 81,   // value
            0x03, 0x00, 4, 0, // Parameter ID | length
            52, 62, 0, 0,   // value
            0x01, 0x00, 0, 0, // Sentinel: PID_SENTINEL | PID_PAD
        ]);
    }

    #[test]
    fn serialize_parameter_list_empty() {
        let parameter = ParameterList::empty();
        #[rustfmt::skip]
        assert_eq!(write_into_bytes_vec(parameter), vec![
            0x01, 0x00, 0, 0, // Sentinel: PID_SENTINEL | PID_PAD
        ]);
    }

    #[test]
    fn serialize_parameter_write() {
        let parameter = ParameterWrite::new(2, &[51, 61, 71, 81]);
        #[rustfmt::skip]
        assert_eq!(write_into_bytes_vec(parameter), vec![
            0x02, 0x00, 4, 0, // Parameter ID | length
            51, 61, 71, 81,   // value
        ]);
    }

    #[test]
    fn serialize_parameter_list_write() {
        let parameter_1 = ParameterWrite::new(2, &[51, 61, 71, 81]);
        let parameter_2 = ParameterWrite::new(3, &[52, 62, 0, 0]);
        let parameters = [parameter_1, parameter_2];
        let parameter_list = ParameterListWrite::new(&parameters);
        #[rustfmt::skip]
        assert_eq!(write_into_bytes_vec(parameter_list), vec![
            0x02, 0x00, 4, 0, // Parameter ID | length
            51, 61, 71, 81,   // value
            0x03, 0x00, 4, 0, // Parameter ID | length
            52, 62, 0, 0,   // value
            0x01, 0x00, 0, 0, // Sentinel: PID_SENTINEL | PID_PAD
        ]);
    }

    #[test]
    fn serialize_parameter_list_write_empty() {
        let parameter_list = ParameterListWrite::empty();
        #[rustfmt::skip]
        assert_eq!(write_into_bytes_vec(parameter_list), vec![
            0x01, 0x00, 0, 0, // Sentinel: PID_SENTINEL | PID_PAD
        ]);
    }

    #[test]
    fn deserialize_parameter() {
        let expected = Parameter::new(2, vec![5, 6, 7, 8, 9, 10, 11, 12].into());
        let result = Parameter::try_read_from_bytes(
            &mut [
                0x02, 0x00, 8, 0, // Parameter ID | length
                5, 6, 7, 8, // value
                9, 10, 11, 12, // value
            ]
            .as_slice(),
            &Endianness::LittleEndian,
        )
        .unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter_non_multiple_of_4() {
        // Note: the padding 0 is part of the resulting parameter (this may seems wrong but is specified in RTPS 2.5).
        let expected = Parameter::new(2, vec![5, 6, 7, 8, 9, 10, 11, 0].into());
        let result = Parameter::try_read_from_bytes(
            &mut [
                0x02, 0x00, 8, 0, // Parameter ID | length
                5, 6, 7, 8, // value | value | value | value
                9, 10, 11,
                0, // value | value | value | padding
                   //13, 13, 13, 13, // other data
            ]
            .as_slice(),
            &Endianness::LittleEndian,
        )
        .unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter_faulty_non_multiple_of_4_length() {
        let result = Parameter::try_read_from_bytes(
            &mut [
                0x02, 0x00, 3, 0, // Parameter ID | length
                5, 6, 7, 8, // value
            ]
            .as_slice(),
            &Endianness::LittleEndian,
        );
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_parameter_faulty_to_large_length() {
        let result = Parameter::try_read_from_bytes(
            &mut [
                0x02, 0x00, 8, 0, // Parameter ID | length
                5, 6, 7, 8, // value
            ]
            .as_slice(),
            &Endianness::LittleEndian,
        );
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_parameter_sentinel_length_should_be_ignored() {
        let expected = Parameter::new(1, vec![].into());
        let result = Parameter::try_read_from_bytes(
            &mut [
                0x01, 0x00, 3, 0, // Parameter ID | length
            ]
            .as_slice(),
            &Endianness::LittleEndian,
        )
        .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn deserialize_parameter_list() {
        let expected = ParameterList::new(vec![
            Parameter::new(2, vec![15, 16, 17, 18].into()),
            Parameter::new(3, vec![25, 26, 27, 28].into()),
        ]);
        let result = ParameterList::try_read_from_bytes(
            &mut [
                0x02, 0x00, 4, 0, // Parameter ID | length
                15, 16, 17, 18, // value
                0x03, 0x00, 4, 0, // Parameter ID | length
                25, 26, 27, 28, // value
                0x01, 0x00, 0, 0, // Sentinel: Parameter ID | length
            ]
            .as_slice(),
            &Endianness::LittleEndian,
        )
        .unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter_list_with_long_parameter_including_sentinel() {
        let parameter_value_expected = vec![
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
        ];

        let expected =
            ParameterList::new(vec![Parameter::new(0x32, parameter_value_expected.into())]);
        let result = ParameterList::try_read_from_bytes(
            &mut [
                0x32, 0x00, 24, 0x00, // Parameter ID | length
                0x01, 0x00, 0x00, 0x00, // Parameter value
                0x01, 0x00, 0x00, 0x00, // Parameter value
                0x01, 0x01, 0x01, 0x01, // Parameter value
                0x01, 0x01, 0x01, 0x01, // Parameter value
                0x01, 0x01, 0x01, 0x01, // Parameter value
                0x01, 0x01, 0x01, 0x01, // Parameter value
                0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
            ]
            .as_slice(),
            &Endianness::LittleEndian,
        )
        .unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter_list_with_multiple_parameters_with_same_id() {
        #[rustfmt::skip]
        let parameter_value_expected1 = vec![
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
        ];
        #[rustfmt::skip]
        let parameter_value_expected2 = vec![
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0x02, 0x02, 0x02, 0x02,
            0x02, 0x02, 0x02, 0x02,
            0x02, 0x02, 0x02, 0x02,
            0x02, 0x02, 0x02, 0x02,
        ];

        let expected = ParameterList::new(vec![
            Parameter::new(0x32, parameter_value_expected1.into()),
            Parameter::new(0x32, parameter_value_expected2.into()),
        ]);
        #[rustfmt::skip]
        let result = ParameterList::try_read_from_bytes(&mut [
            0x32, 0x00, 24, 0x00, // Parameter ID | length
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x32, 0x00, 24, 0x00, // Parameter ID | length
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
        ].as_slice(), &Endianness::LittleEndian).unwrap();
        assert_eq!(expected, result);
    }
}
