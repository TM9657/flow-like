use alloc::boxed::Box;

use crate::{
    dcps::{
        dcps_mail::{
            DcpsMail, DcpsReply, MessageServiceMail, ParticipantFactoryMail,
            ParticipantServiceMail, PublisherServiceMail, ReaderServiceMail, StatusConditionMail,
            SubscriberServiceMail, TopicServiceMail, WriterServiceMail,
        },
        dcps_participant_factory::DcpsParticipantFactory,
    },
    infrastructure::{error::DdsError, time::Time},
    runtime::DdsRuntime,
};

impl<R: DdsRuntime> DcpsParticipantFactory<R> {
    pub fn handle(&mut self, message: DcpsMail, now: Time) -> DcpsReply {
        match message {
            DcpsMail::ParticipantFactory(ParticipantFactoryMail::CreateParticipant(p)) => {
                DcpsReply::InstanceHandle(self.create_participant(
                    p.guid_prefix,
                    p.domain_id,
                    p.qos,
                    p.dcps_listener,
                    p.listener_mask,
                    p.transport_participant,
                    p.domain_tag,
                    p.participant_announcement_interval,
                    p.enable_type_information,
                    now,
                ))
            }
            DcpsMail::ParticipantFactory(ParticipantFactoryMail::DeleteParticipant {
                participant_handle,
            }) => DcpsReply::Ok(self.delete_participant(&participant_handle, now)),
            DcpsMail::ParticipantFactory(ParticipantFactoryMail::SetDefaultParticipantQos {
                qos,
            }) => DcpsReply::Ok(self.set_default_participant_qos(*qos)),
            DcpsMail::ParticipantFactory(ParticipantFactoryMail::GetDefaultParticipantQos) => {
                DcpsReply::ParticipantQos(Ok(self.get_default_participant_qos()))
            }
            DcpsMail::ParticipantFactory(ParticipantFactoryMail::SetQos { qos }) => {
                DcpsReply::Ok(self.set_qos(*qos))
            }
            DcpsMail::ParticipantFactory(ParticipantFactoryMail::GetQos) => {
                DcpsReply::FactoryQos(Ok(self.get_qos()))
            }
            DcpsMail::ParticipantFactory(ParticipantFactoryMail::LookupParticipant {
                domain_id,
            }) => DcpsReply::OptionInstanceHandle(Ok(self
                .domain_participant_list
                .iter()
                .find(|x| x.domain_id() == domain_id)
                .map(|x| *x.get_instance_handle()))),
            DcpsMail::Participant(ParticipantServiceMail::CreateUserDefinedPublisher {
                qos,
                participant_handle,
                dcps_listener,
                listener_mask,
            }) => DcpsReply::InstanceHandle(
                self.domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                    .and_then(|p| {
                        p.create_user_defined_publisher(
                            qos,
                            dcps_listener,
                            listener_mask,
                            &self.runtime,
                        )
                    }),
            ),
            DcpsMail::Participant(ParticipantServiceMail::DeleteUserDefinedPublisher {
                participant_handle,
                parent_participant_handle,
                publisher_handle,
            }) => DcpsReply::Ok(self.find_participant(&participant_handle).and_then(|p| {
                p.delete_user_defined_publisher(&parent_participant_handle, &publisher_handle)
            })),
            DcpsMail::Participant(ParticipantServiceMail::CreateUserDefinedSubscriber {
                participant_handle,
                qos,
                dcps_listener,
                listener_mask,
            }) => DcpsReply::InstanceHandle(
                self.domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                    .and_then(|p| {
                        p.create_user_defined_subscriber(
                            qos,
                            dcps_listener,
                            listener_mask,
                            &self.runtime,
                        )
                    }),
            ),
            DcpsMail::Participant(ParticipantServiceMail::DeleteUserDefinedSubscriber {
                participant_handle,
                parent_participant_handle,
                subscriber_handle,
            }) => DcpsReply::Ok(self.find_participant(&participant_handle).and_then(|p| {
                p.delete_user_defined_subscriber(&parent_participant_handle, &subscriber_handle)
            })),
            DcpsMail::Participant(ParticipantServiceMail::CreateTopic(p)) => {
                match self
                    .domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &p.participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                {
                    Ok(participant) => DcpsReply::InstanceHandle(participant.create_topic(
                        p.topic_name,
                        p.type_name,
                        p.qos,
                        p.dcps_listener,
                        p.listener_mask,
                        p.type_support,
                        &self.runtime,
                        now,
                    )),
                    Err(e) => DcpsReply::InstanceHandle(Err(e)),
                }
            }
            DcpsMail::Participant(ParticipantServiceMail::DeleteUserDefinedTopic {
                participant_handle,
                parent_participant_handle,
                topic_name,
            }) => {
                DcpsReply::Ok(self.find_participant(&participant_handle).and_then(|p| {
                    p.delete_user_defined_topic(&parent_participant_handle, topic_name)
                }))
            }
            DcpsMail::Participant(ParticipantServiceMail::CreateContentFilteredTopic(p)) => {
                match self.find_participant(&p.participant_handle) {
                    Ok(participant) => {
                        DcpsReply::InstanceHandle(participant.create_content_filtered_topic(
                            &p.participant_handle,
                            p.name,
                            p.related_topic_name,
                            p.filter_expression,
                            p.expression_parameters,
                        ))
                    }
                    Err(e) => DcpsReply::InstanceHandle(Err(e)),
                }
            }
            DcpsMail::Participant(ParticipantServiceMail::DeleteContentFilteredTopic {
                participant_handle,
                name,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.delete_content_filtered_topic(&participant_handle, name)),
            ),
            DcpsMail::Participant(ParticipantServiceMail::FindTopic {
                participant_handle,
                topic_name,
                type_support,
                timeout,
                reply_sender,
            }) => {
                match self.find_participant(&participant_handle) {
                    Ok(p) => p.find_topic(topic_name, type_support, timeout, now, reply_sender),
                    Err(e) => reply_sender.send(Err(e)),
                }
                DcpsReply::Ok(Ok(()))
            }
            DcpsMail::Participant(ParticipantServiceMail::LookupTopicdescription {
                participant_handle,
                topic_name,
            }) => DcpsReply::TopicDescription(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.lookup_topicdescription(topic_name)),
            ),
            DcpsMail::Participant(ParticipantServiceMail::IgnoreParticipant {
                participant_handle,
                handle,
            }) => match self.find_participant(&participant_handle) {
                Ok(p) => DcpsReply::Ok(p.ignore_participant(&handle, now)),
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Participant(ParticipantServiceMail::IgnoreSubscription {
                participant_handle,
                handle,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.ignore_subscription(&handle)),
            ),
            DcpsMail::Participant(ParticipantServiceMail::IgnorePublication {
                participant_handle,
                handle,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.ignore_publication(&handle)),
            ),
            DcpsMail::Participant(ParticipantServiceMail::DeleteContainedEntities {
                participant_handle,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => DcpsReply::Ok(p.delete_participant_contained_entities(now)),
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Participant(ParticipantServiceMail::SetDefaultPublisherQos {
                participant_handle,
                qos,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.set_default_publisher_qos(*qos)),
            ),
            DcpsMail::Participant(ParticipantServiceMail::GetDefaultPublisherQos {
                participant_handle,
            }) => DcpsReply::PublisherQos(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.get_default_publisher_qos()),
            ),
            DcpsMail::Participant(ParticipantServiceMail::SetDefaultSubscriberQos {
                participant_handle,
                qos,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.set_default_subscriber_qos(*qos)),
            ),
            DcpsMail::Participant(ParticipantServiceMail::GetDefaultSubscriberQos {
                participant_handle,
            }) => DcpsReply::SubscriberQos(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.get_default_subscriber_qos()),
            ),
            DcpsMail::Participant(ParticipantServiceMail::SetDefaultTopicQos {
                participant_handle,
                qos,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.set_default_topic_qos(*qos)),
            ),
            DcpsMail::Participant(ParticipantServiceMail::GetDefaultTopicQos {
                participant_handle,
            }) => DcpsReply::TopicQos(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.get_default_topic_qos().map(Box::new)),
            ),
            DcpsMail::Participant(ParticipantServiceMail::GetCurrentTime {
                participant_handle,
            }) => DcpsReply::Time(
                self.domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                    .map(|_| now),
            ),
            DcpsMail::Participant(ParticipantServiceMail::GetDiscoveredParticipants {
                participant_handle,
            }) => DcpsReply::InstanceHandleList(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.get_discovered_participants()),
            ),
            DcpsMail::Participant(ParticipantServiceMail::GetDiscoveredParticipantData {
                participant_handle,
                discovered_participant_handle,
            }) => DcpsReply::ParticipantBuiltinTopicData(
                self.find_participant(&participant_handle).and_then(|p| {
                    p.get_discovered_participant_data(&discovered_participant_handle)
                        .map(Box::new)
                }),
            ),
            DcpsMail::Participant(ParticipantServiceMail::GetDiscoveredTopics {
                participant_handle,
            }) => DcpsReply::InstanceHandleList(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.get_discovered_topics()),
            ),
            DcpsMail::Participant(ParticipantServiceMail::GetDiscoveredTopicData {
                participant_handle,
                topic_handle,
            }) => DcpsReply::TopicBuiltinTopicData(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.get_discovered_topic_data(&topic_handle).map(Box::new)),
            ),
            DcpsMail::Participant(ParticipantServiceMail::SetQos {
                participant_handle,
                qos,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => DcpsReply::Ok(p.set_domain_participant_qos(*qos, now)),
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Participant(ParticipantServiceMail::GetQos { participant_handle }) => {
                DcpsReply::ParticipantQos(
                    self.find_participant(&participant_handle)
                        .and_then(|p| p.get_domain_participant_qos()),
                )
            }
            DcpsMail::Participant(ParticipantServiceMail::SetListener {
                participant_handle,
                dcps_listener,
                listener_mask,
            }) => DcpsReply::Ok(
                self.domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                    .and_then(|p| {
                        p.set_domain_participant_listener(
                            dcps_listener,
                            listener_mask,
                            &self.runtime,
                        )
                    }),
            ),
            DcpsMail::Participant(ParticipantServiceMail::Enable { participant_handle }) => {
                match self
                    .domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                {
                    Ok(p) => DcpsReply::Ok(p.enable_domain_participant(now)),
                    Err(e) => DcpsReply::Ok(Err(e)),
                }
            }
            DcpsMail::Topic(TopicServiceMail::GetInconsistentTopicStatus {
                participant_handle,
                topic_name,
            }) => match self.find_participant(&participant_handle) {
                Ok(p) => {
                    DcpsReply::InconsistentTopicStatus(p.get_inconsistent_topic_status(topic_name))
                }
                Err(e) => DcpsReply::InconsistentTopicStatus(Err(e)),
            },
            DcpsMail::Topic(TopicServiceMail::SetQos {
                participant_handle,
                topic_name,
                topic_qos,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.set_topic_qos(topic_name, *topic_qos)),
            ),
            DcpsMail::Topic(TopicServiceMail::GetQos {
                participant_handle,
                topic_name,
            }) => DcpsReply::TopicQos(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.get_topic_qos(topic_name).map(Box::new)),
            ),
            DcpsMail::Topic(TopicServiceMail::Enable {
                participant_handle,
                topic_name,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => DcpsReply::Ok(p.enable_topic(topic_name, now)),
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Topic(TopicServiceMail::GetTypeSupport {
                participant_handle,
                topic_name,
            }) => DcpsReply::DynamicType(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.get_type_support(topic_name)),
            ),
            DcpsMail::Publisher(PublisherServiceMail::CreateDataWriter(p)) => {
                match self
                    .domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &p.participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                {
                    Ok(participant) => DcpsReply::InstanceHandle(participant.create_data_writer(
                        &p.publisher_handle,
                        p.topic_name,
                        p.qos,
                        p.dcps_listener,
                        p.listener_mask,
                        &self.runtime,
                        now,
                    )),
                    Err(e) => DcpsReply::InstanceHandle(Err(e)),
                }
            }
            DcpsMail::Publisher(PublisherServiceMail::DeleteDataWriter {
                participant_handle,
                publisher_handle,
                datawriter_handle,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => {
                    DcpsReply::Ok(p.delete_data_writer(&publisher_handle, &datawriter_handle, now))
                }
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Publisher(PublisherServiceMail::GetDefaultDataWriterQos {
                participant_handle,
                publisher_handle,
            }) => {
                DcpsReply::DataWriterQos(self.find_participant(&participant_handle).and_then(|p| {
                    p.get_default_datawriter_qos(&publisher_handle)
                        .map(Box::new)
                }))
            }
            DcpsMail::Publisher(PublisherServiceMail::SetDefaultDataWriterQos {
                participant_handle,
                publisher_handle,
                qos,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.set_default_datawriter_qos(&publisher_handle, *qos)),
            ),
            DcpsMail::Publisher(PublisherServiceMail::GetPublisherQos {
                participant_handle,
                publisher_handle,
            }) => DcpsReply::PublisherQos(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.get_publisher_qos(&publisher_handle)),
            ),
            DcpsMail::Publisher(PublisherServiceMail::SetPublisherQos {
                participant_handle,
                publisher_handle,
                qos,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.set_publisher_qos(&publisher_handle, *qos)),
            ),
            DcpsMail::Publisher(PublisherServiceMail::SetPublisherListener {
                participant_handle,
                publisher_handle,
                dcps_listener,
                listener_mask,
            }) => DcpsReply::Ok(
                self.domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                    .and_then(|p| {
                        p.set_publisher_listener(
                            &publisher_handle,
                            dcps_listener,
                            listener_mask,
                            &self.runtime,
                        )
                    }),
            ),
            DcpsMail::Writer(WriterServiceMail::SetListener {
                participant_handle,
                publisher_handle,
                data_writer_handle,
                dcps_listener,
                listener_mask,
            }) => DcpsReply::Ok(
                self.domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                    .and_then(|p| {
                        p.set_listener_data_writer(
                            &publisher_handle,
                            &data_writer_handle,
                            dcps_listener,
                            listener_mask,
                            &self.runtime,
                        )
                    }),
            ),
            DcpsMail::Writer(WriterServiceMail::GetDataWriterQos {
                participant_handle,
                publisher_handle,
                data_writer_handle,
            }) => {
                DcpsReply::DataWriterQos(self.find_participant(&participant_handle).and_then(|p| {
                    p.get_data_writer_qos(&publisher_handle, &data_writer_handle)
                        .map(Box::new)
                }))
            }
            DcpsMail::Writer(WriterServiceMail::GetMatchedSubscriptions {
                participant_handle,
                publisher_handle,
                data_writer_handle,
            }) => {
                DcpsReply::InstanceHandleList(self.find_participant(&participant_handle).and_then(
                    |p| p.get_matched_subscriptions(&publisher_handle, &data_writer_handle),
                ))
            }
            DcpsMail::Writer(WriterServiceMail::GetMatchedSubscriptionData {
                participant_handle,
                publisher_handle,
                data_writer_handle,
                subscription_handle,
            }) => DcpsReply::SubscriptionBuiltinTopicData(
                self.find_participant(&participant_handle).and_then(|p| {
                    p.get_matched_subscription_data(
                        &publisher_handle,
                        &data_writer_handle,
                        &subscription_handle,
                    )
                    .map(Box::new)
                }),
            ),
            DcpsMail::Writer(WriterServiceMail::GetPublicationMatchedStatus {
                participant_handle,
                publisher_handle,
                data_writer_handle,
            }) => match self.find_participant(&participant_handle) {
                Ok(p) => DcpsReply::PublicationMatchedStatus(
                    p.get_publication_matched_status(&publisher_handle, &data_writer_handle),
                ),
                Err(e) => DcpsReply::PublicationMatchedStatus(Err(e)),
            },
            DcpsMail::Writer(WriterServiceMail::RegisterInstance {
                participant_handle,
                publisher_handle,
                data_writer_handle,
                dynamic_data,
                timestamp,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => {
                    let timestamp = timestamp.unwrap_or(now);
                    DcpsReply::OptionInstanceHandle(p.register_instance(
                        &publisher_handle,
                        &data_writer_handle,
                        &dynamic_data,
                        timestamp,
                    ))
                }
                Err(e) => DcpsReply::OptionInstanceHandle(Err(e)),
            },
            DcpsMail::Writer(WriterServiceMail::UnregisterInstance {
                participant_handle,
                publisher_handle,
                data_writer_handle,
                dynamic_data,
                timestamp,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => {
                    let timestamp = timestamp.unwrap_or(now);
                    DcpsReply::Ok(p.unregister_instance(
                        &publisher_handle,
                        &data_writer_handle,
                        &dynamic_data,
                        timestamp,
                        now,
                    ))
                }
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Writer(WriterServiceMail::LookupInstance {
                participant_handle,
                publisher_handle,
                data_writer_handle,
                dynamic_data,
            }) => DcpsReply::OptionInstanceHandle(
                self.find_participant(&participant_handle).and_then(|p| {
                    p.lookup_instance(&publisher_handle, &data_writer_handle, &dynamic_data)
                }),
            ),
            DcpsMail::Writer(WriterServiceMail::WriteWTimestamp {
                participant_handle,
                publisher_handle,
                data_writer_handle,
                dynamic_data,
                timestamp,
                reply_sender,
            }) => {
                match self
                    .domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                {
                    Ok(p) => {
                        let timestamp = timestamp.unwrap_or(now);
                        p.write_w_timestamp(
                            &publisher_handle,
                            &data_writer_handle,
                            &dynamic_data,
                            timestamp,
                            now,
                            reply_sender,
                        );
                    }
                    Err(e) => reply_sender.send(Err(e)),
                }
                DcpsReply::Ok(Ok(()))
            }
            DcpsMail::Writer(WriterServiceMail::DisposeWTimestamp {
                participant_handle,
                publisher_handle,
                data_writer_handle,
                dynamic_data,
                timestamp,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => {
                    let timestamp = timestamp.unwrap_or(now);
                    DcpsReply::Ok(p.dispose_w_timestamp(
                        &publisher_handle,
                        &data_writer_handle,
                        &dynamic_data,
                        timestamp,
                        now,
                    ))
                }
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Writer(WriterServiceMail::GetOfferedDeadlineMissedStatus {
                participant_handle,
                publisher_handle,
                data_writer_handle,
            }) => match self.find_participant(&participant_handle) {
                Ok(p) => DcpsReply::OfferedDeadlineMissedStatus(
                    p.get_offered_deadline_missed_status(&publisher_handle, &data_writer_handle),
                ),
                Err(e) => DcpsReply::OfferedDeadlineMissedStatus(Err(e)),
            },
            DcpsMail::Writer(WriterServiceMail::EnableDataWriter {
                participant_handle,
                publisher_handle,
                data_writer_handle,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => {
                    DcpsReply::Ok(p.enable_data_writer(&publisher_handle, &data_writer_handle, now))
                }
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Writer(WriterServiceMail::SetDataWriterQos {
                participant_handle,
                publisher_handle,
                data_writer_handle,
                qos,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => DcpsReply::Ok(p.set_data_writer_qos(
                    &publisher_handle,
                    &data_writer_handle,
                    *qos,
                    now,
                )),
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Subscriber(SubscriberServiceMail::CreateDataReader(p)) => {
                match self
                    .domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &p.participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                {
                    Ok(participant) => DcpsReply::InstanceHandle(participant.create_data_reader(
                        &p.subscriber_handle,
                        p.topic_name,
                        p.qos,
                        p.dcps_listener,
                        p.listener_mask,
                        &self.runtime,
                        now,
                    )),
                    Err(e) => DcpsReply::InstanceHandle(Err(e)),
                }
            }
            DcpsMail::Subscriber(SubscriberServiceMail::DeleteDataReader {
                participant_handle,
                subscriber_handle,
                datareader_handle,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => {
                    DcpsReply::Ok(p.delete_data_reader(&subscriber_handle, &datareader_handle, now))
                }
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Subscriber(SubscriberServiceMail::LookupDataReader {
                participant_handle,
                subscriber_handle,
                topic_name,
            }) => DcpsReply::OptionInstanceHandle(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.lookup_data_reader(&subscriber_handle, topic_name)),
            ),
            DcpsMail::Subscriber(SubscriberServiceMail::SetDefaultDataReaderQos {
                participant_handle,
                subscriber_handle,
                qos,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.set_default_data_reader_qos(&subscriber_handle, *qos)),
            ),
            DcpsMail::Subscriber(SubscriberServiceMail::GetDefaultDataReaderQos {
                participant_handle,
                subscriber_handle,
            }) => {
                DcpsReply::DataReaderQos(self.find_participant(&participant_handle).and_then(|p| {
                    p.get_default_data_reader_qos(&subscriber_handle)
                        .map(Box::new)
                }))
            }
            DcpsMail::Subscriber(SubscriberServiceMail::SetQos {
                participant_handle,
                subscriber_handle,
                qos,
            }) => DcpsReply::Ok(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.set_subscriber_qos(&subscriber_handle, *qos)),
            ),
            DcpsMail::Subscriber(SubscriberServiceMail::GetSubscriberQos {
                participant_handle,
                subscriber_handle,
            }) => DcpsReply::SubscriberQos(
                self.find_participant(&participant_handle)
                    .and_then(|p| p.get_subscriber_qos(&subscriber_handle)),
            ),
            DcpsMail::Subscriber(SubscriberServiceMail::SetListener {
                participant_handle,
                subscriber_handle,
                dcps_listener,
                listener_mask,
            }) => DcpsReply::Ok(
                self.domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                    .and_then(|p| {
                        p.set_subscriber_listener(
                            &subscriber_handle,
                            dcps_listener,
                            listener_mask,
                            &self.runtime,
                        )
                    }),
            ),
            DcpsMail::Reader(ReaderServiceMail::Enable {
                participant_handle,
                subscriber_handle,
                data_reader_handle,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => DcpsReply::Ok(p.enable_data_reader(
                    &subscriber_handle,
                    &data_reader_handle,
                    now,
                )),
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Reader(ReaderServiceMail::Read(p)) => {
                match self
                    .domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &p.participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                {
                    Ok(participant) => DcpsReply::Samples(participant.read(
                        &p.subscriber_handle,
                        &p.data_reader_handle,
                        p.max_samples,
                        &p.sample_states,
                        &p.view_states,
                        &p.instance_states,
                        &p.specific_instance_handle,
                    )),
                    Err(e) => DcpsReply::Samples(Err(e)),
                }
            }
            DcpsMail::Reader(ReaderServiceMail::Take(p)) => {
                match self
                    .domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &p.participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                {
                    Ok(participant) => DcpsReply::Samples(participant.take(
                        &p.subscriber_handle,
                        &p.data_reader_handle,
                        p.max_samples,
                        &p.sample_states,
                        &p.view_states,
                        &p.instance_states,
                        &p.specific_instance_handle,
                    )),
                    Err(e) => DcpsReply::Samples(Err(e)),
                }
            }
            DcpsMail::Reader(ReaderServiceMail::ReadNextInstance(p)) => {
                match self
                    .domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &p.participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                {
                    Ok(participant) => DcpsReply::Samples(participant.read_next_instance(
                        &p.subscriber_handle,
                        &p.data_reader_handle,
                        p.max_samples,
                        &p.previous_handle,
                        &p.sample_states,
                        &p.view_states,
                        &p.instance_states,
                    )),
                    Err(e) => DcpsReply::Samples(Err(e)),
                }
            }
            DcpsMail::Reader(ReaderServiceMail::TakeNextInstance(p)) => {
                match self
                    .domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &p.participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                {
                    Ok(participant) => DcpsReply::Samples(participant.take_next_instance(
                        &p.subscriber_handle,
                        &p.data_reader_handle,
                        p.max_samples,
                        &p.previous_handle,
                        &p.sample_states,
                        &p.view_states,
                        &p.instance_states,
                    )),
                    Err(e) => DcpsReply::Samples(Err(e)),
                }
            }
            DcpsMail::Reader(ReaderServiceMail::GetSubscriptionMatchedStatus {
                participant_handle,
                subscriber_handle,
                data_reader_handle,
            }) => match self.find_participant(&participant_handle) {
                Ok(p) => DcpsReply::SubscriptionMatchedStatus(
                    p.get_subscription_matched_status(&subscriber_handle, &data_reader_handle),
                ),
                Err(e) => DcpsReply::SubscriptionMatchedStatus(Err(e)),
            },
            DcpsMail::Reader(ReaderServiceMail::GetMatchedPublicationData {
                participant_handle,
                subscriber_handle,
                data_reader_handle,
                publication_handle,
            }) => DcpsReply::PublicationBuiltinTopicData(
                self.find_participant(&participant_handle).and_then(|p| {
                    p.get_matched_publication_data(
                        &subscriber_handle,
                        &data_reader_handle,
                        &publication_handle,
                    )
                    .map(Box::new)
                }),
            ),
            DcpsMail::Reader(ReaderServiceMail::GetMatchedPublications {
                participant_handle,
                subscriber_handle,
                data_reader_handle,
            }) => {
                DcpsReply::InstanceHandleList(self.find_participant(&participant_handle).and_then(
                    |p| p.get_matched_publications(&subscriber_handle, &data_reader_handle),
                ))
            }
            DcpsMail::Reader(ReaderServiceMail::SetQos {
                participant_handle,
                subscriber_handle,
                data_reader_handle,
                qos,
            }) => match self
                .domain_participant_list
                .iter_mut()
                .find(|x| x.get_instance_handle() == &participant_handle)
                .ok_or(DdsError::AlreadyDeleted)
            {
                Ok(p) => DcpsReply::Ok(p.set_data_reader_qos(
                    &subscriber_handle,
                    &data_reader_handle,
                    *qos,
                    now,
                )),
                Err(e) => DcpsReply::Ok(Err(e)),
            },
            DcpsMail::Reader(ReaderServiceMail::GetQos {
                participant_handle,
                subscriber_handle,
                data_reader_handle,
            }) => {
                DcpsReply::DataReaderQos(self.find_participant(&participant_handle).and_then(|p| {
                    p.get_data_reader_qos(&subscriber_handle, &data_reader_handle)
                        .map(Box::new)
                }))
            }
            DcpsMail::Reader(ReaderServiceMail::SetListener {
                participant_handle,
                subscriber_handle,
                data_reader_handle,
                dcps_listener,
                listener_mask,
            }) => DcpsReply::Ok(
                self.domain_participant_list
                    .iter_mut()
                    .find(|x| x.get_instance_handle() == &participant_handle)
                    .ok_or(DdsError::AlreadyDeleted)
                    .and_then(|p| {
                        p.set_data_reader_listener(
                            &subscriber_handle,
                            &data_reader_handle,
                            dcps_listener,
                            listener_mask,
                            &self.runtime,
                        )
                    }),
            ),
            DcpsMail::StatusCondition(StatusConditionMail::GetStatusConditionEnabledStatuses {
                entity,
            }) => DcpsReply::StatusMask(self.get_status_condition_enabled_statuses(entity)),
            DcpsMail::StatusCondition(StatusConditionMail::GetStatusConditionTriggerValue {
                entity,
            }) => DcpsReply::TriggerValue(self.get_status_condition_trigger_value(entity)),
            DcpsMail::StatusCondition(StatusConditionMail::RegisterNotification {
                entity,
                notification_sender,
            }) => DcpsReply::Ok(self.register_notification(entity, notification_sender)),
            DcpsMail::StatusCondition(StatusConditionMail::SetStatusConditionEnabledStatuses {
                entity,
                status_mask,
            }) => DcpsReply::Ok(self.set_status_condition_enabled_statuses(entity, status_mask)),
            DcpsMail::Message(MessageServiceMail::NotifyAcknowledgments {
                participant_handle,
                publisher_handle,
                data_writer_handle,
                reply_sender,
            }) => {
                match self.find_participant(&participant_handle) {
                    Ok(p) => p.notify_acknowledgments(
                        &publisher_handle,
                        &data_writer_handle,
                        reply_sender,
                    ),
                    Err(e) => reply_sender.send(Err(e)),
                }
                DcpsReply::Ok(Ok(()))
            }
            DcpsMail::Message(MessageServiceMail::NotifyHistoricalData {
                participant_handle,
                subscriber_handle,
                data_reader_handle,
                reply_sender,
            }) => {
                match self.find_participant(&participant_handle) {
                    Ok(p) => p.notify_historical_data(
                        &subscriber_handle,
                        &data_reader_handle,
                        reply_sender,
                    ),
                    Err(e) => reply_sender.send(Err(e)),
                }
                DcpsReply::Ok(Ok(()))
            }
        }
    }
}
