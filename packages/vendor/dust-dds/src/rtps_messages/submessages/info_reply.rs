use super::super::{
    error::RtpsMessageResult,
    overall_structure::{
        Submessage, SubmessageHeaderRead, SubmessageHeaderWrite, TryReadFromBytes, Write,
        WriteIntoBytes,
    },
    submessage_elements::LocatorList,
    types::{SubmessageFlag, SubmessageKind},
};
use alloc::vec::Vec;

#[derive(Debug, PartialEq, Eq)]
pub struct InfoReplySubmessage {
    multicast_flag: SubmessageFlag,
    unicast_locator_list: LocatorList,
    multicast_locator_list: LocatorList,
}

impl InfoReplySubmessage {
    pub fn try_from_bytes(
        submessage_header: &SubmessageHeaderRead,
        mut data: &[u8],
    ) -> RtpsMessageResult<Self> {
        let endianness = submessage_header.endianness();
        let multicast_flag = submessage_header.flags()[1];
        let unicast_locator_list = LocatorList::try_read_from_bytes(&mut data, endianness)?;
        let multicast_locator_list = if multicast_flag {
            LocatorList::try_read_from_bytes(&mut data, endianness)?
        } else {
            LocatorList::new(Vec::new())
        };
        Ok(Self {
            multicast_flag,
            unicast_locator_list,
            multicast_locator_list,
        })
    }

    pub fn _multicast_flag(&self) -> bool {
        self.multicast_flag
    }

    pub fn _unicast_locator_list(&self) -> &LocatorList {
        &self.unicast_locator_list
    }

    pub fn _multicast_locator_list(&self) -> &LocatorList {
        &self.multicast_locator_list
    }
}

impl Submessage for InfoReplySubmessage {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write) {
        SubmessageHeaderWrite::new(SubmessageKind::INFO_REPLY, &[], octets_to_next_header)
            .write_into_bytes(buf);
    }

    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write) {
        self.unicast_locator_list.write_into_bytes(buf);
        if self.multicast_flag {
            self.multicast_locator_list.write_into_bytes(buf);
        }
    }
}

impl InfoReplySubmessage {
    pub fn _new(
        multicast_flag: SubmessageFlag,
        unicast_locator_list: LocatorList,
        multicast_locator_list: LocatorList,
    ) -> Self {
        Self {
            multicast_flag,
            unicast_locator_list,
            multicast_locator_list,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rtps_messages::overall_structure::write_submessage_into_bytes_vec,
        transport::types::Locator,
    };

    #[test]
    fn serialize_info_reply() {
        let locator = Locator::new(11, 12, [1; 16]);
        let submessage = InfoReplySubmessage::_new(
            false,
            LocatorList::new(vec![locator]),
            LocatorList::new(vec![]),
        );
        #[rustfmt::skip]
        assert_eq!(write_submessage_into_bytes_vec(&submessage), vec![
                0x0f, 0b_0000_0001, 28, 0, // Submessage header
                1, 0, 0, 0, //numLocators
                11, 0, 0, 0, //kind
                12, 0, 0, 0, //port
                1, 1, 1, 1, //address
                1, 1, 1, 1, //address
                1, 1, 1, 1, //address
                1, 1, 1, 1, //address
            ]
        );
    }

    #[test]
    fn deserialize_info_reply() {
        #[rustfmt::skip]
        let mut data = &[
            0x0f, 0b_0000_0001, 28, 0, // Submessage header
            1, 0, 0, 0, //numLocators
            11, 0, 0, 0, //kind
            12, 0, 0, 0, //port
            1, 1, 1, 1, //address
            1, 1, 1, 1, //address
            1, 1, 1, 1, //address
            1, 1, 1, 1, //address
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let submessage = InfoReplySubmessage::try_from_bytes(&submessage_header, data).unwrap();
        let locator = Locator::new(11, 12, [1; 16]);
        let expected_multicast_flag = false;
        let expected_unicast_locator_list = LocatorList::new(vec![locator]);
        let expected_multicast_locator_list = LocatorList::new(vec![]);

        assert_eq!(expected_multicast_flag, submessage._multicast_flag());
        assert_eq!(
            &expected_unicast_locator_list,
            submessage._unicast_locator_list()
        );
        assert_eq!(
            &expected_multicast_locator_list,
            submessage._multicast_locator_list()
        );
    }

    #[test]
    fn deserialize_info_reply_with_multicast() {
        #[rustfmt::skip]
        let mut data = &[
            0x0f, 0b_0000_0011, 56, 0, // Submessage header
            0, 0, 0, 0, //numLocators
            2, 0, 0, 0, //numLocators
            11, 0, 0, 0, //kind
            12, 0, 0, 0, //port
            1, 1, 1, 1, //address
            1, 1, 1, 1, //address
            1, 1, 1, 1, //address
            1, 1, 1, 1, //address
            11, 0, 0, 0, //kind
            12, 0, 0, 0, //port
            2, 2, 2, 2, //address
            2, 2, 2, 2, //address
            2, 2, 2, 2, //address
            2, 2, 2, 2, //address
        ][..];
        let submessage_header = SubmessageHeaderRead::try_read_from_bytes(&mut data).unwrap();
        let submessage = InfoReplySubmessage::try_from_bytes(&submessage_header, data).unwrap();
        let locator1 = Locator::new(11, 12, [1; 16]);
        let locator2 = Locator::new(11, 12, [2; 16]);
        let expected_multicast_flag = true;
        let expected_unicast_locator_list = LocatorList::new(vec![]);
        let expected_multicast_locator_list = LocatorList::new(vec![locator1, locator2]);

        assert_eq!(expected_multicast_flag, submessage._multicast_flag());
        assert_eq!(
            &expected_unicast_locator_list,
            submessage._unicast_locator_list()
        );
        assert_eq!(
            &expected_multicast_locator_list,
            submessage._multicast_locator_list()
        );
    }
}
