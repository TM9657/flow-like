use crate::{
    dcps::dcps_domain_participant::data_reader_entity::DataReaderEntity,
    infrastructure::{instance::InstanceHandle, qos::DataReaderQos},
};
use alloc::sync::Arc;
use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub struct BuiltinDataReader<R, T> {
    pub reader: DataReaderEntity<R>,
    phantom: PhantomData<fn() -> T>,
}

impl<R, T> BuiltinDataReader<R, T> {
    pub fn new(
        instance_handle: InstanceHandle,
        qos: DataReaderQos,
        topic_name: Arc<str>,
        transport_reader: R,
    ) -> Self {
        Self {
            reader: DataReaderEntity::new(instance_handle, qos, topic_name, transport_reader),
            phantom: PhantomData,
        }
    }
}

impl<R, T> Deref for BuiltinDataReader<R, T> {
    type Target = DataReaderEntity<R>;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

impl<R, T> DerefMut for BuiltinDataReader<R, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}
