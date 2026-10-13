use crate::dds_async::{
    data_reader_listener::DataReaderListener, data_writer_listener::DataWriterListener,
    domain_participant_listener::DomainParticipantListener, publisher_listener::PublisherListener,
    subscriber_listener::SubscriberListener, topic_listener::TopicListener,
};

/// Convinience constant to be used when the user does not want to install a listener.
pub const NO_LISTENER: Option<()> = None;

impl DomainParticipantListener for () {}

impl PublisherListener for () {}

impl SubscriberListener for () {}

impl TopicListener for () {}

impl<Foo> DataWriterListener<Foo> for () {}

impl<Foo> DataReaderListener<Foo> for () {}
