use super::{
    condition::StatusConditionAsync, data_reader::DataReaderAsync,
    data_reader_listener::DataReaderListener, domain_participant::DomainParticipantAsync,
    subscriber_listener::SubscriberListener,
};
use crate::{
    dcps::{
        dcps_mail::{CreateDataReaderMail, DcpsMail, SubscriberServiceMail},
        listeners::{
            data_reader_listener::DcpsDataReaderListener,
            subscriber_listener::DcpsSubscriberListener,
        },
        status_condition::StatusConditionEntity,
    },
    dds_async::{domain_participant_factory::DcpsSender, topic_description::TopicDescriptionAsync},
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::{DataReaderQos, QosKind, SubscriberQos, TopicQos},
        status::{SampleLostStatus, StatusKind},
    },
};
use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};

/// Async version of [`Subscriber`](crate::subscription::subscriber::Subscriber).
#[derive(Clone)]
pub struct SubscriberAsync {
    handle: InstanceHandle,
    participant: DomainParticipantAsync,
}

impl SubscriberAsync {
    pub(crate) fn new(handle: InstanceHandle, participant: DomainParticipantAsync) -> Self {
        Self {
            handle,
            participant,
        }
    }

    pub(crate) fn dcps_sender(&self) -> &DcpsSender {
        self.participant.dcps_sender()
    }
}

impl SubscriberAsync {
    /// Async version of [`create_datareader`](crate::subscription::subscriber::Subscriber::create_datareader).
    #[tracing::instrument(skip(self, a_topic, a_listener))]
    pub async fn create_datareader<Foo>(
        &self,
        a_topic: &dyn TopicDescriptionAsync,
        qos: QosKind<DataReaderQos>,
        a_listener: Option<impl DataReaderListener<Foo> + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<DataReaderAsync<Foo>> {
        let dcps_listener = a_listener.map(DcpsDataReaderListener::new);
        let reply = self
            .dcps_sender()
            .call(DcpsMail::Subscriber(
                SubscriberServiceMail::CreateDataReader(Box::new(CreateDataReaderMail {
                    participant_handle: self.participant.get_instance_handle(),
                    subscriber_handle: self.handle,
                    topic_name: a_topic.get_name(),
                    qos,
                    dcps_listener,
                    listener_mask: mask.iter().collect(),
                })),
            ))
            .await?;
        let guid = reply.expect_instance_handle()?;

        Ok(DataReaderAsync::new(
            guid,
            self.clone(),
            Arc::from(a_topic.get_name().as_str()),
            Arc::from(a_topic.get_type_name().as_str()),
        ))
    }

    /// Async version of [`delete_datareader`](crate::subscription::subscriber::Subscriber::delete_datareader).
    #[tracing::instrument(skip(self, a_datareader))]
    pub async fn delete_datareader<Foo>(
        &self,
        a_datareader: &DataReaderAsync<Foo>,
    ) -> DdsResult<()> {
        self.dcps_sender()
            .call(DcpsMail::Subscriber(
                SubscriberServiceMail::DeleteDataReader {
                    participant_handle: self.participant.get_instance_handle(),
                    subscriber_handle: self.handle,
                    datareader_handle: a_datareader.get_instance_handle(),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`lookup_datareader`](crate::subscription::subscriber::Subscriber::lookup_datareader).
    #[tracing::instrument(skip(self))]
    pub async fn lookup_datareader<Foo>(
        &self,
        topic_name: &str,
    ) -> DdsResult<Option<DataReaderAsync<Foo>>> {
        if let Some(topic) = self.participant.lookup_topicdescription(topic_name).await? {
            let reply = self
                .dcps_sender()
                .call(DcpsMail::Subscriber(
                    SubscriberServiceMail::LookupDataReader {
                        participant_handle: self.participant.get_instance_handle(),
                        subscriber_handle: self.handle,
                        topic_name: String::from(topic_name),
                    },
                ))
                .await?;
            if let Some(reader_handle) = reply.expect_option_instance_handle()? {
                Ok(Some(DataReaderAsync::new(
                    reader_handle,
                    self.clone(),
                    Arc::from(topic.get_name().as_str()),
                    Arc::from(topic.get_type_name().as_str()),
                )))
            } else {
                Ok(None)
            }
        } else {
            Err(DdsError::BadParameter)
        }
    }

    /// Async version of [`notify_datareaders`](crate::subscription::subscriber::Subscriber::notify_datareaders).
    #[tracing::instrument(skip(self))]
    pub async fn notify_datareaders(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`get_participant`](crate::subscription::subscriber::Subscriber::get_participant).
    #[tracing::instrument(skip(self))]
    pub fn get_participant(&self) -> DomainParticipantAsync {
        self.participant.clone()
    }

    /// Async version of [`get_sample_lost_status`](crate::subscription::subscriber::Subscriber::get_sample_lost_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_sample_lost_status(&self) -> DdsResult<SampleLostStatus> {
        todo!()
    }

    /// Async version of [`delete_contained_entities`](crate::subscription::subscriber::Subscriber::delete_contained_entities).
    #[tracing::instrument(skip(self))]
    pub async fn delete_contained_entities(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`set_default_datareader_qos`](crate::subscription::subscriber::Subscriber::set_default_datareader_qos).
    #[tracing::instrument(skip(self))]
    pub async fn set_default_datareader_qos(&self, qos: QosKind<DataReaderQos>) -> DdsResult<()> {
        self.dcps_sender()
            .call(DcpsMail::Subscriber(
                SubscriberServiceMail::SetDefaultDataReaderQos {
                    participant_handle: self.participant.get_instance_handle(),
                    subscriber_handle: self.handle,
                    qos: Box::new(qos),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_default_datareader_qos`](crate::subscription::subscriber::Subscriber::get_default_datareader_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_default_datareader_qos(&self) -> DdsResult<DataReaderQos> {
        self.dcps_sender()
            .call(DcpsMail::Subscriber(
                SubscriberServiceMail::GetDefaultDataReaderQos {
                    participant_handle: self.participant.get_instance_handle(),
                    subscriber_handle: self.handle,
                },
            ))
            .await?
            .expect_data_reader_qos()
    }

    /// Async version of [`copy_from_topic_qos`](crate::subscription::subscriber::Subscriber::copy_from_topic_qos).
    #[tracing::instrument]
    pub async fn copy_from_topic_qos(
        _a_datareader_qos: &mut DataReaderQos,
        _a_topic_qos: &TopicQos,
    ) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`set_qos`](crate::subscription::subscriber::Subscriber::set_qos).
    #[tracing::instrument(skip(self))]
    pub async fn set_qos(&self, qos: QosKind<SubscriberQos>) -> DdsResult<()> {
        self.dcps_sender()
            .call(DcpsMail::Subscriber(SubscriberServiceMail::SetQos {
                participant_handle: self.participant.get_instance_handle(),
                subscriber_handle: self.handle,
                qos: Box::new(qos),
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_qos`](crate::subscription::subscriber::Subscriber::get_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_qos(&self) -> DdsResult<SubscriberQos> {
        self.dcps_sender()
            .call(DcpsMail::Subscriber(
                SubscriberServiceMail::GetSubscriberQos {
                    participant_handle: self.participant.get_instance_handle(),
                    subscriber_handle: self.handle,
                },
            ))
            .await?
            .expect_subscriber_qos()
    }

    /// Async version of [`set_listener`](crate::subscription::subscriber::Subscriber::set_listener).
    #[tracing::instrument(skip(self, a_listener))]
    pub async fn set_listener(
        &self,
        a_listener: Option<impl SubscriberListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        let dcps_listener = a_listener.map(DcpsSubscriberListener::new);
        self.dcps_sender()
            .call(DcpsMail::Subscriber(SubscriberServiceMail::SetListener {
                participant_handle: self.participant.get_instance_handle(),
                subscriber_handle: self.handle,
                dcps_listener,
                listener_mask: mask.iter().collect(),
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_statuscondition`](crate::subscription::subscriber::Subscriber::get_statuscondition).
    #[tracing::instrument(skip(self))]
    pub fn get_statuscondition(&self) -> StatusConditionAsync {
        StatusConditionAsync::new(
            self.dcps_sender().clone(),
            StatusConditionEntity::Subscriber {
                participant_handle: self.get_participant().get_instance_handle(),
                subscriber_handle: self.handle,
            },
        )
    }

    /// Async version of [`get_status_changes`](crate::subscription::subscriber::Subscriber::get_status_changes).
    #[tracing::instrument(skip(self))]
    pub async fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        todo!()
    }

    /// Async version of [`enable`](crate::subscription::subscriber::Subscriber::enable).
    #[tracing::instrument(skip(self))]
    pub async fn enable(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`get_instance_handle`](crate::subscription::subscriber::Subscriber::get_instance_handle).
    #[tracing::instrument(skip(self))]
    pub fn get_instance_handle(&self) -> InstanceHandle {
        self.handle
    }
}
