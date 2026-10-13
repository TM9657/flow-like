use crate::infrastructure::{instance::InstanceHandle, qos::SubscriberQos};

pub struct SubscriberEntity {
    pub instance_handle: InstanceHandle,
    pub qos: SubscriberQos,
    pub enabled: bool,
}

impl SubscriberEntity {
    pub fn new(instance_handle: InstanceHandle, qos: SubscriberQos) -> Self {
        Self {
            instance_handle,
            qos,
            enabled: false,
        }
    }
}
