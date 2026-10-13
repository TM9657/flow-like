use tracing::warn;

use super::{condition::StatusConditionAsync, subscriber::SubscriberAsync};
use crate::{
    builtin_topics::PublicationBuiltinTopicData,
    dcps::{
        channels::oneshot::oneshot,
        dcps_mail::{
            DcpsMail, MessageServiceMail, ReadMail, ReadNextInstanceMail, ReaderServiceMail,
        },
        listeners::data_reader_listener::DcpsDataReaderListener,
        status_condition::StatusConditionEntity,
    },
    dds_async::{
        data_reader_listener::DataReaderListener, domain_participant::DomainParticipantAsync,
        domain_participant_factory::DcpsSender, topic_description::TopicDescriptionAsync,
    },
    infrastructure::{
        error::DdsResult,
        instance::InstanceHandle,
        qos::{DataReaderQos, QosKind},
        sample_info::{
            ANY_INSTANCE_STATE, ANY_VIEW_STATE, InstanceStateKind, Sample, SampleStateKind,
            ViewStateKind,
        },
        status::{
            LivelinessChangedStatus, RequestedDeadlineMissedStatus, RequestedIncompatibleQosStatus,
            SampleLostStatus, SampleRejectedStatus, StatusKind, SubscriptionMatchedStatus,
        },
    },
    xtypes::type_support::TypeSupport,
};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::marker::PhantomData;

#[derive(Clone)]
struct DataReaderAsyncTopic {
    participant: DomainParticipantAsync,
    topic_name: Arc<str>,
    type_name: Arc<str>,
}

impl TopicDescriptionAsync for DataReaderAsyncTopic {
    fn get_participant(&self) -> DomainParticipantAsync {
        self.participant.clone()
    }

    fn get_type_name(&self) -> String {
        self.type_name.to_string()
    }

    fn get_name(&self) -> String {
        self.topic_name.to_string()
    }
}

/// Async version of [`DataReader`](crate::subscription::data_reader::DataReader).
pub struct DataReaderAsync<Foo> {
    handle: InstanceHandle,
    subscriber: SubscriberAsync,
    topic: DataReaderAsyncTopic,
    phantom: PhantomData<Foo>,
}

impl<Foo> DataReaderAsync<Foo> {
    pub(crate) fn new(
        handle: InstanceHandle,
        subscriber: SubscriberAsync,
        topic_name: Arc<str>,
        type_name: Arc<str>,
    ) -> Self {
        let participant = subscriber.get_participant();
        Self {
            handle,
            subscriber,
            topic: DataReaderAsyncTopic {
                participant,
                topic_name,
                type_name,
            },
            phantom: PhantomData,
        }
    }

    pub(crate) fn dcps_sender(&self) -> &DcpsSender {
        self.subscriber.dcps_sender()
    }

    pub(crate) fn change_foo_type<T>(self) -> DataReaderAsync<T> {
        DataReaderAsync {
            handle: self.handle,
            subscriber: self.subscriber,
            topic: self.topic,
            phantom: PhantomData,
        }
    }
}

impl<Foo> Clone for DataReaderAsync<Foo> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle,
            subscriber: self.subscriber.clone(),
            topic: self.topic.clone(),
            phantom: self.phantom,
        }
    }
}

impl<Foo: TypeSupport> DataReaderAsync<Foo> {
    /// Async version of [`read`](crate::subscription::data_reader::DataReader::read).
    #[tracing::instrument(skip(self))]
    pub async fn read(
        &self,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let reply = self
            .dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::Read(Box::new(
                ReadMail {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                    max_samples,
                    sample_states: sample_states.to_vec(),
                    view_states: view_states.to_vec(),
                    instance_states: instance_states.to_vec(),
                    specific_instance_handle: None,
                },
            ))))
            .await?;
        let samples = reply.expect_samples()?;

        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`take`](crate::subscription::data_reader::DataReader::take).
    #[tracing::instrument(skip(self))]
    pub async fn take(
        &self,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let reply = self
            .dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::Take(Box::new(
                ReadMail {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                    max_samples,
                    sample_states: sample_states.to_vec(),
                    view_states: view_states.to_vec(),
                    instance_states: instance_states.to_vec(),
                    specific_instance_handle: None,
                },
            ))))
            .await?;
        let samples = reply.expect_samples()?;

        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`read_next_sample`](crate::subscription::data_reader::DataReader::read_next_sample).
    #[tracing::instrument(skip(self))]
    pub async fn read_next_sample(&self) -> DdsResult<Sample<Foo>> {
        let reply = self
            .dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::Read(Box::new(
                ReadMail {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                    max_samples: 1,
                    sample_states: vec![SampleStateKind::NotRead],
                    view_states: ANY_VIEW_STATE.to_vec(),
                    instance_states: ANY_INSTANCE_STATE.to_vec(),
                    specific_instance_handle: None,
                },
            ))))
            .await?;
        let mut samples = reply.expect_samples()?;
        let (data, sample_info) = samples.pop().expect("Would return NoData if empty");
        Ok(Sample::new(data, sample_info))
    }

    /// Async version of [`take_next_sample`](crate::subscription::data_reader::DataReader::take_next_sample).
    #[tracing::instrument(skip(self))]
    pub async fn take_next_sample(&self) -> DdsResult<Sample<Foo>> {
        let reply = self
            .dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::Take(Box::new(
                ReadMail {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                    max_samples: 1,
                    sample_states: vec![SampleStateKind::NotRead],
                    view_states: ANY_VIEW_STATE.to_vec(),
                    instance_states: ANY_INSTANCE_STATE.to_vec(),
                    specific_instance_handle: None,
                },
            ))))
            .await?;
        let mut samples = reply.expect_samples()?;
        let (data, sample_info) = samples.pop().expect("Would return NoData if empty");
        Ok(Sample::new(data, sample_info))
    }

    /// Async version of [`read_instance`](crate::subscription::data_reader::DataReader::read_instance).
    #[tracing::instrument(skip(self))]
    pub async fn read_instance(
        &self,
        max_samples: i32,
        a_handle: InstanceHandle,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let reply = self
            .dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::Read(Box::new(
                ReadMail {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                    max_samples,
                    sample_states: sample_states.to_vec(),
                    view_states: view_states.to_vec(),
                    instance_states: instance_states.to_vec(),
                    specific_instance_handle: Some(a_handle),
                },
            ))))
            .await?;
        let samples = reply.expect_samples()?;
        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`take_instance`](crate::subscription::data_reader::DataReader::take_instance).
    #[tracing::instrument(skip(self))]
    pub async fn take_instance(
        &self,
        max_samples: i32,
        a_handle: InstanceHandle,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let reply = self
            .dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::Take(Box::new(
                ReadMail {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                    max_samples,
                    sample_states: sample_states.to_vec(),
                    view_states: view_states.to_vec(),
                    instance_states: instance_states.to_vec(),
                    specific_instance_handle: Some(a_handle),
                },
            ))))
            .await?;
        let samples = reply.expect_samples()?;

        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`read_next_instance`](crate::subscription::data_reader::DataReader::read_next_instance).
    #[tracing::instrument(skip(self))]
    pub async fn read_next_instance(
        &self,
        max_samples: i32,
        previous_handle: Option<InstanceHandle>,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let reply = self
            .dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::ReadNextInstance(
                Box::new(ReadNextInstanceMail {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                    max_samples,
                    previous_handle,
                    sample_states: sample_states.to_vec(),
                    view_states: view_states.to_vec(),
                    instance_states: instance_states.to_vec(),
                }),
            )))
            .await?;
        let samples = reply.expect_samples()?;
        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`take_next_instance`](crate::subscription::data_reader::DataReader::take_next_instance).
    #[tracing::instrument(skip(self))]
    pub async fn take_next_instance(
        &self,
        max_samples: i32,
        previous_handle: Option<InstanceHandle>,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let reply = self
            .dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::TakeNextInstance(
                Box::new(ReadNextInstanceMail {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                    max_samples,
                    previous_handle,
                    sample_states: sample_states.to_vec(),
                    view_states: view_states.to_vec(),
                    instance_states: instance_states.to_vec(),
                }),
            )))
            .await?;
        let samples = reply.expect_samples()?;
        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`get_key_value`](crate::subscription::data_reader::DataReader::get_key_value).
    #[tracing::instrument(skip(self, _key_holder))]
    pub async fn get_key_value(
        &self,
        _key_holder: &mut Foo,
        _handle: InstanceHandle,
    ) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`lookup_instance`](crate::subscription::data_reader::DataReader::lookup_instance).
    #[tracing::instrument(skip(self, _instance))]
    pub async fn lookup_instance(&self, _instance: &Foo) -> DdsResult<Option<InstanceHandle>> {
        todo!()
    }
}

impl<Foo> DataReaderAsync<Foo> {
    /// Async version of [`get_liveliness_changed_status`](crate::subscription::data_reader::DataReader::get_liveliness_changed_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_liveliness_changed_status(&self) -> DdsResult<LivelinessChangedStatus> {
        todo!()
    }

    /// Async version of [`get_requested_deadline_missed_status`](crate::subscription::data_reader::DataReader::get_requested_deadline_missed_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_requested_deadline_missed_status(
        &self,
    ) -> DdsResult<RequestedDeadlineMissedStatus> {
        todo!()
    }

    /// Async version of [`get_requested_incompatible_qos_status`](crate::subscription::data_reader::DataReader::get_requested_incompatible_qos_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_requested_incompatible_qos_status(
        &self,
    ) -> DdsResult<RequestedIncompatibleQosStatus> {
        todo!()
    }

    /// Async version of [`get_sample_lost_status`](crate::subscription::data_reader::DataReader::get_sample_lost_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_sample_lost_status(&self) -> DdsResult<SampleLostStatus> {
        todo!()
    }

    /// Async version of [`get_sample_rejected_status`](crate::subscription::data_reader::DataReader::get_sample_rejected_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_sample_rejected_status(&self) -> DdsResult<SampleRejectedStatus> {
        todo!()
    }

    /// Async version of [`get_subscription_matched_status`](crate::subscription::data_reader::DataReader::get_subscription_matched_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_subscription_matched_status(&self) -> DdsResult<SubscriptionMatchedStatus> {
        self.dcps_sender()
            .call(DcpsMail::Reader(
                ReaderServiceMail::GetSubscriptionMatchedStatus {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                },
            ))
            .await?
            .expect_subscription_matched_status()
    }

    /// Async version of [`get_topicdescription`](crate::subscription::data_reader::DataReader::get_topicdescription).
    #[tracing::instrument(skip(self))]
    pub fn get_topicdescription(&self) -> impl TopicDescriptionAsync {
        self.topic.clone()
    }

    /// Async version of [`get_subscriber`](crate::subscription::data_reader::DataReader::get_subscriber).
    #[tracing::instrument(skip(self))]
    pub fn get_subscriber(&self) -> SubscriberAsync {
        self.subscriber.clone()
    }

    /// Async version of [`wait_for_historical_data`](crate::subscription::data_reader::DataReader::wait_for_historical_data).
    /// This method does not internally wait for a maximum timeout and that is expected
    /// to be handle on the user side if needed.
    #[tracing::instrument(skip(self))]
    pub async fn wait_for_historical_data(&self) -> DdsResult<()> {
        let (reply_sender, reply_receiver) = oneshot();
        self.dcps_sender()
            .call(DcpsMail::Message(
                MessageServiceMail::NotifyHistoricalData {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                    reply_sender,
                },
            ))
            .await?;
        reply_receiver.await?
    }

    /// Async version of [`get_matched_publication_data`](crate::subscription::data_reader::DataReader::get_matched_publication_data).
    #[tracing::instrument(skip(self))]
    pub async fn get_matched_publication_data(
        &self,
        publication_handle: InstanceHandle,
    ) -> DdsResult<PublicationBuiltinTopicData> {
        self.dcps_sender()
            .call(DcpsMail::Reader(
                ReaderServiceMail::GetMatchedPublicationData {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                    publication_handle,
                },
            ))
            .await?
            .expect_publication_builtin_topic_data()
    }

    /// Async version of [`get_matched_publications`](crate::subscription::data_reader::DataReader::get_matched_publications).
    #[tracing::instrument(skip(self))]
    pub async fn get_matched_publications(&self) -> DdsResult<Vec<InstanceHandle>> {
        self.dcps_sender()
            .call(DcpsMail::Reader(
                ReaderServiceMail::GetMatchedPublications {
                    participant_handle: self.subscriber.get_participant().get_instance_handle(),
                    subscriber_handle: self.subscriber.get_instance_handle(),
                    data_reader_handle: self.handle,
                },
            ))
            .await?
            .expect_instance_handle_list()
    }
}

impl<Foo> DataReaderAsync<Foo> {
    /// Async version of [`set_qos`](crate::subscription::data_reader::DataReader::set_qos).
    pub async fn set_qos(&self, qos: QosKind<DataReaderQos>) -> DdsResult<()> {
        self.dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::SetQos {
                participant_handle: self.subscriber.get_participant().get_instance_handle(),
                subscriber_handle: self.subscriber.get_instance_handle(),
                data_reader_handle: self.handle,
                qos: Box::new(qos),
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_qos`](crate::subscription::data_reader::DataReader::get_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_qos(&self) -> DdsResult<DataReaderQos> {
        self.dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::GetQos {
                participant_handle: self.subscriber.get_participant().get_instance_handle(),
                subscriber_handle: self.subscriber.get_instance_handle(),
                data_reader_handle: self.handle,
            }))
            .await?
            .expect_data_reader_qos()
    }

    /// Async version of [`get_statuscondition`](crate::subscription::data_reader::DataReader::get_statuscondition).
    #[tracing::instrument(skip(self))]
    pub fn get_statuscondition(&self) -> StatusConditionAsync {
        StatusConditionAsync::new(
            self.dcps_sender().clone(),
            StatusConditionEntity::DataReader {
                participant_handle: self
                    .get_subscriber()
                    .get_participant()
                    .get_instance_handle(),
                subscriber_handle: self.get_subscriber().get_instance_handle(),
                reader_handle: self.handle,
            },
        )
    }

    /// Async version of [`get_status_changes`](crate::subscription::data_reader::DataReader::get_status_changes).
    #[tracing::instrument(skip(self))]
    pub async fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        todo!()
    }

    /// Async version of [`enable`](crate::subscription::data_reader::DataReader::enable).
    #[tracing::instrument(skip(self))]
    pub async fn enable(&self) -> DdsResult<()> {
        self.dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::Enable {
                participant_handle: self.subscriber.get_participant().get_instance_handle(),
                subscriber_handle: self.subscriber.get_instance_handle(),
                data_reader_handle: self.handle,
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_instance_handle`](crate::subscription::data_reader::DataReader::get_instance_handle).
    #[tracing::instrument(skip(self))]
    pub fn get_instance_handle(&self) -> InstanceHandle {
        self.handle
    }
}

impl<Foo> DataReaderAsync<Foo> {
    /// Async version of [`set_listener`](crate::subscription::data_reader::DataReader::set_listener).
    #[tracing::instrument(skip(self, a_listener))]
    pub async fn set_listener(
        &self,
        a_listener: Option<impl DataReaderListener<Foo> + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        let dcps_listener = a_listener.map(DcpsDataReaderListener::new);
        self.dcps_sender()
            .call(DcpsMail::Reader(ReaderServiceMail::SetListener {
                participant_handle: self.subscriber.get_participant().get_instance_handle(),
                subscriber_handle: self.subscriber.get_instance_handle(),
                data_reader_handle: self.handle,
                dcps_listener,
                listener_mask: mask.iter().collect(),
            }))
            .await?
            .expect_ok()
    }
}
