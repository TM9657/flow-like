use core::pin::Pin;

use alloc::boxed::Box;

use crate::{
    dcps::channels::mpsc::{MpscSender, mpsc_channel},
    dds_async::subscriber_listener::SubscriberListener,
    runtime::Spawner,
};

use super::domain_participant_listener::ListenerMail;

pub struct DcpsSubscriberListener {
    sender: MpscSender<ListenerMail>,
    task: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl DcpsSubscriberListener {
    pub fn new(mut listener: impl SubscriberListener + Send + 'static) -> Self {
        let (sender, listener_receiver) = mpsc_channel();
        let task = Box::pin(async move {
            while let Some(m) = listener_receiver.receive().await {
                match m {
                    ListenerMail::DataOnReaders { the_subscriber } => {
                        listener.on_data_on_readers(the_subscriber).await;
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
                    ListenerMail::DataAvailable { the_reader: _ } => {
                        panic!("Not valid for subscriber")
                    }
                    ListenerMail::PublicationMatched {
                        the_writer: _,
                        status: _,
                    } => {
                        panic!("Not valid for subscriber")
                    }
                    ListenerMail::OfferedIncompatibleQos {
                        the_writer: _,
                        status: _,
                    } => {
                        panic!("Not valid for subscriber")
                    }
                    ListenerMail::OfferedDeadlineMissed {
                        the_writer: _,
                        status: _,
                    } => {
                        panic!("Not valid for subscriber")
                    }
                    ListenerMail::InconsistentTopic {
                        the_topic: _,
                        status: _,
                    } => {
                        panic!("Not valid for subscriber")
                    }
                }
            }
        });
        Self { sender, task }
    }

    pub fn spawn(self, spawner: &impl Spawner) -> MpscSender<ListenerMail> {
        spawner.spawn(self.task);
        self.sender
    }
}
