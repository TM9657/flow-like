use super::{
    data_writer::DataWriterAsync, data_writer_listener::DataWriterListener,
    domain_participant::DomainParticipantAsync, publisher_listener::PublisherListener,
};
use crate::{
    dcps::{
        dcps_mail::{CreateDataWriterMail, DcpsMail, PublisherServiceMail},
        listeners::{
            data_writer_listener::DcpsDataWriterListener, publisher_listener::DcpsPublisherListener,
        },
    },
    dds_async::{
        domain_participant_factory::DcpsSender, topic::TopicAsync,
        topic_description::TopicDescriptionAsync,
    },
    infrastructure::{
        error::DdsResult,
        instance::InstanceHandle,
        qos::{DataWriterQos, PublisherQos, QosKind, TopicQos},
        status::StatusKind,
        time::Duration,
    },
};
use alloc::{boxed::Box, vec::Vec};

/// Async version of [`Publisher`](crate::publication::publisher::Publisher).
#[derive(Clone)]
pub struct PublisherAsync {
    handle: InstanceHandle,
    participant: DomainParticipantAsync,
}

impl PublisherAsync {
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

impl PublisherAsync {
    /// Async version of [`create_datawriter`](crate::publication::publisher::Publisher::create_datawriter).
    #[tracing::instrument(skip(self, a_topic, a_listener))]
    pub async fn create_datawriter<Foo>(
        &self,
        a_topic: &TopicAsync,
        qos: QosKind<DataWriterQos>,
        a_listener: Option<impl DataWriterListener<Foo> + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<DataWriterAsync<Foo>> {
        let topic_name = a_topic.get_name();
        let dcps_listener = a_listener.map(DcpsDataWriterListener::new);
        let reply = self
            .dcps_sender()
            .call(DcpsMail::Publisher(PublisherServiceMail::CreateDataWriter(
                Box::new(CreateDataWriterMail {
                    participant_handle: self.participant.get_instance_handle(),
                    publisher_handle: self.handle,
                    topic_name,
                    qos,
                    dcps_listener,
                    listener_mask: mask.iter().collect(),
                }),
            )))
            .await?;
        let guid = reply.expect_instance_handle()?;

        Ok(DataWriterAsync::new(guid, self.clone(), a_topic.clone()))
    }

    /// Async version of [`delete_datawriter`](crate::publication::publisher::Publisher::delete_datawriter).
    #[tracing::instrument(skip(self, a_datawriter))]
    pub async fn delete_datawriter<Foo>(
        &self,
        a_datawriter: &DataWriterAsync<Foo>,
    ) -> DdsResult<()> {
        self.dcps_sender()
            .call(DcpsMail::Publisher(
                PublisherServiceMail::DeleteDataWriter {
                    participant_handle: self.participant.get_instance_handle(),
                    publisher_handle: self.handle,
                    datawriter_handle: a_datawriter.get_instance_handle(),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`delete_datawriter`](crate::publication::publisher::Publisher::lookup_datawriter).
    #[tracing::instrument(skip(self))]
    pub async fn lookup_datawriter<Foo>(
        &self,
        topic_name: &str,
    ) -> DdsResult<Option<DataWriterAsync<Foo>>> {
        todo!()
    }

    /// Async version of [`delete_datawriter`](crate::publication::publisher::Publisher::suspend_publications).
    #[tracing::instrument(skip(self))]
    pub async fn suspend_publications(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`delete_datawriter`](crate::publication::publisher::Publisher::resume_publications).
    #[tracing::instrument(skip(self))]
    pub async fn resume_publications(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`delete_datawriter`](crate::publication::publisher::Publisher::begin_coherent_changes).
    #[tracing::instrument(skip(self))]
    pub async fn begin_coherent_changes(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`delete_datawriter`](crate::publication::publisher::Publisher::end_coherent_changes).
    #[tracing::instrument(skip(self))]
    pub async fn end_coherent_changes(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`delete_datawriter`](crate::publication::publisher::Publisher::wait_for_acknowledgments).
    #[tracing::instrument(skip(self))]
    pub async fn wait_for_acknowledgments(&self, _max_wait: Duration) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`get_participant`](crate::publication::publisher::Publisher::get_participant).
    #[tracing::instrument(skip(self))]
    pub fn get_participant(&self) -> DomainParticipantAsync {
        self.participant.clone()
    }

    /// Async version of [`delete_contained_entities`](crate::publication::publisher::Publisher::delete_contained_entities).
    #[tracing::instrument(skip(self))]
    pub async fn delete_contained_entities(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`set_default_datawriter_qos`](crate::publication::publisher::Publisher::set_default_datawriter_qos).
    #[tracing::instrument(skip(self))]
    pub async fn set_default_datawriter_qos(&self, qos: QosKind<DataWriterQos>) -> DdsResult<()> {
        self.dcps_sender()
            .call(DcpsMail::Publisher(
                PublisherServiceMail::SetDefaultDataWriterQos {
                    participant_handle: self.participant.get_instance_handle(),
                    publisher_handle: self.handle,
                    qos: Box::new(qos),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_default_datawriter_qos`](crate::publication::publisher::Publisher::get_default_datawriter_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_default_datawriter_qos(&self) -> DdsResult<DataWriterQos> {
        self.dcps_sender()
            .call(DcpsMail::Publisher(
                PublisherServiceMail::GetDefaultDataWriterQos {
                    participant_handle: self.participant.get_instance_handle(),
                    publisher_handle: self.handle,
                },
            ))
            .await?
            .expect_data_writer_qos()
    }

    /// Async version of [`copy_from_topic_qos`](crate::publication::publisher::Publisher::copy_from_topic_qos).
    #[tracing::instrument(skip(self))]
    pub async fn copy_from_topic_qos(
        &self,
        _a_datawriter_qos: &mut DataWriterQos,
        _a_topic_qos: &TopicQos,
    ) -> DdsResult<()> {
        todo!()
    }
}

impl PublisherAsync {
    /// Async version of [`set_qos`](crate::publication::publisher::Publisher::set_qos).
    #[tracing::instrument(skip(self))]
    pub async fn set_qos(&self, qos: QosKind<PublisherQos>) -> DdsResult<()> {
        self.dcps_sender()
            .call(DcpsMail::Publisher(PublisherServiceMail::SetPublisherQos {
                participant_handle: self.participant.get_instance_handle(),
                publisher_handle: self.handle,
                qos: Box::new(qos),
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_qos`](crate::publication::publisher::Publisher::get_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_qos(&self) -> DdsResult<PublisherQos> {
        self.dcps_sender()
            .call(DcpsMail::Publisher(PublisherServiceMail::GetPublisherQos {
                participant_handle: self.participant.get_instance_handle(),
                publisher_handle: self.handle,
            }))
            .await?
            .expect_publisher_qos()
    }

    /// Async version of [`set_listener`](crate::publication::publisher::Publisher::set_listener).
    #[tracing::instrument(skip(self, a_listener))]
    pub async fn set_listener(
        &self,
        a_listener: Option<impl PublisherListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        let dcps_listener = a_listener.map(DcpsPublisherListener::new);
        self.dcps_sender()
            .call(DcpsMail::Publisher(
                PublisherServiceMail::SetPublisherListener {
                    participant_handle: self.participant.get_instance_handle(),
                    publisher_handle: self.handle,
                    dcps_listener,
                    listener_mask: mask.iter().collect(),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_status_changes`](crate::publication::publisher::Publisher::get_status_changes).
    #[tracing::instrument(skip(self))]
    pub async fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        todo!()
    }

    /// Async version of [`enable`](crate::publication::publisher::Publisher::enable).
    #[tracing::instrument(skip(self))]
    pub async fn enable(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`get_instance_handle`](crate::publication::publisher::Publisher::get_instance_handle).
    #[tracing::instrument(skip(self))]
    pub fn get_instance_handle(&self) -> InstanceHandle {
        self.handle
    }
}
