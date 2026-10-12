use crate::{
    dds_async::topic_description::TopicDescriptionAsync,
    domain::domain_participant::DomainParticipant,
};
use alloc::string::String;

/// This implementation block represents the TopicDescription operations for the [`Topic`].
pub trait TopicDescription {
    /// This operation returns the [`DomainParticipant`] to which the [`Topic`] belongs.
    fn get_participant(&self) -> DomainParticipant;

    /// The name of the type used to create the [`Topic`]
    fn get_type_name(&self) -> String;

    /// The name used to create the [`Topic`]
    fn get_name(&self) -> String;
}

impl TopicDescriptionAsync for &dyn TopicDescription {
    fn get_participant(&self) -> crate::dds_async::domain_participant::DomainParticipantAsync {
        TopicDescription::get_participant(*self).into()
    }

    fn get_type_name(&self) -> String {
        TopicDescription::get_type_name(*self)
    }

    fn get_name(&self) -> String {
        TopicDescription::get_name(*self)
    }
}

impl<T: TopicDescriptionAsync> TopicDescription for T {
    fn get_participant(&self) -> DomainParticipant {
        TopicDescriptionAsync::get_participant(self).into()
    }

    fn get_type_name(&self) -> String {
        TopicDescriptionAsync::get_type_name(self)
    }

    fn get_name(&self) -> String {
        TopicDescriptionAsync::get_name(self)
    }
}
