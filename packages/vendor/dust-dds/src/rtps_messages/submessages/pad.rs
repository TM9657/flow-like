use super::super::{
    error::RtpsMessageResult,
    overall_structure::{
        Submessage, SubmessageHeaderRead, SubmessageHeaderWrite, Write, WriteIntoBytes,
    },
    types::SubmessageKind,
};

#[derive(Debug, PartialEq, Eq)]
pub struct PadSubmessage {}

impl PadSubmessage {
    pub fn try_from_bytes(
        _submessage_header: &SubmessageHeaderRead,
        _data: &[u8],
    ) -> RtpsMessageResult<Self> {
        Ok(Self {})
    }
}

impl PadSubmessage {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PadSubmessage {
    fn default() -> Self {
        Self::new()
    }
}

impl Submessage for PadSubmessage {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(SubmessageKind::PAD, &[], octets_to_next_header)
            .write_into_bytes(buf);
    }

    fn write_submessage_elements_into_bytes(&self, _buf: &mut dyn Write) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtps_messages::overall_structure::{
        SubmessageHeaderRead, write_submessage_into_bytes_vec,
    };

    #[test]
    fn serialize_pad() {
        let submessage = PadSubmessage::new();
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x01, 0b_0000_0001, 0, 0, // Submessage header
            ]
        );
    }

    #[test]
    fn deserialize_pad() {
        #[rustfmt::skip]
        let mut data = &[
            0x01, 0b_0000_0001, 0, 0, // Submessage header
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let submessage = PadSubmessage::try_from_bytes(&submessage_header, data);

        assert!(submessage.is_ok())
    }
}
