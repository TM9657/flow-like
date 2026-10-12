use crate::transport::types::{GuidPrefix, Long, ProtocolVersion, VendorId};

use super::super::{
    error::RtpsMessageResult,
    overall_structure::{
        Submessage, SubmessageHeaderRead, SubmessageHeaderWrite, TryReadFromBytes, Write,
        WriteIntoBytes,
    },
    types::SubmessageKind,
};

#[derive(Debug, PartialEq, Eq)]
pub struct InfoSourceSubmessage {
    protocol_version: ProtocolVersion,
    vendor_id: VendorId,
    guid_prefix: GuidPrefix,
}

impl InfoSourceSubmessage {
    pub fn try_from_bytes(
        submessage_header: &SubmessageHeaderRead,
        mut data: &[u8],
    ) -> RtpsMessageResult<Self> {
        let endianness = submessage_header.endianness();
        let _unused = Long::try_read_from_bytes(&mut data, endianness)?;
        Ok(Self {
            protocol_version: ProtocolVersion::try_read_from_bytes(&mut data, endianness)?,
            vendor_id: VendorId::try_read_from_bytes(&mut data, endianness)?,
            guid_prefix: GuidPrefix::try_read_from_bytes(&mut data, endianness)?,
        })
    }

    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub fn vendor_id(&self) -> VendorId {
        self.vendor_id
    }

    pub fn guid_prefix(&self) -> GuidPrefix {
        self.guid_prefix
    }
}

impl InfoSourceSubmessage {
    pub fn _new(
        protocol_version: ProtocolVersion,
        vendor_id: VendorId,
        guid_prefix: GuidPrefix,
    ) -> Self {
        Self {
            protocol_version,
            vendor_id,
            guid_prefix,
        }
    }
}

impl Submessage for InfoSourceSubmessage {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(SubmessageKind::INFO_SRC, &[], octets_to_next_header)
            .write_into_bytes(buf);
    }

    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write) {
        0_u32.write_into_bytes(buf);
        self.protocol_version.write_into_bytes(buf);
        self.vendor_id.write_into_bytes(buf);
        self.guid_prefix.write_into_bytes(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rtps::types::{PROTOCOLVERSION_1_0, VENDOR_ID_UNKNOWN},
        rtps_messages::overall_structure::write_submessage_into_bytes_vec,
        transport::types::GUIDPREFIX_UNKNOWN,
    };

    #[test]
    fn serialize_info_source() {
        let submessage =
            InfoSourceSubmessage::_new(PROTOCOLVERSION_1_0, VENDOR_ID_UNKNOWN, GUIDPREFIX_UNKNOWN);
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x0c, 0b_0000_0001, 20, 0, // Submessage header
                0, 0, 0, 0, // unused
                1, 0, 0, 0, //protocol_version | vendor_id
                0, 0, 0, 0, //guid_prefix
                0, 0, 0, 0, //guid_prefix
                0, 0, 0, 0, //guid_prefix
            ]
        );
    }

    #[test]
    fn deserialize_info_source() {
        #[rustfmt::skip]
        let mut data = &[
            0x0c, 0b_0000_0001, 20, 0, // Submessage header
            0, 0, 0, 0, // unused
            1, 0, 0, 0, //protocol_version | vendor_id
            0, 0, 0, 0, //guid_prefix
            0, 0, 0, 0, //guid_prefix
            0, 0, 0, 0, //guid_prefix
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let submessage = InfoSourceSubmessage::try_from_bytes(&submessage_header, data).unwrap();

        let expected_protocol_version = PROTOCOLVERSION_1_0;
        let expected_vendor_id = VENDOR_ID_UNKNOWN;
        let expected_guid_prefix = GUIDPREFIX_UNKNOWN;

        assert_eq!(expected_protocol_version, submessage.protocol_version());
        assert_eq!(expected_vendor_id, submessage.vendor_id());
        assert_eq!(expected_guid_prefix, submessage.guid_prefix());
    }
}
