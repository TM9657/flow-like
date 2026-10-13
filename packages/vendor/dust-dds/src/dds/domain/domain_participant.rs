use super::domain_participant_listener::DomainParticipantListener;
use crate::{
    builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    dds_async::domain_participant::DomainParticipantAsync,
    infrastructure::{
        domain::DomainId,
        error::DdsResult,
        instance::InstanceHandle,
        qos::{DomainParticipantQos, PublisherQos, QosKind, SubscriberQos, TopicQos},
        status::StatusKind,
        time::{Duration, Time},
    },
    publication::{publisher::Publisher, publisher_listener::PublisherListener},
    std_runtime::executor::block_on,
    subscription::{subscriber::Subscriber, subscriber_listener::SubscriberListener},
    topic_definition::{
        content_filtered_topic::ContentFilteredTopic, topic::Topic,
        topic_description::TopicDescription, topic_listener::TopicListener,
    },
    xtypes::{dynamic_type::DynamicType, type_support::TypeSupport},
};
use alloc::{string::String, vec::Vec};

/// The [`DomainParticipant`] represents the participation of the application on a communication plane that isolates applications running on the
/// same set of physical computers from each other. A domain establishes a *virtual network* linking all applications that
/// share the same domain_id and isolating them from applications running on different domains. In this way, several
/// independent distributed applications can coexist in the same physical network without interfering, or even being aware
/// of each other.
///
/// The [`DomainParticipant`] object plays several roles:
/// - It acts as a container for all other Entity objects
/// - It acts as factory for the [`Publisher`], [`Subscriber`] and [`Topic`] Entity objects
/// - It provides administration services in the domain, offering operations that allow the application to 'ignore' locally any
///   information about a given participant ([`DomainParticipant::ignore_participant()`]), publication ([`DomainParticipant::ignore_publication()`]), subscription
///   ([`DomainParticipant::ignore_subscription()`]), or topic ([`DomainParticipant::ignore_topic()`]).
///
/// The following operations may be called even if the [`DomainParticipant`] is not enabled. Other operations will return a NotEnabled error if called on a disabled [`DomainParticipant`]:
/// - Operations defined at the base-class level namely, [`DomainParticipant::set_qos()`], [`DomainParticipant::get_qos()`], [`DomainParticipant::set_listener()`], and [`DomainParticipant::enable()`].
/// - Factory methods: [`DomainParticipant::create_topic()`], [`DomainParticipant::create_publisher()`], [`DomainParticipant::create_subscriber()`], [`DomainParticipant::delete_topic()`], [`DomainParticipant::delete_publisher()`],
///   [`DomainParticipant::delete_subscriber()`]
/// - Operations that access the status: [`DomainParticipant::get_statuscondition()`]
#[derive(Clone)]
pub struct DomainParticipant {
    participant_async: DomainParticipantAsync,
}

impl From<DomainParticipantAsync> for DomainParticipant {
    fn from(participant_async: DomainParticipantAsync) -> Self {
        Self { participant_async }
    }
}

impl From<DomainParticipant> for DomainParticipantAsync {
    fn from(value: DomainParticipant) -> Self {
        value.participant_async
    }
}

impl DomainParticipant {
    /// This operation creates a [`Publisher`] with the desired QoS policies and attaches to it the specified [`PublisherListener`].
    /// If the specified QoS policies are not consistent, the operation will fail and no [`Publisher`] will be created.
    /// The value [`QosKind::Default`] can be used to indicate that the Publisher should be created with the default
    /// Publisher QoS set in the factory. The use of this value is equivalent to the application obtaining the default Publisher QoS by
    /// means of the operation [`DomainParticipant::get_default_publisher_qos()`] and using the resulting QoS to create the [`Publisher`].
    /// The created [`Publisher`] belongs to the [`DomainParticipant`] that is its factory.
    /// In case of failure, the operation will return an error and no [`Publisher`] will be created.
    #[tracing::instrument(skip(self, a_listener))]
    pub fn create_publisher(
        &self,
        qos: QosKind<PublisherQos>,
        a_listener: Option<impl PublisherListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<Publisher> {
        block_on(
            self.participant_async
                .create_publisher(qos, a_listener, mask),
        )
        .map(Publisher::from)
    }

    /// This operation deletes an existing [`Publisher`].
    /// A [`Publisher`] cannot be deleted if it has any attached [`DataWriter`](crate::publication::data_writer::DataWriter) objects. If [`DomainParticipant::delete_publisher()`]
    /// is called on a [`Publisher`] with existing [`DataWriter`](crate::publication::data_writer::DataWriter) objects, it will return a
    /// [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError) error.
    /// The [`DomainParticipant::delete_publisher()`] operation must be called on the same [`DomainParticipant`] object used to create the [`Publisher`].
    /// If [`DomainParticipant::delete_publisher()`] is called on a different [`DomainParticipant`], the operation will have no effect and it will return
    /// a PreconditionNotMet error.
    #[tracing::instrument(skip(self, a_publisher))]
    pub fn delete_publisher(&self, a_publisher: &Publisher) -> DdsResult<()> {
        block_on(
            self.participant_async
                .delete_publisher(a_publisher.publisher_async()),
        )
    }

    /// This operation creates a [`Subscriber`] with the desired QoS policies and attaches to it the specified [`SubscriberListener`].
    /// If the specified QoS policies are not consistent, the operation will fail and no [`Subscriber`] will be created.
    /// The value [`QosKind::Default`] can be used to indicate that the [`Subscriber`] should be created with the
    /// default Subscriber QoS set in the factory. The use of this value is equivalent to the application obtaining the default
    /// Subscriber QoS by means of the operation [`Self::get_default_subscriber_qos()`] and using the resulting QoS to create the
    /// [`Subscriber`].
    /// The created [`Subscriber`] belongs to the [`DomainParticipant`] that is its factory.
    /// In case of failure, the operation will return an error and no [`Subscriber`] will be created.
    #[tracing::instrument(skip(self, a_listener))]
    pub fn create_subscriber(
        &self,
        qos: QosKind<SubscriberQos>,
        a_listener: Option<impl SubscriberListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<Subscriber> {
        block_on(
            self.participant_async
                .create_subscriber(qos, a_listener, mask),
        )
        .map(Subscriber::from)
    }

    /// This operation deletes an existing [`Subscriber`].
    /// A [`Subscriber`] cannot be deleted if it has any attached [`DataReader`](crate::subscription::data_reader::DataReader) objects. If the [`DomainParticipant::delete_subscriber()`] operation is called on a
    /// [`Subscriber`] with existing [`DataReader`](crate::subscription::data_reader::DataReader) objects, it will return [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError).
    /// The [`DomainParticipant::delete_subscriber()`] operation must be called on the same [`DomainParticipant`] object used to create the Subscriber. If
    /// it is called on a different [`DomainParticipant`], the operation will have no effect and it will return
    /// [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError).
    #[tracing::instrument(skip(self, a_subscriber))]
    pub fn delete_subscriber(&self, a_subscriber: &Subscriber) -> DdsResult<()> {
        block_on(
            self.participant_async
                .delete_subscriber(a_subscriber.subscriber_async()),
        )
    }

    /// This operation creates a [`Topic`] with the desired QoS policies and attaches to it the specified [`TopicListener`].
    /// If the specified QoS policies are not consistent, the operation will fail and no [`Topic`] will be created.
    /// The value [`QosKind::Default`] can be used to indicate that the [`Topic`] should be created with the default Topic QoS
    /// set in the factory. The use of this value is equivalent to the application obtaining the default Topic QoS by means of the
    /// operation [`DomainParticipant::get_default_topic_qos`] and using the resulting QoS to create the [`Topic`].
    /// The created [`Topic`] belongs to the [`DomainParticipant`] that is its factory.
    /// In case of failure, the operation will return an error and no [`Topic`] will be created.
    #[tracing::instrument(skip(self, a_listener))]
    pub fn create_topic<Foo>(
        &self,
        topic_name: &str,
        type_name: &str,
        qos: QosKind<TopicQos>,
        a_listener: Option<impl TopicListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<Topic>
    where
        Foo: TypeSupport,
    {
        block_on(
            self.participant_async
                .create_topic::<Foo>(topic_name, type_name, qos, a_listener, mask),
        )
        .map(Topic::from)
    }

    /// This operation create a ['Topic'] is equivalent to [`DomainParticipant::create_topic`] but the DynamicType information
    /// is passed as a function argument instead of retrieved from the Foo type. This allow the user to communicate using
    /// arbitrary dynamic types which can also be defined at runtime.
    #[tracing::instrument(skip(self, a_listener, dynamic_type_representation))]
    pub fn create_dynamic_topic(
        &self,
        topic_name: &str,
        type_name: &str,
        qos: QosKind<TopicQos>,
        a_listener: Option<impl TopicListener + Send + 'static>,
        mask: &[StatusKind],
        dynamic_type_representation: DynamicType<'static>,
    ) -> DdsResult<Topic> {
        block_on(self.participant_async.create_dynamic_topic(
            topic_name,
            type_name,
            qos,
            a_listener,
            mask,
            dynamic_type_representation,
        ))
        .map(Topic::from)
    }

    /// This operation deletes a [`Topic`].
    /// The deletion of a [`Topic`] is not allowed if there are any existing [`DataReader`](crate::subscription::data_reader::DataReader) or [`DataWriter`](crate::publication::data_writer::DataWriter)
    /// objects that are using the [`Topic`]. If the [`DomainParticipant::delete_topic()`] operation is called on a [`Topic`] with any of these existing objects attached to
    /// it, it will return [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError).
    /// The [`DomainParticipant::delete_topic()`] operation must be called on the same [`DomainParticipant`] object used to create the [`Topic`]. If [`DomainParticipant::delete_topic()`] is
    /// called on a different [`DomainParticipant`], the operation will have no effect and it will return [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError).
    #[tracing::instrument(skip(self, a_topic))]
    pub fn delete_topic(&self, a_topic: &Topic) -> DdsResult<()> {
        block_on(self.participant_async.delete_topic(&a_topic.clone().into()))
    }

    /// This operation creates a ContentFilteredTopic. which can be used to do content-based subscriptions.
    /// The related Topic being subscribed to is specified by means of the related_topic parameter. The ContentFilteredTopic only
    /// relates to samples published under that Topic, filtered according to their content. The filtering is done by means of evaluating a
    /// logical expression that involves the values of some of the data-fields in the sample. The logical expression is derived from the
    /// filter_expression and expression_parameters arguments
    #[tracing::instrument(skip(self, related_topic))]
    pub fn create_contentfilteredtopic(
        &self,
        name: &str,
        related_topic: &Topic,
        filter_expression: String,
        expression_parameters: Vec<String>,
    ) -> DdsResult<ContentFilteredTopic> {
        block_on(self.participant_async.create_contentfilteredtopic(
            name,
            &related_topic.clone().into(),
            filter_expression,
            expression_parameters,
        ))
        .map(ContentFilteredTopic::from)
    }

    /// This operation deletes a ContentFilteredTopic.
    ///
    /// The deletion of a ContentFilteredTopic is not allowed if there are any existing DataReader objects that are using the
    /// ContentFilteredTopic. If the delete_contentfilteredtopic operation is called on a ContentFilteredTopic with existing
    /// DataReader objects attached to it, it will return PRECONDITION_NOT_MET.
    ///
    /// The delete_contentfilteredtopic operation must be called on the same DomainParticipant object used to create the
    /// ContentFilteredTopic. If delete_contentfilteredtopic is called on a different DomainParticipant, the operation will have no
    /// effect and it will return PRECONDITION_NOT_MET.
    #[tracing::instrument(skip(self, a_contentfilteredtopic))]
    pub fn delete_contentfilteredtopic(
        &self,
        a_contentfilteredtopic: &ContentFilteredTopic,
    ) -> DdsResult<()> {
        block_on(
            self.participant_async
                .delete_contentfilteredtopic(&a_contentfilteredtopic.clone().into()),
        )
    }

    /// This operation gives access to an existing (or ready to exist) enabled [`Topic`], based on its name. The operation takes
    /// as arguments the name of the [`Topic`], a timeout and the type as a generic type argument `Foo`.
    /// If a [`Topic`] of the same name and type already exists, it gives access to it, otherwise it waits (blocks the caller) until another mechanism
    /// creates it (or the specified timeout occurs). This other mechanism can be another thread, a configuration tool, or some other
    /// middleware service. Note that the [`Topic`] is a local object that acts as a 'proxy' to designate the global concept of topic.
    /// Middleware implementations could choose to propagate topics and make remotely created topics locally available.
    /// A [`Topic`] obtained by means of [`DomainParticipant::find_topic()`], must also be deleted by means of [`DomainParticipant::delete_topic()`] so that the local resources can be
    /// released. If a [`Topic`] is obtained multiple times by means of [`DomainParticipant::find_topic()`] or [`DomainParticipant::create_topic()`], it must also be deleted that same number
    /// of times using [`DomainParticipant::delete_topic()`].
    /// Regardless of whether the middleware chooses to propagate topics, the [`DomainParticipant::delete_topic()`] operation deletes only the local proxy.
    /// If the operation times-out, a [`DdsError::Timeout`](crate::infrastructure::error::DdsError) error is returned.
    #[tracing::instrument(skip(self))]
    pub fn find_topic<Foo>(&self, topic_name: &str, timeout: Duration) -> DdsResult<Topic>
    where
        Foo: TypeSupport,
    {
        block_on(
            self.participant_async
                .find_topic::<Foo>(topic_name, timeout),
        )
        .map(Topic::from)
    }

    /// This operation gives access to an existing locally-created [`Topic`], based on its name and type. The
    /// operation takes as argument the name of the [`Topic`] and the type as a generic type argument `Foo`.
    /// If a [`Topic`] of the same name already exists, it gives access to it, otherwise it returns a [`None`] value. The operation
    /// never blocks.
    /// The operation [`DomainParticipant::lookup_topicdescription()`] may be used to locate any locally-created [`Topic`].
    /// Unlike [`DomainParticipant::find_topic()`], the operation [`DomainParticipant::lookup_topicdescription()`] searches only among the locally created topics. Therefore, it should
    /// never create a new [`Topic`]. The [`Topic`] returned by [`DomainParticipant::lookup_topicdescription()`] does not require any extra
    /// deletion. It is still possible to delete the [`Topic`] returned by [`DomainParticipant::lookup_topicdescription()`], provided it has no readers or
    /// writers, but then it is really deleted and subsequent lookups will fail.
    /// If the operation fails to locate a [`Topic`], the operation succeeds and a [`None`] value is returned.
    #[tracing::instrument(skip(self))]
    pub fn lookup_topicdescription(
        &self,
        topic_name: &str,
    ) -> DdsResult<Option<impl TopicDescription>> {
        block_on(self.participant_async.lookup_topicdescription(topic_name))
    }

    /// This operation allows access to the built-in [`Subscriber`]. Each [`DomainParticipant`] contains several built-in [`Topic`] objects as
    /// well as corresponding [`DataReader`](crate::subscription::data_reader::DataReader) objects to access them. All these [`DataReader`](crate::subscription::data_reader::DataReader) objects belong to a single built-in [`Subscriber`].
    /// The built-in topics are used to communicate information about other [`DomainParticipant`], [`Topic`], [`DataReader`](crate::subscription::data_reader::DataReader), and [`DataWriter`](crate::publication::data_writer::DataWriter)
    /// objects.
    #[tracing::instrument(skip(self))]
    pub fn get_builtin_subscriber(&self) -> Subscriber {
        Subscriber::from(self.participant_async.get_builtin_subscriber())
    }

    /// This operation allows an application to instruct the Service to locally ignore a remote domain participant. From that point
    /// onwards the Service will locally behave as if the remote participant did not exist. This means it will ignore any topic,
    /// publication, or subscription that originates on that domain participant.
    /// This operation can be used, in conjunction with the discovery of remote participants offered by means of the
    /// *DCPSParticipant* built-in [`Topic`], to provide, for example, access control.
    /// Application data can be associated with a [`DomainParticipant`] by means of the [`UserDataQosPolicy`](crate::infrastructure::qos_policy::UserDataQosPolicy).
    /// This application data is propagated as a field in the built-in topic and can be used by an application to implement its own access control policy.
    /// The domain participant to ignore is identified by the `handle` argument. This handle is the one that appears in the [`SampleInfo`](crate::subscription::sample_info::SampleInfo)
    /// retrieved when reading the data-samples available for the built-in DataReader to the *DCPSParticipant* topic. The built-in
    /// [`DataReader`](crate::subscription::data_reader::DataReader) is read with the same read/take operations used for any DataReader.
    /// The [`DomainParticipant::ignore_participant()`] operation is not reversible.
    #[tracing::instrument(skip(self))]
    pub fn ignore_participant(&self, handle: InstanceHandle) -> DdsResult<()> {
        block_on(self.participant_async.ignore_participant(handle))
    }

    /// This operation allows an application to instruct the Service to locally ignore a remote topic. This means it will locally ignore any
    /// publication or subscription to the Topic.
    /// This operation can be used to save local resources when the application knows that it will never publish or subscribe to data
    /// under certain topics.
    /// The Topic to ignore is identified by the handle argument. This handle is the one that appears in the [`SampleInfo`](crate::subscription::sample_info::SampleInfo) retrieved when
    /// reading the data-samples from the built-in [`DataReader`](crate::subscription::data_reader::DataReader) to the *DCPSTopic* topic.
    /// The [`DomainParticipant::ignore_topic()`] operation is not reversible.
    #[tracing::instrument(skip(self))]
    pub fn ignore_topic(&self, handle: InstanceHandle) -> DdsResult<()> {
        block_on(self.participant_async.ignore_topic(handle))
    }

    /// This operation allows an application to instruct the Service to locally ignore a remote publication; a publication is defined by
    /// the association of a topic name, and user data and partition set on the Publisher. After this call, any data written related to that publication will be ignored.
    /// The DataWriter to ignore is identified by the handle argument. This handle is the one that appears in the [`SampleInfo`](crate::subscription::sample_info::SampleInfo) retrieved
    /// when reading the data-samples from the built-in [`DataReader`](crate::subscription::data_reader::DataReader) to the *DCPSPublication* topic.
    /// The [`DomainParticipant::ignore_publication()`] operation is not reversible.
    #[tracing::instrument(skip(self))]
    pub fn ignore_publication(&self, handle: InstanceHandle) -> DdsResult<()> {
        block_on(self.participant_async.ignore_publication(handle))
    }

    /// This operation allows an application to instruct the Service to locally ignore a remote subscription; a subscription is defined by
    /// the association of a topic name, and user data and partition set on the Subscriber.
    /// After this call, any data received related to that subscription will be ignored.
    /// The DataReader to ignore is identified by the handle argument. This handle is the one that appears in the [`SampleInfo`](crate::subscription::sample_info::SampleInfo)
    /// retrieved when reading the data-samples from the built-in [`DataReader`](crate::subscription::data_reader::DataReader) to the *DCPSSubscription* topic.
    /// The [`DomainParticipant::ignore_subscription()`] operation is not reversible.
    #[tracing::instrument(skip(self))]
    pub fn ignore_subscription(&self, handle: InstanceHandle) -> DdsResult<()> {
        block_on(self.participant_async.ignore_subscription(handle))
    }

    /// This operation retrieves the [`DomainId`] used to create the DomainParticipant. The [`DomainId`] identifies the DDS domain to
    /// which the [`DomainParticipant`] belongs. Each DDS domain represents a separate data *communication plane* isolated from other domains.
    #[tracing::instrument(skip(self))]
    pub fn get_domain_id(&self) -> DomainId {
        self.participant_async.get_domain_id()
    }

    /// This operation deletes all the entities that were created by means of the *create* operations on the DomainParticipant. That is,
    /// it deletes all contained [`Publisher`], [`Subscriber`] and [`Topic`] entities.
    /// Prior to deleting each contained entity, this operation will recursively call the corresponding `delete_contained_entities()`
    /// operation on each contained entity (if applicable). This pattern is applied recursively. In this manner the operation
    /// [`DomainParticipant::delete_contained_entities()`] will end up deleting all the entities recursively contained in the
    /// [`DomainParticipant`], that is also the [`DataWriter`](crate::publication::data_writer::DataWriter), [`DataReader`](crate::subscription::data_reader::DataReader).
    /// The operation will return [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError) if the any of the contained entities is in a state where it cannot be
    /// deleted.
    /// Once this operation returns successfully, the application may delete the [`DomainParticipant`] knowing that it has no
    /// contained entities.
    #[tracing::instrument(skip(self))]
    pub fn delete_contained_entities(&self) -> DdsResult<()> {
        block_on(self.participant_async.delete_contained_entities())
    }

    /// This operation manually asserts the liveliness of the [`DomainParticipant`]. This is used in combination
    /// with the [`LivelinessQosPolicy`](crate::infrastructure::qos_policy::LivelinessQosPolicy)
    /// to indicate to the Service that the entity remains active.
    /// This operation needs to only be used if the [`DomainParticipant`] contains  [`DataWriter`](crate::publication::data_writer::DataWriter) entities with the LIVELINESS set to
    /// MANUAL_BY_PARTICIPANT and it only affects the liveliness of those  [`DataWriter`](crate::publication::data_writer::DataWriter) entities. Otherwise, it has no effect.
    /// NOTE: Writing data via the write operation on a  [`DataWriter`](crate::publication::data_writer::DataWriter) asserts liveliness on the DataWriter itself and its
    /// [`DomainParticipant`]. Consequently the use of this operation is only needed if the application is not writing data regularly.
    #[tracing::instrument(skip(self))]
    pub fn assert_liveliness(&self) -> DdsResult<()> {
        block_on(self.participant_async.assert_liveliness())
    }

    /// This operation sets a default value of the Publisher QoS policies which will be used for newly created [`Publisher`] entities in the
    /// case where the QoS policies are defaulted in the [`DomainParticipant::create_publisher()`] operation.
    /// This operation will check that the resulting policies are self consistent; if they are not, the operation will have no effect and
    /// return [`DdsError::InconsistenPolicy`](crate::infrastructure::error::DdsError).
    /// The special value [`QosKind::Default`] may be passed to this operation to indicate that the default QoS should be
    /// reset back to the initial values the factory would use, that is the values the default values of [`PublisherQos`].
    #[tracing::instrument(skip(self))]
    pub fn set_default_publisher_qos(&self, qos: QosKind<PublisherQos>) -> DdsResult<()> {
        block_on(self.participant_async.set_default_publisher_qos(qos))
    }

    /// This operation retrieves the default value of the Publisher QoS, that is, the QoS policies which will be used for newly created
    /// [`Publisher`] entities in the case where the QoS policies are defaulted in the [`DomainParticipant::create_publisher()`] operation.
    /// The values retrieved by this operation will match the set of values specified on the last successful call to
    /// [`DomainParticipant::set_default_publisher_qos()`], or else, if the call was never made, the default values of the [`PublisherQos`].
    #[tracing::instrument(skip(self))]
    pub fn get_default_publisher_qos(&self) -> DdsResult<PublisherQos> {
        block_on(self.participant_async.get_default_publisher_qos())
    }

    /// This operation sets a default value of the Subscriber QoS policies that will be used for newly created [`Subscriber`] entities in the
    /// case where the QoS policies are defaulted in the [`DomainParticipant::create_subscriber()`] operation.
    /// This operation will check that the resulting policies are self consistent; if they are not, the operation will have no effect and
    /// return [`DdsError::InconsistenPolicy`](crate::infrastructure::error::DdsError).
    /// The special value [`QosKind::Default`] may be passed to this operation to indicate that the default QoS should be
    /// reset back to the initial values the factory would use, that is the default values of [`SubscriberQos`].
    #[tracing::instrument(skip(self))]
    pub fn set_default_subscriber_qos(&self, qos: QosKind<SubscriberQos>) -> DdsResult<()> {
        block_on(self.participant_async.set_default_subscriber_qos(qos))
    }

    /// This operation retrieves the default value of the Subscriber QoS, that is, the QoS policies which will be used for newly created
    /// [`Subscriber`] entities in the case where the QoS policies are defaulted in the [`DomainParticipant::create_subscriber()`] operation.
    /// The values retrieved by this operation will match the set of values specified on the last successful call to
    /// [`DomainParticipant::set_default_subscriber_qos()`], or else, if the call was never made, the default values of [`SubscriberQos`].
    #[tracing::instrument(skip(self))]
    pub fn get_default_subscriber_qos(&self) -> DdsResult<SubscriberQos> {
        block_on(self.participant_async.get_default_subscriber_qos())
    }

    /// This operation sets a default value of the Topic QoS policies which will be used for newly created [`Topic`] entities in the case
    /// where the QoS policies are defaulted in the [`DomainParticipant::create_topic`] operation.
    /// This operation will check that the resulting policies are self consistent; if they are not, the operation will have no effect and
    /// return [`DdsError::InconsistenPolicy`](crate::infrastructure::error::DdsError).
    /// The special value [`QosKind::Default`] may be passed to this operation to indicate that the default QoS should be reset
    /// back to the initial values the factory would use, that is the default values of [`TopicQos`].
    #[tracing::instrument(skip(self))]
    pub fn set_default_topic_qos(&self, qos: QosKind<TopicQos>) -> DdsResult<()> {
        block_on(self.participant_async.set_default_topic_qos(qos))
    }

    /// This operation retrieves the default value of the Topic QoS, that is, the QoS policies that will be used for newly created [`Topic`]
    /// entities in the case where the QoS policies are defaulted in the [`DomainParticipant::create_topic()`] operation.
    /// The values retrieved by this operation will match the set of values specified on the last successful call to
    /// [`DomainParticipant::set_default_topic_qos()`], or else, if the call was never made, the default values of [`TopicQos`]
    #[tracing::instrument(skip(self))]
    pub fn get_default_topic_qos(&self) -> DdsResult<TopicQos> {
        block_on(self.participant_async.get_default_topic_qos())
    }

    /// This operation retrieves the list of DomainParticipants that have been discovered in the domain and that the application has not
    /// indicated should be *ignored* by means of the [`DomainParticipant::ignore_participant()`] operation.
    #[tracing::instrument(skip(self))]
    pub fn get_discovered_participants(&self) -> DdsResult<Vec<InstanceHandle>> {
        block_on(self.participant_async.get_discovered_participants())
    }

    /// This operation retrieves information on a [`DomainParticipant`] that has been discovered on the network. The participant must
    /// be in the same domain as the participant on which this operation is invoked and must not have been *ignored* by means of the
    /// [`DomainParticipant::ignore_participant()`] operation.
    /// The participant_handle must correspond to such a DomainParticipant. Otherwise, the operation will fail and return
    /// [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError).
    /// Use the operation [`DomainParticipant::get_discovered_participants()`] to find the DomainParticipants that are currently discovered.
    #[tracing::instrument(skip(self))]
    pub fn get_discovered_participant_data(
        &self,
        participant_handle: InstanceHandle,
    ) -> DdsResult<ParticipantBuiltinTopicData> {
        block_on(
            self.participant_async
                .get_discovered_participant_data(participant_handle),
        )
    }

    /// This operation retrieves the list of Topics that have been discovered in the domain and that the application has not indicated
    /// should be *ignored* by means of the [`DomainParticipant::ignore_topic()`] operation.
    #[tracing::instrument(skip(self))]
    pub fn get_discovered_topics(&self) -> DdsResult<Vec<InstanceHandle>> {
        block_on(self.participant_async.get_discovered_topics())
    }

    /// This operation retrieves information on a Topic that has been discovered on the network. The topic must have been created by
    /// a participant in the same domain as the participant on which this operation is invoked and must not have been *ignored* by
    /// means of the [`DomainParticipant::ignore_topic()`] operation.
    /// The `topic_handle` must correspond to such a topic. Otherwise, the operation will fail and return
    /// [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError).
    /// Use the operation [`DomainParticipant::get_discovered_topics()`] to find the topics that are currently discovered.
    #[tracing::instrument(skip(self))]
    pub fn get_discovered_topic_data(
        &self,
        topic_handle: InstanceHandle,
    ) -> DdsResult<TopicBuiltinTopicData> {
        block_on(
            self.participant_async
                .get_discovered_topic_data(topic_handle),
        )
    }

    /// This operation checks whether or not the given `a_handle` represents an Entity that was created from the [`DomainParticipant`].
    /// The containment applies recursively. That is, it applies both to entities ([`Topic`], [`Publisher`], or [`Subscriber`]) created
    /// directly using the [`DomainParticipant`] as well as entities created using a contained [`Publisher`], or [`Subscriber`] as the factory, and
    /// so forth.
    /// The instance handle for an Entity may be obtained from built-in topic data, from various statuses, or from the Entity operation
    /// `get_instance_handle`.
    #[tracing::instrument(skip(self))]
    pub fn contains_entity(&self, a_handle: InstanceHandle) -> DdsResult<bool> {
        block_on(self.participant_async.contains_entity(a_handle))
    }

    /// This operation returns the current value of the time that the service uses to time-stamp data-writes and to set the reception timestamp
    /// for the data-updates it receives.
    #[tracing::instrument(skip(self))]
    pub fn get_current_time(&self) -> DdsResult<Time> {
        block_on(self.participant_async.get_current_time())
    }
}

/// This implementation block contains the Entity operations for the [`DomainParticipant`].
impl DomainParticipant {
    /// This operation is used to set the QoS policies of the Entity and replacing the values of any policies previously set.
    /// Certain policies are *immutable;* they can only be set at Entity creation time, or before the entity is made enabled.
    /// If [`Self::set_qos()`] is invoked after the Entity is enabled and it attempts to change the value of an *immutable* policy, the operation will
    /// fail and returns [`DdsError::ImmutablePolicy`](crate::infrastructure::error::DdsError).
    /// Certain values of QoS policies can be incompatible with the settings of the other policies. This operation will also fail if it specifies
    /// a set of values that once combined with the existing values would result in an inconsistent set of policies. In this case,
    /// the return value is [`DdsError::InconsistentPolicy`](crate::infrastructure::error::DdsError).
    /// The existing set of policies are only changed if the [`Self::set_qos()`] operation succeeds. This is indicated by the [`Ok`] return value. In all
    /// other cases, none of the policies is modified.
    /// The parameter `qos` can be set to [`QosKind::Default`] to indicate that the QoS of the Entity should be changed to match the current default QoS set in the Entity's factory.
    /// The operation [`Self::set_qos()`] cannot modify the immutable QoS so a successful return of the operation indicates that the mutable QoS for the Entity has been
    /// modified to match the current default for the Entity's factory.
    #[tracing::instrument(skip(self))]
    pub fn set_qos(&self, qos: QosKind<DomainParticipantQos>) -> DdsResult<()> {
        block_on(self.participant_async.set_qos(qos))
    }

    /// This operation allows access to the existing set of [`DomainParticipantQos`] policies.
    #[tracing::instrument(skip(self))]
    pub fn get_qos(&self) -> DdsResult<DomainParticipantQos> {
        block_on(self.participant_async.get_qos())
    }

    /// This operation installs a Listener on the Entity. The listener will only be invoked on the changes of communication status
    /// indicated by the specified mask. It is permitted to use [`None`] as the value of the listener. The [`None`] listener behaves
    /// as a Listener whose operations perform no action.
    /// Only one listener can be attached to each Entity. If a listener was already set, the operation [`Self::set_listener()`] will replace it with the
    /// new one. Consequently if the value [`None`] is passed for the listener parameter to the [`Self::set_listener()`] operation, any existing listener
    /// will be removed.
    #[tracing::instrument(skip(self, a_listener))]
    pub fn set_listener(
        &self,
        a_listener: Option<impl DomainParticipantListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        block_on(self.participant_async.set_listener(a_listener, mask))
    }

    /// This operation retrieves the list of communication statuses in the Entity that are 'triggered.' That is, the list of statuses whose
    /// value has changed since the last time the application read the status.
    /// When the entity is first created or if the entity is not enabled, all communication statuses are in the *untriggered* state so the
    /// list returned by the [`Self::get_status_changes`] operation will be empty.
    /// The list of statuses returned by the [`Self::get_status_changes`] operation refers to the status that are triggered on the Entity itself
    /// and does not include statuses that apply to contained entities.
    #[tracing::instrument(skip(self))]
    pub fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        block_on(self.participant_async.get_status_changes())
    }

    /// This operation enables the Entity. Entity objects can be created either enabled or disabled. This is controlled by the value of
    /// the [`EntityFactoryQosPolicy`](crate::infrastructure::qos_policy::EntityFactoryQosPolicy) on the corresponding factory for the Entity.
    /// The default setting of [`EntityFactoryQosPolicy`](crate::infrastructure::qos_policy::EntityFactoryQosPolicy) is such that, by default, it is not necessary to explicitly call enable on newly
    /// created entities.
    /// The [`Self::enable()`] operation is idempotent. Calling [`Self::enable()`] on an already enabled Entity returns [`Ok`] and has no effect.
    /// If an Entity has not yet been enabled, the following kinds of operations may be invoked on it:
    /// - Operations to set or get an Entity's QoS policies (including default QoS policies) and listener
    /// - [`Self::get_statuscondition()`]
    /// - Factory and lookup operations
    /// - [`Self::get_status_changes()`] and other get status operations (although the status of a disabled entity never changes)
    /// Other operations may explicitly state that they may be called on disabled entities; those that do not will return the error
    /// NotEnabled.
    /// It is legal to delete an Entity that has not been enabled by calling the proper operation on its factory.
    /// Entities created from a factory that is disabled, are created disabled regardless of the setting of the
    /// [`EntityFactoryQosPolicy`](crate::infrastructure::qos_policy::EntityFactoryQosPolicy).
    /// Calling enable on an Entity whose factory is not enabled will fail and return [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError).
    /// If the `autoenable_created_entities` field of [`EntityFactoryQosPolicy`](crate::infrastructure::qos_policy::EntityFactoryQosPolicy) is set to [`true`], the [`Self::enable()`] operation on the factory will
    /// automatically enable all entities created from the factory.
    /// The Listeners associated with an entity are not called until the entity is enabled. Conditions associated with an entity that is not
    /// enabled are *inactive*, that is, the operation [`StatusCondition::get_trigger_value()`] will always return `false`.
    #[tracing::instrument(skip(self))]
    pub fn enable(&self) -> DdsResult<()> {
        block_on(self.participant_async.enable())
    }

    /// This operation returns the [`InstanceHandle`] that represents the Entity.
    #[tracing::instrument(skip(self))]
    pub fn get_instance_handle(&self) -> InstanceHandle {
        self.participant_async.get_instance_handle()
    }
}
