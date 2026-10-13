use crate::dds_async::domain_participant::DomainParticipantAsync;
use alloc::string::String;

/// Async version of [`TopicDescription`](crate::topic_definition::topic_description::TopicDescription).
pub trait TopicDescriptionAsync {
    /// Async version of [`get_participant`](crate::topic_definition::topic::Topic::get_participant).
    fn get_participant(&self) -> DomainParticipantAsync;

    /// Async version of [`get_type_name`](crate::topic_definition::topic::Topic::get_type_name).
    fn get_type_name(&self) -> String;

    /// Async version of [`get_name`](crate::topic_definition::topic::Topic::get_name).
    fn get_name(&self) -> String;
}
