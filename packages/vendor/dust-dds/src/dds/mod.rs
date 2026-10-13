/// Contains the [`DomainParticipantFactory`](crate::domain::domain_participant_factory::DomainParticipantFactory) which is responsible for creating the
/// [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant). The [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant)
/// acts as an entry-point of the Service and a factory and contained for many of the classes that make up the Service.
pub mod domain;

/// Contains the [`Publisher`](crate::publication::publisher::Publisher) and [`DataWriter`](crate::publication::data_writer::DataWriter) classes as well as its
/// listener traits, and more generally, all that is needed on the publication side.
pub mod publication;

/// Contains the [`Subscriber`](crate::subscription::subscriber::Subscriber) and [`DataReader`](crate::subscription::data_reader::DataReader) classes as well as
/// its listener traits, and more generally, all that is needed on the subscription side.
pub mod subscription;

/// Contains the [`Topic`](crate::topic_definition::topic::Topic) class as well as its listener trait, and more generally, all that is needed
/// by the application to define topics and attach qos policies.
pub mod topic_definition;

/// Contains the [`DustDdsConfiguration`](crate::configuration::DustDdsConfiguration) struct that allow configuring the runtime options
/// of the Dust DDS systems
pub mod configuration {
    pub use crate::dds_async::configuration::*;
}

/// Classes related to the status conditions.
pub mod condition;

/// Classes related to WaitSet.
pub mod wait_set;
