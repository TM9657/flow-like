use crate::{
    rtps_messages::overall_structure::{RtpsMessageHeader, RtpsMessageWrite, Submessage},
    transport::types::GuidPrefix,
};

use super::types::{PROTOCOLVERSION_2_4, VENDOR_ID_S2E};

impl<'a> RtpsMessageWrite<'a> {
    pub fn from_submessages(
        buffer: &'a mut [u8],
        submessages: &[&(dyn Submessage + Send)],
        guid_prefix: GuidPrefix,
    ) -> Self {
        let header = RtpsMessageHeader::new(PROTOCOLVERSION_2_4, VENDOR_ID_S2E, guid_prefix);
        RtpsMessageWrite::new(buffer, &header, submessages)
    }
}
