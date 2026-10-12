use super::{
    publisher::PublisherAsync, publisher_listener::PublisherListener, subscriber::SubscriberAsync,
    subscriber_listener::SubscriberListener, topic::TopicAsync, topic_listener::TopicListener,
};
use crate::{
    builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    dcps::{
        channels::oneshot::oneshot,
        dcps_mail::{
            CreateContentFilteredTopicMail, CreateTopicMail, DcpsMail, ParticipantServiceMail,
        },
        listeners::{
            domain_participant_listener::DcpsDomainParticipantListener,
            publisher_listener::DcpsPublisherListener, subscriber_listener::DcpsSubscriberListener,
            topic_listener::DcpsTopicListener,
        },
    },
    dds_async::{
        content_filtered_topic::ContentFilteredTopicAsync, domain_participant_factory::DcpsSender,
        domain_participant_listener::DomainParticipantListener,
        topic_description::TopicDescriptionAsync,
    },
    infrastructure::{
        domain::DomainId,
        error::DdsResult,
        instance::InstanceHandle,
        qos::{DomainParticipantQos, PublisherQos, QosKind, SubscriberQos, TopicQos},
        status::StatusKind,
        time::{Duration, Time},
    },
    xtypes::{dynamic_type::DynamicType, type_support::TypeSupport},
};

use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

/// Async version of [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant).
#[derive(Clone)]
pub struct DomainParticipantAsync {
    dcps_sender: DcpsSender,
    domain_id: DomainId,
    pub(crate) handle: InstanceHandle,
}

impl DomainParticipantAsync {
    pub(crate) fn new(
        dcps_sender: DcpsSender,
        domain_id: DomainId,
        handle: InstanceHandle,
    ) -> Self {
        Self {
            dcps_sender,
            domain_id,
            handle,
        }
    }

    pub(crate) fn dcps_sender(&self) -> &DcpsSender {
        &self.dcps_sender
    }
}

impl DomainParticipantAsync {
    /// Async version of [`create_publisher`](crate::domain::domain_participant::DomainParticipant::create_publisher).
    #[tracing::instrument(skip(self, a_listener))]
    pub async fn create_publisher(
        &self,
        qos: QosKind<PublisherQos>,
        a_listener: Option<impl PublisherListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<PublisherAsync> {
        let dcps_listener = a_listener.map(DcpsPublisherListener::new);
        let reply = self
            .dcps_sender
            .call(DcpsMail::Participant(
                crate::dcps::dcps_mail::ParticipantServiceMail::CreateUserDefinedPublisher {
                    participant_handle: self.handle,
                    qos,
                    dcps_listener,
                    listener_mask: mask.iter().collect(),
                },
            ))
            .await?;
        let guid = reply.expect_instance_handle()?;
        let publisher = PublisherAsync::new(guid, self.clone());

        Ok(publisher)
    }

    /// Async version of [`delete_publisher`](crate::domain::domain_participant::DomainParticipant::delete_publisher).
    #[tracing::instrument(skip(self, a_publisher))]
    pub async fn delete_publisher(&self, a_publisher: &PublisherAsync) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::DeleteUserDefinedPublisher {
                    participant_handle: self.handle,
                    parent_participant_handle: a_publisher.get_participant().handle,
                    publisher_handle: a_publisher.get_instance_handle(),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`create_subscriber`](crate::domain::domain_participant::DomainParticipant::create_subscriber).
    #[tracing::instrument(skip(self, a_listener))]
    pub async fn create_subscriber(
        &self,
        qos: QosKind<SubscriberQos>,
        a_listener: Option<impl SubscriberListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<SubscriberAsync> {
        let dcps_listener = a_listener.map(DcpsSubscriberListener::new);
        let reply = self
            .dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::CreateUserDefinedSubscriber {
                    participant_handle: self.handle,
                    qos,
                    dcps_listener,
                    listener_mask: mask.iter().collect(),
                },
            ))
            .await?;
        let guid = reply.expect_instance_handle()?;
        let subscriber = SubscriberAsync::new(guid, self.clone());

        Ok(subscriber)
    }

    /// Async version of [`delete_subscriber`](crate::domain::domain_participant::DomainParticipant::delete_subscriber).
    #[tracing::instrument(skip(self, a_subscriber))]
    pub async fn delete_subscriber(&self, a_subscriber: &SubscriberAsync) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::DeleteUserDefinedSubscriber {
                    participant_handle: self.handle,
                    parent_participant_handle: a_subscriber.get_participant().handle,
                    subscriber_handle: a_subscriber.get_instance_handle(),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`create_topic`](crate::domain::domain_participant::DomainParticipant::create_topic).
    #[tracing::instrument(skip(self, a_listener))]
    pub async fn create_topic<Foo>(
        &self,
        topic_name: &str,
        type_name: &str,
        qos: QosKind<TopicQos>,
        a_listener: Option<impl TopicListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<TopicAsync>
    where
        Foo: TypeSupport,
    {
        self.create_dynamic_topic(topic_name, type_name, qos, a_listener, mask, Foo::TYPE)
            .await
    }

    /// Async version of [`create_dynamic_topic`](crate::domain::domain_participant::DomainParticipant::create_dynamic_topic).
    #[tracing::instrument(skip(self, a_listener, dynamic_type_representation))]
    pub async fn create_dynamic_topic(
        &self,
        topic_name: &str,
        type_name: &str,
        qos: QosKind<TopicQos>,
        a_listener: Option<impl TopicListener + Send + 'static>,
        mask: &[StatusKind],
        dynamic_type_representation: DynamicType<'static>,
    ) -> DdsResult<TopicAsync> {
        let dcps_listener = a_listener.map(DcpsTopicListener::new);
        let reply = self
            .dcps_sender
            .call(DcpsMail::Participant(ParticipantServiceMail::CreateTopic(
                Box::new(CreateTopicMail {
                    participant_handle: self.handle,
                    topic_name: String::from(topic_name),
                    type_name: String::from(type_name),
                    qos,
                    dcps_listener,
                    listener_mask: mask.iter().collect(),
                    type_support: dynamic_type_representation,
                }),
            )))
            .await?;
        let guid = reply.expect_instance_handle()?;

        Ok(TopicAsync::new(
            guid,
            Arc::from(type_name),
            Arc::from(topic_name),
            self.clone(),
        ))
    }

    /// Async version of [`delete_topic`](crate::domain::domain_participant::DomainParticipant::delete_topic).
    #[tracing::instrument(skip(self, a_topic))]
    pub async fn delete_topic(&self, a_topic: &TopicAsync) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::DeleteUserDefinedTopic {
                    participant_handle: a_topic.get_participant().handle,
                    parent_participant_handle: self.handle,
                    topic_name: a_topic.get_name(),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`create_contentfilteredtopic`](crate::domain::domain_participant::DomainParticipant::create_contentfilteredtopic).
    #[tracing::instrument(skip(self, related_topic))]
    pub async fn create_contentfilteredtopic(
        &self,
        name: &str,
        related_topic: &TopicAsync,
        filter_expression: String,
        expression_parameters: Vec<String>,
    ) -> DdsResult<ContentFilteredTopicAsync> {
        let topic = related_topic.clone();
        let name = name.to_string();
        let related_topic_name = related_topic.get_name();
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::CreateContentFilteredTopic(Box::new(
                    CreateContentFilteredTopicMail {
                        participant_handle: topic.get_participant().handle,
                        name: name.clone(),
                        related_topic_name,
                        filter_expression,
                        expression_parameters,
                    },
                )),
            ))
            .await?
            .expect_instance_handle()?;
        Ok(ContentFilteredTopicAsync::new(name.clone(), topic))
    }

    /// Async version of [`delete_contentfilteredtopic`](crate::domain::domain_participant::DomainParticipant::delete_contentfilteredtopic).
    #[tracing::instrument(skip(self, a_contentfilteredtopic))]
    pub async fn delete_contentfilteredtopic(
        &self,
        a_contentfilteredtopic: &ContentFilteredTopicAsync,
    ) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::DeleteContentFilteredTopic {
                    participant_handle: a_contentfilteredtopic.get_participant().handle,
                    name: a_contentfilteredtopic.get_name(),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`find_topic`](crate::domain::domain_participant::DomainParticipant::find_topic).
    #[tracing::instrument(skip(self))]
    pub async fn find_topic<Foo>(
        &self,
        topic_name: &str,
        timeout: Duration,
    ) -> DdsResult<TopicAsync>
    where
        Foo: TypeSupport,
    {
        let topic_name = String::from(topic_name);
        let participant_address = self.dcps_sender.clone();
        let participant_async = self.clone();
        let (reply_sender, reply_receiver) = oneshot();

        participant_address
            .call(DcpsMail::Participant(ParticipantServiceMail::FindTopic {
                participant_handle: self.handle,
                topic_name: topic_name.clone(),
                type_support: Foo::TYPE,
                timeout,
                reply_sender,
            }))
            .await?;
        let (guid, type_name) = reply_receiver.await??;
        Ok(TopicAsync::new(
            guid,
            Arc::from(type_name),
            Arc::from(topic_name),
            participant_async,
        ))
    }

    /// Async version of [`lookup_topicdescription`](crate::domain::domain_participant::DomainParticipant::lookup_topicdescription).
    #[tracing::instrument(skip(self))]
    pub async fn lookup_topicdescription(
        &self,
        topic_name: &str,
    ) -> DdsResult<Option<impl TopicDescriptionAsync>> {
        struct LocalTopicDescription {
            participant: DomainParticipantAsync,
            type_name: String,
            topic_name: String,
        }
        impl TopicDescriptionAsync for LocalTopicDescription {
            fn get_participant(&self) -> DomainParticipantAsync {
                self.participant.clone()
            }

            fn get_type_name(&self) -> String {
                self.type_name.clone()
            }

            fn get_name(&self) -> String {
                self.topic_name.clone()
            }
        }

        let reply = self
            .dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::LookupTopicdescription {
                    participant_handle: self.handle,
                    topic_name: String::from(topic_name),
                },
            ))
            .await?;
        if let Some(type_name) = reply.expect_topic_description()? {
            Ok(Some(LocalTopicDescription {
                participant: self.clone(),
                type_name,
                topic_name: String::from(topic_name),
            }))
        } else {
            Ok(None)
        }
    }

    /// Async version of [`get_builtin_subscriber`](crate::domain::domain_participant::DomainParticipant::get_builtin_subscriber).
    #[tracing::instrument(skip(self))]
    pub fn get_builtin_subscriber(&self) -> SubscriberAsync {
        SubscriberAsync::new(self.handle, self.clone())
    }

    /// Async version of [`ignore_participant`](crate::domain::domain_participant::DomainParticipant::ignore_participant).
    #[tracing::instrument(skip(self))]
    pub async fn ignore_participant(&self, handle: InstanceHandle) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::IgnoreParticipant {
                    participant_handle: self.handle,
                    handle,
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`ignore_topic`](crate::domain::domain_participant::DomainParticipant::ignore_topic).
    #[tracing::instrument(skip(self))]
    pub async fn ignore_topic(&self, handle: InstanceHandle) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`ignore_publication`](crate::domain::domain_participant::DomainParticipant::ignore_publication).
    #[tracing::instrument(skip(self))]
    pub async fn ignore_publication(&self, handle: InstanceHandle) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::IgnorePublication {
                    participant_handle: self.handle,
                    handle,
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`ignore_subscription`](crate::domain::domain_participant::DomainParticipant::ignore_subscription).
    #[tracing::instrument(skip(self))]
    pub async fn ignore_subscription(&self, handle: InstanceHandle) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::IgnoreSubscription {
                    participant_handle: self.handle,
                    handle,
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_domain_id`](crate::domain::domain_participant::DomainParticipant::get_domain_id).
    #[tracing::instrument(skip(self))]
    pub fn get_domain_id(&self) -> DomainId {
        self.domain_id
    }

    /// Async version of [`delete_contained_entities`](crate::domain::domain_participant::DomainParticipant::delete_contained_entities).
    #[tracing::instrument(skip(self))]
    pub async fn delete_contained_entities(&self) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::DeleteContainedEntities {
                    participant_handle: self.handle,
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`assert_liveliness`](crate::domain::domain_participant::DomainParticipant::assert_liveliness).
    #[tracing::instrument(skip(self))]
    pub async fn assert_liveliness(&self) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`set_default_publisher_qos`](crate::domain::domain_participant::DomainParticipant::set_default_publisher_qos).
    #[tracing::instrument(skip(self))]
    pub async fn set_default_publisher_qos(&self, qos: QosKind<PublisherQos>) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::SetDefaultPublisherQos {
                    participant_handle: self.handle,
                    qos: Box::new(qos),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_default_publisher_qos`](crate::domain::domain_participant::DomainParticipant::get_default_publisher_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_default_publisher_qos(&self) -> DdsResult<PublisherQos> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::GetDefaultPublisherQos {
                    participant_handle: self.handle,
                },
            ))
            .await?
            .expect_publisher_qos()
    }

    /// Async version of [`set_default_subscriber_qos`](crate::domain::domain_participant::DomainParticipant::set_default_subscriber_qos).
    #[tracing::instrument(skip(self))]
    pub async fn set_default_subscriber_qos(&self, qos: QosKind<SubscriberQos>) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::SetDefaultSubscriberQos {
                    participant_handle: self.handle,
                    qos: Box::new(qos),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_default_subscriber_qos`](crate::domain::domain_participant::DomainParticipant::get_default_subscriber_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_default_subscriber_qos(&self) -> DdsResult<SubscriberQos> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::GetDefaultSubscriberQos {
                    participant_handle: self.handle,
                },
            ))
            .await?
            .expect_subscriber_qos()
    }

    /// Async version of [`set_default_topic_qos`](crate::domain::domain_participant::DomainParticipant::set_default_topic_qos).
    #[tracing::instrument(skip(self))]
    pub async fn set_default_topic_qos(&self, qos: QosKind<TopicQos>) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::SetDefaultTopicQos {
                    participant_handle: self.handle,
                    qos: Box::new(qos),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_default_topic_qos`](crate::domain::domain_participant::DomainParticipant::get_default_topic_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_default_topic_qos(&self) -> DdsResult<TopicQos> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::GetDefaultTopicQos {
                    participant_handle: self.handle,
                },
            ))
            .await?
            .expect_topic_qos()
    }

    /// Async version of [`get_discovered_participants`](crate::domain::domain_participant::DomainParticipant::get_discovered_participants).
    #[tracing::instrument(skip(self))]
    pub async fn get_discovered_participants(&self) -> DdsResult<Vec<InstanceHandle>> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::GetDiscoveredParticipants {
                    participant_handle: self.handle,
                },
            ))
            .await?
            .expect_instance_handle_list()
    }

    /// Async version of [`get_discovered_participant_data`](crate::domain::domain_participant::DomainParticipant::get_discovered_participant_data).
    #[tracing::instrument(skip(self))]
    pub async fn get_discovered_participant_data(
        &self,
        participant_handle: InstanceHandle,
    ) -> DdsResult<ParticipantBuiltinTopicData> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::GetDiscoveredParticipantData {
                    participant_handle: self.handle,
                    discovered_participant_handle: participant_handle,
                },
            ))
            .await?
            .expect_participant_builtin_topic_data()
    }

    /// Async version of [`get_discovered_topics`](crate::domain::domain_participant::DomainParticipant::get_discovered_topics).
    #[tracing::instrument(skip(self))]
    pub async fn get_discovered_topics(&self) -> DdsResult<Vec<InstanceHandle>> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::GetDiscoveredTopics {
                    participant_handle: self.handle,
                },
            ))
            .await?
            .expect_instance_handle_list()
    }

    /// Async version of [`get_discovered_topic_data`](crate::domain::domain_participant::DomainParticipant::get_discovered_topic_data).
    #[tracing::instrument(skip(self))]
    pub async fn get_discovered_topic_data(
        &self,
        topic_handle: InstanceHandle,
    ) -> DdsResult<TopicBuiltinTopicData> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::GetDiscoveredTopicData {
                    participant_handle: self.handle,
                    topic_handle,
                },
            ))
            .await?
            .expect_topic_builtin_topic_data()
    }

    /// Async version of [`contains_entity`](crate::domain::domain_participant::DomainParticipant::contains_entity).
    #[tracing::instrument(skip(self))]
    pub async fn contains_entity(&self, _a_handle: InstanceHandle) -> DdsResult<bool> {
        todo!()
    }

    /// Async version of [`get_current_time`](crate::domain::domain_participant::DomainParticipant::get_current_time).
    #[tracing::instrument(skip(self))]
    pub async fn get_current_time(&self) -> DdsResult<Time> {
        self.dcps_sender
            .call(DcpsMail::Participant(
                ParticipantServiceMail::GetCurrentTime {
                    participant_handle: self.handle,
                },
            ))
            .await?
            .expect_time()
    }
}

impl DomainParticipantAsync {
    /// Async version of [`set_qos`](crate::domain::domain_participant::DomainParticipant::set_qos).
    #[tracing::instrument(skip(self))]
    pub async fn set_qos(&self, qos: QosKind<DomainParticipantQos>) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(ParticipantServiceMail::SetQos {
                participant_handle: self.handle,
                qos: Box::new(qos),
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_qos`](crate::domain::domain_participant::DomainParticipant::get_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_qos(&self) -> DdsResult<DomainParticipantQos> {
        self.dcps_sender
            .call(DcpsMail::Participant(ParticipantServiceMail::GetQos {
                participant_handle: self.handle,
            }))
            .await?
            .expect_participant_qos()
    }

    /// Async version of [`set_listener`](crate::domain::domain_participant::DomainParticipant::set_listener).
    #[tracing::instrument(skip(self, a_listener))]
    pub async fn set_listener(
        &self,
        a_listener: Option<impl DomainParticipantListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        let dcps_listener = a_listener.map(DcpsDomainParticipantListener::new);
        self.dcps_sender
            .call(DcpsMail::Participant(ParticipantServiceMail::SetListener {
                participant_handle: self.handle,
                dcps_listener,
                listener_mask: mask.iter().collect(),
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_status_changes`](crate::domain::domain_participant::DomainParticipant::get_status_changes).
    #[tracing::instrument(skip(self))]
    pub async fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        todo!()
    }

    /// Async version of [`enable`](crate::domain::domain_participant::DomainParticipant::enable).
    #[tracing::instrument(skip(self))]
    pub async fn enable(&self) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::Participant(ParticipantServiceMail::Enable {
                participant_handle: self.handle,
            }))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_instance_handle`](crate::domain::domain_participant::DomainParticipant::get_instance_handle).
    #[tracing::instrument(skip(self))]
    pub fn get_instance_handle(&self) -> InstanceHandle {
        self.handle
    }
}
