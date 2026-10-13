use tracing::warn;

use super::{condition::StatusConditionAsync, publisher::PublisherAsync};
use crate::{
    builtin_topics::SubscriptionBuiltinTopicData,
    dcps::{
        channels::oneshot::oneshot,
        dcps_mail::{DcpsMail, MessageServiceMail, WriterServiceMail},
        listeners::data_writer_listener::DcpsDataWriterListener,
        status_condition::StatusConditionEntity,
    },
    dds_async::{
        data_writer_listener::DataWriterListener, domain_participant_factory::DcpsSender,
        topic::TopicAsync,
    },
    infrastructure::{
        error::DdsResult,
        instance::InstanceHandle,
        qos::{DataWriterQos, QosKind},
        status::{
            LivelinessLostStatus, OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus,
            PublicationMatchedStatus, StatusKind,
        },
        time::Time,
    },
    xtypes::type_support::TypeSupport,
};
use alloc::{boxed::Box, vec::Vec};
use core::marker::PhantomData;

/// Async version of [`DataWriter`](crate::publication::data_writer::DataWriter).
pub struct DataWriterAsync<Foo> {
    handle: InstanceHandle,
    publisher: PublisherAsync,
    topic: TopicAsync,
    phantom: PhantomData<Foo>,
}

impl<Foo> Clone for DataWriterAsync<Foo> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle,
            publisher: self.publisher.clone(),
            topic: self.topic.clone(),
            phantom: self.phantom,
        }
    }
}

impl<Foo> DataWriterAsync<Foo> {
    pub(crate) fn new(
        handle: InstanceHandle,
        publisher: PublisherAsync,
        topic: TopicAsync,
    ) -> Self {
        Self {
            handle,
            publisher,
            topic,
            phantom: PhantomData,
        }
    }

    pub(crate) fn dcps_sender(&self) -> &DcpsSender {
        self.publisher.dcps_sender()
    }

    pub(crate) fn change_foo_type<T>(self) -> DataWriterAsync<T> {
        DataWriterAsync {
            handle: self.handle,
            publisher: self.publisher,
            topic: self.topic,
            phantom: PhantomData,
        }
    }
}

impl<Foo> DataWriterAsync<Foo>
where
    Foo: TypeSupport,
{
    /// Async version of [`register_instance`](crate::publication::data_writer::DataWriter::register_instance).
    #[tracing::instrument(skip(self, instance))]
    pub async fn register_instance(&self, instance: Foo) -> DdsResult<Option<InstanceHandle>> {
        let dynamic_data = instance.create_dynamic_sample();
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::RegisterInstance {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                dynamic_data,
                timestamp: None,
            }))
            .await?
            .expect_option_instance_handle()
    }

    /// Async version of [`register_instance_w_timestamp`](crate::publication::data_writer::DataWriter::register_instance_w_timestamp).
    #[tracing::instrument(skip(self, instance))]
    pub async fn register_instance_w_timestamp(
        &self,
        instance: Foo,
        timestamp: Time,
    ) -> DdsResult<Option<InstanceHandle>> {
        let dynamic_data = instance.create_dynamic_sample();
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::RegisterInstance {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                dynamic_data,
                timestamp: Some(timestamp),
            }))
            .await?
            .expect_option_instance_handle()
    }

    /// Async version of [`unregister_instance`](crate::publication::data_writer::DataWriter::unregister_instance).
    #[tracing::instrument(skip(self, instance))]
    pub async fn unregister_instance(
        &self,
        instance: Foo,
        handle: Option<InstanceHandle>,
    ) -> DdsResult<()> {
        let dynamic_data = instance.create_dynamic_sample();
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::UnregisterInstance {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                dynamic_data,
                timestamp: None,
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`unregister_instance_w_timestamp`](crate::publication::data_writer::DataWriter::unregister_instance_w_timestamp).
    #[tracing::instrument(skip(self, instance))]
    pub async fn unregister_instance_w_timestamp(
        &self,
        instance: Foo,
        handle: Option<InstanceHandle>,
        timestamp: Time,
    ) -> DdsResult<()> {
        let dynamic_data = instance.create_dynamic_sample();
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::UnregisterInstance {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                dynamic_data,
                timestamp: Some(timestamp),
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_key_value`](crate::publication::data_writer::DataWriter::get_key_value).
    #[tracing::instrument(skip(self, _key_holder))]
    pub async fn get_key_value(
        &self,
        _key_holder: &mut Foo,
        _handle: InstanceHandle,
    ) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`lookup_instance`](crate::publication::data_writer::DataWriter::lookup_instance).
    #[tracing::instrument(skip(self, instance))]
    pub async fn lookup_instance(&self, instance: Foo) -> DdsResult<Option<InstanceHandle>> {
        let dynamic_data = instance.create_dynamic_sample();
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::LookupInstance {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                dynamic_data,
            }))
            .await?
            .expect_option_instance_handle()
    }

    /// Async version of [`write`](crate::publication::data_writer::DataWriter::write).
    #[tracing::instrument(skip(self, data))]
    pub async fn write(&self, data: Foo, handle: Option<InstanceHandle>) -> DdsResult<()> {
        let (reply_sender, reply_receiver) = oneshot();
        let dynamic_data = data.create_dynamic_sample();
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::WriteWTimestamp {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                dynamic_data,
                timestamp: None,
                reply_sender,
            }))
            .await?;
        reply_receiver.await?
    }

    /// Async version of [`write_w_timestamp`](crate::publication::data_writer::DataWriter::write_w_timestamp).
    #[tracing::instrument(skip(self, data))]
    pub async fn write_w_timestamp(
        &self,
        data: Foo,
        handle: Option<InstanceHandle>,
        timestamp: Time,
    ) -> DdsResult<()> {
        let (reply_sender, reply_receiver) = oneshot();
        let dynamic_data = data.create_dynamic_sample();
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::WriteWTimestamp {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                dynamic_data,
                timestamp: Some(timestamp),
                reply_sender,
            }))
            .await?;
        reply_receiver.await?
    }

    /// Async version of [`dispose`](crate::publication::data_writer::DataWriter::dispose).
    #[tracing::instrument(skip(self, data))]
    pub async fn dispose(&self, data: Foo, handle: Option<InstanceHandle>) -> DdsResult<()> {
        let dynamic_data = data.create_dynamic_sample();
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::DisposeWTimestamp {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                dynamic_data,
                timestamp: None,
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`dispose_w_timestamp`](crate::publication::data_writer::DataWriter::dispose_w_timestamp).
    #[tracing::instrument(skip(self, data))]
    pub async fn dispose_w_timestamp(
        &self,
        data: Foo,
        handle: Option<InstanceHandle>,
        timestamp: Time,
    ) -> DdsResult<()> {
        let dynamic_data = data.create_dynamic_sample();
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::DisposeWTimestamp {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                dynamic_data,
                timestamp: Some(timestamp),
            }))
            .await?
            .expect_ok()
    }
}

impl<Foo> DataWriterAsync<Foo> {
    /// Async version of [`wait_for_acknowledgments`](crate::publication::data_writer::DataWriter::wait_for_acknowledgments).
    /// This method does not internally wait for a maximum timeout and that is expected
    /// to be handle on the user side if needed.
    #[tracing::instrument(skip(self))]
    pub async fn wait_for_acknowledgments(&self) -> DdsResult<()> {
        let participant_address = self.dcps_sender().clone();
        let publisher_handle = self.get_publisher().get_instance_handle();
        let data_writer_handle = self.handle;
        let (reply_sender, reply_receiver) = oneshot();
        participant_address
            .call(DcpsMail::Message(
                MessageServiceMail::NotifyAcknowledgments {
                    participant_handle: self.publisher.get_participant().get_instance_handle(),
                    publisher_handle,
                    data_writer_handle,
                    reply_sender,
                },
            ))
            .await?;
        reply_receiver.await?
    }

    /// Async version of [`get_liveliness_lost_status`](crate::publication::data_writer::DataWriter::get_liveliness_lost_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_liveliness_lost_status(&self) -> DdsResult<LivelinessLostStatus> {
        todo!()
    }

    /// Async version of [`get_offered_deadline_missed_status`](crate::publication::data_writer::DataWriter::get_offered_deadline_missed_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_offered_deadline_missed_status(
        &self,
    ) -> DdsResult<OfferedDeadlineMissedStatus> {
        self.dcps_sender()
            .call(DcpsMail::Writer(
                WriterServiceMail::GetOfferedDeadlineMissedStatus {
                    participant_handle: self.publisher.get_participant().get_instance_handle(),
                    publisher_handle: self.publisher.get_instance_handle(),
                    data_writer_handle: self.handle,
                },
            ))
            .await?
            .expect_offered_deadline_missed_status()
    }

    /// Async version of [`get_offered_incompatible_qos_status`](crate::publication::data_writer::DataWriter::get_offered_incompatible_qos_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_offered_incompatible_qos_status(
        &self,
    ) -> DdsResult<OfferedIncompatibleQosStatus> {
        todo!()
    }

    /// Async version of [`get_publication_matched_status`](crate::publication::data_writer::DataWriter::get_publication_matched_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_publication_matched_status(&self) -> DdsResult<PublicationMatchedStatus> {
        self.dcps_sender()
            .call(DcpsMail::Writer(
                WriterServiceMail::GetPublicationMatchedStatus {
                    participant_handle: self.publisher.get_participant().get_instance_handle(),
                    publisher_handle: self.publisher.get_instance_handle(),
                    data_writer_handle: self.handle,
                },
            ))
            .await?
            .expect_publication_matched_status()
    }

    /// Async version of [`get_topic`](crate::publication::data_writer::DataWriter::get_topic).
    #[tracing::instrument(skip(self))]
    pub fn get_topic(&self) -> TopicAsync {
        self.topic.clone()
    }

    /// Async version of [`get_publisher`](crate::publication::data_writer::DataWriter::get_publisher).
    #[tracing::instrument(skip(self))]
    pub fn get_publisher(&self) -> PublisherAsync {
        self.publisher.clone()
    }

    /// Async version of [`assert_liveliness`](crate::publication::data_writer::DataWriter::assert_liveliness).
    #[tracing::instrument(skip(self))]
    pub async fn assert_liveliness(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`get_matched_subscription_data`](crate::publication::data_writer::DataWriter::get_matched_subscription_data).
    #[tracing::instrument(skip(self))]
    pub async fn get_matched_subscription_data(
        &self,
        subscription_handle: InstanceHandle,
    ) -> DdsResult<SubscriptionBuiltinTopicData> {
        self.dcps_sender()
            .call(DcpsMail::Writer(
                WriterServiceMail::GetMatchedSubscriptionData {
                    participant_handle: self.publisher.get_participant().get_instance_handle(),
                    publisher_handle: self.publisher.get_instance_handle(),
                    data_writer_handle: self.handle,
                    subscription_handle,
                },
            ))
            .await?
            .expect_subscription_builtin_topic_data()
    }

    /// Async version of [`get_matched_subscriptions`](crate::publication::data_writer::DataWriter::get_matched_subscriptions).
    #[tracing::instrument(skip(self))]
    pub async fn get_matched_subscriptions(&self) -> DdsResult<Vec<InstanceHandle>> {
        self.dcps_sender()
            .call(DcpsMail::Writer(
                WriterServiceMail::GetMatchedSubscriptions {
                    participant_handle: self.publisher.get_participant().get_instance_handle(),
                    publisher_handle: self.publisher.get_instance_handle(),
                    data_writer_handle: self.handle,
                },
            ))
            .await?
            .expect_instance_handle_list()
    }
}

impl<Foo> DataWriterAsync<Foo> {
    /// Async version of [`set_qos`](crate::publication::data_writer::DataWriter::set_qos).
    #[tracing::instrument(skip(self))]
    pub async fn set_qos(&self, qos: QosKind<DataWriterQos>) -> DdsResult<()> {
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::SetDataWriterQos {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                qos: Box::new(qos),
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_qos`](crate::publication::data_writer::DataWriter::get_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_qos(&self) -> DdsResult<DataWriterQos> {
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::GetDataWriterQos {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
            }))
            .await?
            .expect_data_writer_qos()
    }

    /// Async version of [`get_statuscondition`](crate::publication::data_writer::DataWriter::get_statuscondition).
    #[tracing::instrument(skip(self))]
    pub fn get_statuscondition(&self) -> StatusConditionAsync {
        StatusConditionAsync::new(
            self.dcps_sender().clone(),
            StatusConditionEntity::DataWriter {
                participant_handle: self.get_publisher().get_participant().get_instance_handle(),
                publisher_handle: self.get_publisher().get_instance_handle(),
                writer_handle: self.handle,
            },
        )
    }

    /// Async version of [`get_status_changes`](crate::publication::data_writer::DataWriter::get_status_changes).
    #[tracing::instrument(skip(self))]
    pub async fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        todo!()
    }

    /// Async version of [`enable`](crate::publication::data_writer::DataWriter::enable).
    #[tracing::instrument(skip(self))]
    pub async fn enable(&self) -> DdsResult<()> {
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::EnableDataWriter {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_instance_handle`](crate::publication::data_writer::DataWriter::get_instance_handle).
    #[tracing::instrument(skip(self))]
    pub fn get_instance_handle(&self) -> InstanceHandle {
        self.handle
    }
}
impl<Foo> DataWriterAsync<Foo> {
    /// Async version of [`set_listener`](crate::publication::data_writer::DataWriter::set_listener).
    #[tracing::instrument(skip(self, a_listener))]
    pub async fn set_listener(
        &self,
        a_listener: Option<impl DataWriterListener<Foo> + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        let dcps_listener = a_listener.map(DcpsDataWriterListener::new);
        self.dcps_sender()
            .call(DcpsMail::Writer(WriterServiceMail::SetListener {
                participant_handle: self.publisher.get_participant().get_instance_handle(),
                publisher_handle: self.publisher.get_instance_handle(),
                data_writer_handle: self.handle,
                dcps_listener,
                listener_mask: mask.iter().collect(),
            }))
            .await?
            .expect_ok()
    }
}
