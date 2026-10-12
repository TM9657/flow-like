use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use crate::{
    builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    dcps::{
        channels::oneshot::OneshotSender,
        dcps_domain_participant::{
            participant_entity::{
                BUILT_IN_TOPIC_NAME_LIST, DcpsDomainParticipant, FindTopicNotification,
            },
            topic_entity::{ContentFilteredTopicEntity, TopicEntity, get_topic_type_support},
            user_defined_publisher::PublisherEntity,
            user_defined_subscriber::UserDefinedSubscriber,
        },
        listeners::{
            domain_participant_listener::DcpsDomainParticipantListener,
            publisher_listener::DcpsPublisherListener, subscriber_listener::DcpsSubscriberListener,
            topic_listener::DcpsTopicListener,
        },
        status_condition::DcpsStatusCondition,
        status_mask::StatusMask,
    },
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::{DomainParticipantQos, PublisherQos, QosKind, SubscriberQos, TopicQos},
        time::{Duration, Time},
    },
    runtime::DdsRuntime,
    transport::types::{USER_DEFINED_READER_GROUP, USER_DEFINED_TOPIC, USER_DEFINED_WRITER_GROUP},
    xtypes::dynamic_type::DynamicType,
};

impl DcpsDomainParticipant {
    #[tracing::instrument(skip(self, dcps_listener, runtime))]
    pub fn create_user_defined_publisher(
        &mut self,
        qos: QosKind<PublisherQos>,
        dcps_listener: Option<DcpsPublisherListener>,
        listener_mask: StatusMask,
        runtime: &impl DdsRuntime,
    ) -> DdsResult<InstanceHandle> {
        let publisher_qos = match qos {
            QosKind::Default => self.domain_participant.default_publisher_qos.clone(),
            QosKind::Specific(q) => q,
        };

        let publisher_handle = InstanceHandle::new([
            self.domain_participant.instance_handle[0],
            self.domain_participant.instance_handle[1],
            self.domain_participant.instance_handle[2],
            self.domain_participant.instance_handle[3],
            self.domain_participant.instance_handle[4],
            self.domain_participant.instance_handle[5],
            self.domain_participant.instance_handle[6],
            self.domain_participant.instance_handle[7],
            self.domain_participant.instance_handle[8],
            self.domain_participant.instance_handle[9],
            self.domain_participant.instance_handle[10],
            self.domain_participant.instance_handle[11],
            self.publisher_counter,
            0,
            0,
            USER_DEFINED_WRITER_GROUP,
        ]);
        self.publisher_counter += 1;
        let data_writer_list = Default::default();
        let listener_sender = dcps_listener.map(|l| l.spawn(&runtime.spawner()));
        let mut publisher = PublisherEntity::new(
            publisher_qos,
            publisher_handle,
            data_writer_list,
            listener_sender,
            listener_mask,
        );

        if self.domain_participant.enabled
            && self
                .domain_participant
                .qos
                .entity_factory
                .autoenable_created_entities
        {
            publisher.enabled = true;
        }

        self.domain_participant
            .user_defined_publisher_list
            .push(publisher);

        Ok(publisher_handle)
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_user_defined_publisher(
        &mut self,
        participant_handle: &InstanceHandle,
        publisher_handle: &InstanceHandle,
    ) -> DdsResult<()> {
        if participant_handle != &self.domain_participant.instance_handle {
            return Err(DdsError::PreconditionNotMet(
                "Publisher can only be deleted from its parent participant".to_string(),
            ));
        }
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        if !publisher.data_writer_list.is_empty() {
            return Err(DdsError::PreconditionNotMet(
                "Publisher still contains data writers".to_string(),
            ));
        }
        let Some(_) = self.domain_participant.remove_publisher(publisher_handle) else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(())
    }

    #[tracing::instrument(skip(self, dcps_listener, runtime))]
    pub fn create_user_defined_subscriber(
        &mut self,
        qos: QosKind<SubscriberQos>,
        dcps_listener: Option<DcpsSubscriberListener>,
        listener_mask: StatusMask,
        runtime: &impl DdsRuntime,
    ) -> DdsResult<InstanceHandle> {
        let subscriber_qos = match qos {
            QosKind::Default => self.domain_participant.default_subscriber_qos.clone(),
            QosKind::Specific(q) => q,
        };
        let subscriber_handle = InstanceHandle::new([
            self.domain_participant.instance_handle[0],
            self.domain_participant.instance_handle[1],
            self.domain_participant.instance_handle[2],
            self.domain_participant.instance_handle[3],
            self.domain_participant.instance_handle[4],
            self.domain_participant.instance_handle[5],
            self.domain_participant.instance_handle[6],
            self.domain_participant.instance_handle[7],
            self.domain_participant.instance_handle[8],
            self.domain_participant.instance_handle[9],
            self.domain_participant.instance_handle[10],
            self.domain_participant.instance_handle[11],
            self.subscriber_counter,
            0,
            0,
            USER_DEFINED_READER_GROUP,
        ]);
        self.subscriber_counter += 1;

        let listener_sender = dcps_listener.map(|l| l.spawn(&runtime.spawner()));
        let mut subscriber = UserDefinedSubscriber::new(
            subscriber_handle,
            subscriber_qos,
            listener_sender,
            listener_mask,
        );

        if self.domain_participant.enabled
            && self
                .domain_participant
                .qos
                .entity_factory
                .autoenable_created_entities
        {
            subscriber.enabled = true;
        }

        self.domain_participant
            .user_defined_subscriber_list
            .push(subscriber);

        Ok(subscriber_handle)
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_user_defined_subscriber(
        &mut self,
        participant_handle: &InstanceHandle,
        subscriber_handle: &InstanceHandle,
    ) -> DdsResult<()> {
        if &self.domain_participant.instance_handle != participant_handle {
            return Err(DdsError::PreconditionNotMet(
                "Subscriber can only be deleted from its parent participant".to_string(),
            ));
        }

        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        if !subscriber.data_reader_list.is_empty() {
            return Err(DdsError::PreconditionNotMet(
                "Subscriber still contains data readers".to_string(),
            ));
        }
        let Some(_) = self.domain_participant.remove_subscriber(subscriber_handle) else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(skip(self, dcps_listener, type_support, runtime))]
    pub fn create_topic(
        &mut self,
        topic_name: String,
        type_name: String,
        qos: QosKind<TopicQos>,
        dcps_listener: Option<DcpsTopicListener>,
        listener_mask: StatusMask,
        type_support: DynamicType<'static>,
        runtime: &impl DdsRuntime,
        now: Time,
    ) -> DdsResult<InstanceHandle> {
        if BUILT_IN_TOPIC_NAME_LIST.contains(&topic_name.as_str()) {
            return Err(DdsError::BadParameter);
        }

        if self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .any(|x| x.topic_name.as_ref() == topic_name.as_str())
        {
            return Err(DdsError::PreconditionNotMet(format!(
                "Topic with name {topic_name} already exists.
         To access this topic call the lookup_topicdescription method.",
            )));
        }

        let status_condition = DcpsStatusCondition::default();
        let qos = match qos {
            QosKind::Default => self.domain_participant.default_topic_qos.clone(),
            QosKind::Specific(q) => q,
        };

        let topic_handle = InstanceHandle::new([
            self.domain_participant.instance_handle[0],
            self.domain_participant.instance_handle[1],
            self.domain_participant.instance_handle[2],
            self.domain_participant.instance_handle[3],
            self.domain_participant.instance_handle[4],
            self.domain_participant.instance_handle[5],
            self.domain_participant.instance_handle[6],
            self.domain_participant.instance_handle[7],
            self.domain_participant.instance_handle[8],
            self.domain_participant.instance_handle[9],
            self.domain_participant.instance_handle[10],
            self.domain_participant.instance_handle[11],
            0,
            self.domain_participant.topic_counter.to_ne_bytes()[0],
            self.domain_participant.topic_counter.to_ne_bytes()[1],
            USER_DEFINED_TOPIC,
        ]);
        self.domain_participant.topic_counter += 1;
        let listener_sender = dcps_listener.map(|l| l.spawn(&runtime.spawner()));
        let type_information = self
            .domain_participant
            .type_register
            .register_local_type(Arc::from(type_name.as_ref()), type_support);
        let topic = TopicEntity::new(
            qos,
            Arc::from(type_name),
            Arc::from(topic_name.as_str()),
            topic_handle,
            status_condition,
            listener_sender,
            listener_mask,
            type_information,
        );

        self.domain_participant
            .locally_created_topic_list
            .push(topic);

        if self.domain_participant.enabled
            && self
                .domain_participant
                .qos
                .entity_factory
                .autoenable_created_entities
        {
            self.enable_topic(topic_name, now)?;
        }

        Ok(topic_handle)
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_user_defined_topic(
        &mut self,
        participant_handle: &InstanceHandle,
        topic_name: String,
    ) -> DdsResult<()> {
        if &self.domain_participant.instance_handle != participant_handle {
            return Err(DdsError::PreconditionNotMet(
                "Topic can only be deleted from its parent participant".to_string(),
            ));
        }

        if BUILT_IN_TOPIC_NAME_LIST.contains(&topic_name.as_str()) {
            return Ok(());
        }

        let Some(topic) = self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .find(|x| x.topic_name.as_ref() == topic_name.as_str())
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        for publisher in self.domain_participant.user_defined_publisher_list.iter() {
            for writer in publisher.data_writer_list.iter() {
                if writer.topic_name == topic.topic_name {
                    return Err(DdsError::PreconditionNotMet(
                        "Topic still attached to some data writer or data reader".to_string(),
                    ));
                }
            }
        }

        for subscriber in self.domain_participant.user_defined_subscriber_list.iter() {
            for reader in subscriber.data_reader_list.iter() {
                if reader.topic_name == topic.topic_name {
                    return Err(DdsError::PreconditionNotMet(
                        "Topic still attached to some data writer or data reader".to_string(),
                    ));
                }
            }
        }

        self.domain_participant
            .locally_created_topic_list
            .retain(|x| x.topic_name.as_ref() != topic_name.as_str());

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn create_content_filtered_topic(
        &mut self,
        participant_handle: &InstanceHandle,
        name: String,
        related_topic_name: String,
        filter_expression: String,
        expression_parameters: Vec<String>,
    ) -> DdsResult<InstanceHandle> {
        if !self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .any(|x| x.topic_name.as_ref() == related_topic_name.as_str())
        {
            return Err(DdsError::PreconditionNotMet(format!(
                "Related topic with name {related_topic_name} does not exist."
            )));
        }

        let topic_handle = InstanceHandle::new([
            self.domain_participant.instance_handle[0],
            self.domain_participant.instance_handle[1],
            self.domain_participant.instance_handle[2],
            self.domain_participant.instance_handle[3],
            self.domain_participant.instance_handle[4],
            self.domain_participant.instance_handle[5],
            self.domain_participant.instance_handle[6],
            self.domain_participant.instance_handle[7],
            self.domain_participant.instance_handle[8],
            self.domain_participant.instance_handle[9],
            self.domain_participant.instance_handle[10],
            self.domain_participant.instance_handle[11],
            0,
            self.domain_participant.topic_counter.to_ne_bytes()[0],
            self.domain_participant.topic_counter.to_ne_bytes()[1],
            USER_DEFINED_TOPIC,
        ]);
        self.domain_participant.topic_counter += 1;

        let topic = ContentFilteredTopicEntity::new(
            Arc::from(name),
            Arc::from(related_topic_name),
            filter_expression,
            expression_parameters,
        );
        self.domain_participant
            .content_filtered_topic_list
            .push(topic);

        Ok(topic_handle)
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_content_filtered_topic(
        &mut self,
        participant_handle: &InstanceHandle,
        name: String,
    ) -> DdsResult<()> {
        Ok(())
    }

    #[tracing::instrument(skip(self, type_support, reply_sender))]
    pub fn find_topic(
        &mut self,
        topic_name: String,
        type_support: DynamicType<'static>,
        timeout: Duration,
        now: Time,
        reply_sender: OneshotSender<DdsResult<(InstanceHandle, String)>>,
    ) {
        let found_topic = self
            .domain_participant
            .find_topic(&topic_name, type_support);
        if let Some(t) = found_topic {
            reply_sender.send(Ok(t));
        } else if timeout > Duration::new(0, 0) {
            self.domain_participant
                .find_topic_sender_list
                .push(FindTopicNotification {
                    topic_name,
                    deadline: now + timeout,
                    type_support,
                    reply_sender,
                });
        } else {
            reply_sender.send(Err(DdsError::Timeout));
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn lookup_topicdescription(&mut self, topic_name: String) -> DdsResult<Option<String>> {
        if let Some(topic) = self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .find(|x| x.topic_name.as_ref() == topic_name.as_str())
        {
            Ok(Some(topic.type_name.to_string()))
        } else if BUILT_IN_TOPIC_NAME_LIST.contains(&topic_name.as_str()) {
            let type_support = get_topic_type_support(
                &topic_name,
                &self.domain_participant.content_filtered_topic_list,
                &self.domain_participant.locally_created_topic_list,
                &self.domain_participant.type_register,
            );
            Ok(type_support.map(|t| t.get_name().to_string()))
        } else {
            Ok(None)
        }
    }

    /// Ignore participant with the specified [`handle`](InstanceHandle).
    #[tracing::instrument(skip(self))]
    pub fn ignore_participant(&mut self, handle: &InstanceHandle, now: Time) -> DdsResult<()> {
        // Check enabled
        if !self.domain_participant.enabled {
            return Err(DdsError::NotEnabled);
        }

        // Add to ignored participants
        if !self.domain_participant.ignored_participants.insert(*handle) {
            // Already ignored
            return Ok(());
        }

        // Remove participant
        self.remove_discovered_participant(handle, now);

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn ignore_publication(&mut self, handle: &InstanceHandle) -> DdsResult<()> {
        if !self.domain_participant.enabled {
            return Err(DdsError::NotEnabled);
        }

        self.domain_participant.ignored_publications.insert(*handle);
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn ignore_subscription(&mut self, handle: &InstanceHandle) -> DdsResult<()> {
        if !self.domain_participant.enabled {
            return Err(DdsError::NotEnabled);
        }

        self.domain_participant
            .ignored_subscriptions
            .insert(*handle);
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_participant_contained_entities(&mut self, now: Time) -> DdsResult<()> {
        let deleted_publisher_list: Vec<PublisherEntity> = self
            .domain_participant
            .user_defined_publisher_list
            .drain(..)
            .collect();
        for mut publisher in deleted_publisher_list {
            for data_writer in publisher.data_writer_list.drain(..) {
                self.announce_deleted_data_writer(data_writer, now);
            }
        }

        let deleted_subscriber_list: Vec<UserDefinedSubscriber> = self
            .domain_participant
            .user_defined_subscriber_list
            .drain(..)
            .collect();
        for mut subscriber in deleted_subscriber_list {
            for data_reader in subscriber.data_reader_list.drain(..) {
                self.announce_deleted_data_reader(data_reader, now);
            }
        }

        self.domain_participant
            .locally_created_topic_list
            .retain(|x| BUILT_IN_TOPIC_NAME_LIST.contains(&x.topic_name.as_ref()));

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn set_default_publisher_qos(&mut self, qos: QosKind<PublisherQos>) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => PublisherQos::default(),
            QosKind::Specific(q) => q,
        };

        self.domain_participant.default_publisher_qos = qos;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_default_publisher_qos(&mut self) -> DdsResult<PublisherQos> {
        Ok(self.domain_participant.default_publisher_qos.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn set_default_subscriber_qos(&mut self, qos: QosKind<SubscriberQos>) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => SubscriberQos::default(),
            QosKind::Specific(q) => q,
        };

        self.domain_participant.default_subscriber_qos = qos;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_default_subscriber_qos(&mut self) -> DdsResult<SubscriberQos> {
        Ok(self.domain_participant.default_subscriber_qos.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn set_default_topic_qos(&mut self, qos: QosKind<TopicQos>) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => TopicQos::default(),
            QosKind::Specific(q) => {
                if q.is_consistent().is_ok() {
                    q
                } else {
                    return Err(DdsError::InconsistentPolicy);
                }
            }
        };

        qos.is_consistent()?;
        self.domain_participant.default_topic_qos = qos;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_default_topic_qos(&self) -> DdsResult<TopicQos> {
        Ok(self.domain_participant.default_topic_qos.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_discovered_participants(&mut self) -> DdsResult<Vec<InstanceHandle>> {
        Ok(self
            .domain_participant
            .discovered_participant_list
            .iter()
            .map(|p| InstanceHandle::new(p.dds_participant_data.key().value))
            .collect())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_discovered_participant_data(
        &mut self,
        participant_handle: &InstanceHandle,
    ) -> DdsResult<ParticipantBuiltinTopicData> {
        let Some(handle) = self
            .domain_participant
            .discovered_participant_list
            .iter()
            .find(|p| &p.dds_participant_data.key().value == participant_handle)
        else {
            return Err(DdsError::BadParameter);
        };
        Ok(handle.dds_participant_data.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_discovered_topics(&mut self) -> DdsResult<Vec<InstanceHandle>> {
        Ok(self
            .domain_participant
            .discovered_topic_list
            .iter()
            .map(|x| InstanceHandle::new(x.key().value))
            .collect())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_discovered_topic_data(
        &mut self,
        topic_handle: &InstanceHandle,
    ) -> DdsResult<TopicBuiltinTopicData> {
        let Some(handle) = self
            .domain_participant
            .get_discovered_topic_data(topic_handle)
        else {
            return Err(DdsError::PreconditionNotMet(String::from(
                "Topic with this handle not discovered",
            )));
        };

        Ok(handle.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn set_domain_participant_qos(
        &mut self,
        qos: QosKind<DomainParticipantQos>,
        now: Time,
    ) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => DomainParticipantQos::default(),
            QosKind::Specific(q) => q,
        };

        self.domain_participant.qos = qos;
        if self.domain_participant.enabled {
            self.announce_participant(now);
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_domain_participant_qos(&mut self) -> DdsResult<DomainParticipantQos> {
        Ok(self.domain_participant.qos.clone())
    }

    #[tracing::instrument(skip(self, dcps_listener, runtime))]
    pub fn set_domain_participant_listener(
        &mut self,
        dcps_listener: Option<DcpsDomainParticipantListener>,
        listener_mask: StatusMask,
        runtime: &impl DdsRuntime,
    ) -> DdsResult<()> {
        let listener_sender = dcps_listener.map(|l| l.spawn(&runtime.spawner()));
        self.domain_participant.listener_sender = listener_sender;
        self.domain_participant.listener_mask = listener_mask;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn enable_domain_participant(&mut self, now: Time) -> DdsResult<()> {
        if !self.domain_participant.enabled {
            for t in &mut self.domain_participant.locally_created_topic_list {
                t.enabled = true;
            }
            self.domain_participant.builtin_publisher.enable();

            self.domain_participant.builtin_subscriber.enable();
            self.domain_participant.enabled = true;

            self.announce_participant(now);
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn is_participant_empty(&mut self) -> bool {
        self.domain_participant.is_empty()
    }
}
