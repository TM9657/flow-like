use core::pin::Pin;

use alloc::boxed::Box;

use crate::{
    dcps::channels::mpsc::{MpscSender, mpsc_channel},
    dds_async::publisher_listener::PublisherListener,
    runtime::Spawner,
};

use super::domain_participant_listener::ListenerMail;

pub struct DcpsPublisherListener {
    sender: MpscSender<ListenerMail>,
    task: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl DcpsPublisherListener {
    pub fn new(mut listener: impl PublisherListener + Send + 'static) -> Self {
        let (sender, listener_receiver) = mpsc_channel();
        let task = Box::pin(async move {
            while let Some(m) = listener_receiver.receive().await {
                match m {
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
                    ListenerMail::DataOnReaders { the_subscriber: _ } => {
                        panic!("Not valid for publisher")
                    }
                    ListenerMail::DataAvailable { the_reader: _ } => {
                        panic!("Not valid for publisher")
                    }
                    ListenerMail::RequestedDeadlineMissed {
                        the_reader: _,
                        status: _,
                    } => {
                        panic!("Not valid for publisher")
                    }
                    ListenerMail::SampleRejected {
                        the_reader: _,
                        status: _,
                    } => {
                        panic!("Not valid for publisher")
                    }
                    ListenerMail::SubscriptionMatched {
                        the_reader: _,
                        status: _,
                    } => {
                        panic!("Not valid for publisher")
                    }
                    ListenerMail::RequestedIncompatibleQos {
                        the_reader: _,
                        status: _,
                    } => {
                        panic!("Not valid for publisher")
                    }
                    ListenerMail::InconsistentTopic {
                        the_topic: _,
                        status: _,
                    } => {
                        panic!("Not valid for publisher")
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
