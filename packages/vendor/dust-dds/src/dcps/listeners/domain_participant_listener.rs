use core::pin::Pin;

use alloc::boxed::Box;

use crate::{
    dcps::channels::mpsc::{MpscSender, mpsc_channel},
    dds_async::{
        data_reader::DataReaderAsync, data_writer::DataWriterAsync,
        domain_participant_listener::DomainParticipantListener, subscriber::SubscriberAsync,
        topic::TopicAsync,
    },
    infrastructure::status::{
        InconsistentTopicStatus, OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus,
        PublicationMatchedStatus, RequestedDeadlineMissedStatus, RequestedIncompatibleQosStatus,
        SampleRejectedStatus, SubscriptionMatchedStatus,
    },
    runtime::Spawner,
};

pub struct DcpsDomainParticipantListener {
    sender: MpscSender<ListenerMail>,
    task: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl DcpsDomainParticipantListener {
    pub fn new(mut listener: impl DomainParticipantListener + Send + 'static) -> Self {
        let (sender, listener_receiver) = mpsc_channel();
        let task = Box::pin(async move {
            while let Some(m) = listener_receiver.receive().await {
                match m {
                    ListenerMail::DataAvailable { the_reader } => {
                        listener.on_data_available(the_reader).await;
                    }
                    ListenerMail::RequestedDeadlineMissed { the_reader, status } => {
                        listener
                            .on_requested_deadline_missed(the_reader, status)
                            .await;
                    }
                    ListenerMail::SampleRejected { the_reader, status } => {
                        listener.on_sample_rejected(the_reader, status).await;
                    }
                    ListenerMail::SubscriptionMatched { the_reader, status } => {
                        listener.on_subscription_matched(the_reader, status).await;
                    }
                    ListenerMail::RequestedIncompatibleQos { the_reader, status } => {
                        listener
                            .on_requested_incompatible_qos(the_reader, status)
                            .await;
                    }
                    ListenerMail::PublicationMatched { the_writer, status } => {
                        listener.on_publication_matched(the_writer, status).await;
                    }
                    ListenerMail::OfferedIncompatibleQos { the_writer, status } => {
                        listener
                            .on_offered_incompatible_qos(the_writer, status)
                            .await;
                    }
                    ListenerMail::OfferedDeadlineMissed { the_writer, status } => {
                        listener
                            .on_offered_deadline_missed(the_writer, status)
                            .await;
                    }
                    ListenerMail::InconsistentTopic { the_topic, status } => {
                        listener.on_inconsistent_topic(the_topic, status).await;
                    }
                    ListenerMail::DataOnReaders { the_subscriber: _ } => {
                        panic!("Not valid for domain participant")
                    }
                }
            }
        });
        Self { sender, task }
    }

    pub fn spawn(self, spawner_handle: &impl Spawner) -> MpscSender<ListenerMail> {
        spawner_handle.spawn(self.task);
        self.sender
    }
}

pub enum ListenerMail {
    DataAvailable {
        the_reader: DataReaderAsync<()>,
    },
    DataOnReaders {
        the_subscriber: SubscriberAsync,
    },
    RequestedDeadlineMissed {
        the_reader: DataReaderAsync<()>,
        status: RequestedDeadlineMissedStatus,
    },
    SampleRejected {
        the_reader: DataReaderAsync<()>,
        status: SampleRejectedStatus,
    },
    SubscriptionMatched {
        the_reader: DataReaderAsync<()>,
        status: SubscriptionMatchedStatus,
    },
    RequestedIncompatibleQos {
        the_reader: DataReaderAsync<()>,
        status: RequestedIncompatibleQosStatus,
    },
    PublicationMatched {
        the_writer: DataWriterAsync<()>,
        status: PublicationMatchedStatus,
    },
    OfferedIncompatibleQos {
        the_writer: DataWriterAsync<()>,
        status: OfferedIncompatibleQosStatus,
    },
    OfferedDeadlineMissed {
        the_writer: DataWriterAsync<()>,
        status: OfferedDeadlineMissedStatus,
    },
    InconsistentTopic {
        the_topic: TopicAsync,
        status: InconsistentTopicStatus,
    },
}
