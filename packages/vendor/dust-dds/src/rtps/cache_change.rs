use super::error::RtpsError;
use crate::{
    rtps_messages::{
        self,
        submessage_elements::{ParameterListWrite, ParameterWrite},
        submessages::{
            data::{DataSubmessage, DataSubmessageWrite},
            data_frag::DataFragSubmessageWrite,
        },
        types::ParameterId,
    },
    transport::types::{CacheChange, ChangeKind, EntityId, Guid, GuidPrefix},
};

pub const PID_KEY_HASH: ParameterId = 0x0070;
pub const PID_STATUS_INFO: ParameterId = 0x0071;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct StatusInfo(pub [u8; 4]);
const STATUS_INFO_DISPOSED: StatusInfo = StatusInfo([0, 0, 0, 0b00000001]);
const STATUS_INFO_UNREGISTERED: StatusInfo = StatusInfo([0, 0, 0, 0b0000010]);
const STATUS_INFO_DISPOSED_UNREGISTERED: StatusInfo = StatusInfo([0, 0, 0, 0b00000011]);
const STATUS_INFO_FILTERED: StatusInfo = StatusInfo([0, 0, 0, 0b0000100]);

impl CacheChange {
    pub fn status_info_parameter(&self) -> Option<ParameterWrite<'static>> {
        match self.kind {
            ChangeKind::Alive | ChangeKind::AliveFiltered => None,
            ChangeKind::NotAliveDisposed => Some(ParameterWrite::new(
                PID_STATUS_INFO,
                &STATUS_INFO_DISPOSED.0,
            )),
            ChangeKind::NotAliveUnregistered => Some(ParameterWrite::new(
                PID_STATUS_INFO,
                &STATUS_INFO_UNREGISTERED.0,
            )),
            ChangeKind::NotAliveDisposedUnregistered => Some(ParameterWrite::new(
                PID_STATUS_INFO,
                &STATUS_INFO_DISPOSED_UNREGISTERED.0,
            )),
        }
    }

    pub fn key_hash_parameter<'a>(&'a self) -> Option<ParameterWrite<'a>> {
        self.instance_handle
            .as_ref()
            .map(|i| ParameterWrite::new(PID_KEY_HASH, i))
    }

    pub fn as_data_submessage<'a>(
        &'a self,
        reader_id: EntityId,
        writer_id: EntityId,
        inline_qos: &'a [ParameterWrite<'a>],
    ) -> DataSubmessageWrite<'a> {
        let (data_flag, key_flag) = if self.data_value.is_empty() {
            (false, false)
        } else {
            match self.kind {
                ChangeKind::Alive | ChangeKind::AliveFiltered => (true, false),
                ChangeKind::NotAliveDisposed
                | ChangeKind::NotAliveUnregistered
                | ChangeKind::NotAliveDisposedUnregistered => (false, true),
            }
        };

        let inline_qos_flag = !inline_qos.is_empty();
        let parameter_list = if inline_qos_flag {
            ParameterListWrite::new(inline_qos)
        } else {
            ParameterListWrite::empty()
        };

        DataSubmessageWrite::new(
            inline_qos_flag,
            data_flag,
            key_flag,
            false,
            reader_id,
            writer_id,
            self.sequence_number,
            parameter_list,
            self.data_value.as_ref(),
        )
    }

    pub fn try_from_data_submessage(
        data_submessage: &DataSubmessage,
        source_guid_prefix: GuidPrefix,
        source_timestamp: Option<rtps_messages::types::Time>,
    ) -> Result<Self, RtpsError> {
        let kind = match data_submessage
            .inline_qos()
            .parameter()
            .iter()
            .find(|&x| x.parameter_id() == PID_STATUS_INFO)
        {
            Some(p) => {
                if p.length() == 4 {
                    let status_info =
                        StatusInfo([p.value()[0], p.value()[1], p.value()[2], p.value()[3]]);
                    match status_info {
                        STATUS_INFO_DISPOSED => Ok(ChangeKind::NotAliveDisposed),
                        STATUS_INFO_UNREGISTERED => Ok(ChangeKind::NotAliveUnregistered),
                        STATUS_INFO_DISPOSED_UNREGISTERED => {
                            Ok(ChangeKind::NotAliveDisposedUnregistered)
                        }
                        STATUS_INFO_FILTERED => Ok(ChangeKind::AliveFiltered),
                        _ => Err(RtpsError::InvalidData),
                    }
                } else {
                    Err(RtpsError::InvalidData)
                }
            }
            None => Ok(ChangeKind::Alive),
        }?;

        let instance_handle = match data_submessage
            .inline_qos()
            .parameter()
            .iter()
            .find(|&x| x.parameter_id() == PID_KEY_HASH)
        {
            Some(p) => <[u8; 16]>::try_from(p.value()).ok(),

            None => None,
        };

        Ok(CacheChange {
            kind,
            writer_guid: Guid::new(source_guid_prefix, data_submessage.writer_id()),
            source_timestamp: source_timestamp.map(Into::into),
            instance_handle,
            sequence_number: data_submessage.writer_sn(),
            data_value: data_submessage.serialized_payload().clone().into(),
        })
    }

    pub fn as_data_frag_submessage<'a>(
        &'a self,
        reader_id: EntityId,
        writer_id: EntityId,
        data_max_size_serialized: usize,
        fragment_number: usize,
    ) -> DataFragSubmessageWrite<'a> {
        let inline_qos_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let writer_sn = self.sequence_number;
        let fragment_starting_num = (fragment_number + 1) as u32;
        let fragments_in_submessage = 1;
        let fragment_size = data_max_size_serialized as u16;
        let data_size = self.data_value.len() as u32;

        let start = fragment_number * data_max_size_serialized;
        let end = core::cmp::min(
            (fragment_number + 1) * data_max_size_serialized,
            self.data_value.len(),
        );

        let serialized_payload = &self.data_value.as_ref()[start..end];

        DataFragSubmessageWrite::new(
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
            ParameterListWrite::empty(),
            serialized_payload,
        )
    }
}
