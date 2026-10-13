use super::{
    data_reader::DataReader, data_reader_listener::DataReaderListener,
    subscriber_listener::SubscriberListener,
};
use crate::{
    condition::StatusCondition,
    dds_async::subscriber::SubscriberAsync,
    domain::domain_participant::DomainParticipant,
    infrastructure::{
        error::DdsResult,
        instance::InstanceHandle,
        qos::{DataReaderQos, QosKind, SubscriberQos, TopicQos},
        status::{SampleLostStatus, StatusKind},
    },
    std_runtime::executor::block_on,
    topic_definition::topic_description::TopicDescription,
};
use alloc::vec::Vec;

/// A [`Subscriber`] is the object responsible for the actual reception of the data resulting from its subscriptions.
///
/// A [`Subscriber`] acts on the behalf of one or several [`DataReader`] objects that are related to it. When it receives data (from the
/// other parts of the system), it builds the list of concerned [`DataReader`] objects, and then indicates to the application that data is
/// available, through its listener or by enabling related conditions.
#[derive(Clone)]
pub struct Subscriber {
    subscriber_async: SubscriberAsync,
}

impl Subscriber {
    pub(crate) fn subscriber_async(&self) -> &SubscriberAsync {
        &self.subscriber_async
    }
}

impl From<SubscriberAsync> for Subscriber {
    fn from(value: SubscriberAsync) -> Self {
        Self {
            subscriber_async: value,
        }
    }
}

impl Subscriber {
    /// This operation creates a [`DataReader`]. The returned [`DataReader`] will be attached and belong to the [`Subscriber`].
    /// The [`DataReader`] returned by this operation has an associated [`Topic`] and a type `Foo`.
    /// The [`Topic`] passed to this operation must have been created from the same [`DomainParticipant`] that was used to create this
    /// [`Subscriber`]. If the [`Topic`] was created from a different [`DomainParticipant`], the operation will fail and
    /// return a [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError). In case of failure, the operation
    /// will return an error and no writer will be created.
    ///
    /// The special value [`QosKind::Default`] can be used to indicate that the [`DataReader`] should be created with the
    /// default qos set in the factory. The use of this value is equivalent to the application obtaining the default
    /// [`DataReaderQos`] by means of the operation [`Subscriber::get_default_datareader_qos`] and using the resulting qos
    /// to create the [`DataReader`]. A common application pattern to construct the [`DataReaderQos`] to ensure consistency with the
    /// associated [`TopicQos`] is to:
    /// 1. Retrieve the QoS policies on the associated [`Topic`] by means of the [`Topic::get_qos`] operation.
    /// 2. Retrieve the default [`DataReaderQos`] qos by means of the [`Subscriber::get_default_datareader_qos`] operation.
    /// 3. Combine those two qos policies using the [`Subscriber::copy_from_topic_qos`] and selectively modify policies as desired and
    /// use the resulting [`DataReaderQos`] to construct the [`DataReader`].
    #[tracing::instrument(skip(self, a_topic, a_listener))]
    pub fn create_datareader<Foo>(
        &self,
        a_topic: &dyn TopicDescription,
        qos: QosKind<DataReaderQos>,
        a_listener: Option<impl DataReaderListener<Foo> + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<DataReader<Foo>> {
        block_on(
            self.subscriber_async
                .create_datareader::<Foo>(&a_topic, qos, a_listener, mask),
        )
        .map(DataReader::from)
    }

    /// This operation deletes a [`DataReader`] that belongs to the [`Subscriber`]. This operation must be called on the
    /// same [`Subscriber`] object used to create the [`DataReader`]. If [`Subscriber::delete_datareader`] is called on a
    /// different [`Subscriber`], the operation will have no effect and it will return [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError).
    #[tracing::instrument(skip(self, a_datareader))]
    pub fn delete_datareader<Foo>(&self, a_datareader: &DataReader<Foo>) -> DdsResult<()> {
        block_on(
            self.subscriber_async
                .delete_datareader::<Foo>(a_datareader.reader_async()),
        )
    }

    /// This operation retrieves a previously created [`DataReader`] belonging to the [`Subscriber`] that is attached to a [`Topic`].
    /// If no such [`DataReader`] exists, the operation will succeed but return [`None`].
    /// If multiple [`DataReader`] attached to the [`Subscriber`] satisfy this condition, then the operation will return one of them. It is not
    /// specified which one.
    /// The use of this operation on the built-in [`Subscriber`] allows access to the built-in [`DataReader`] entities for the built-in topics.
    #[tracing::instrument(skip(self))]
    pub fn lookup_datareader<Foo>(&self, topic_name: &str) -> DdsResult<Option<DataReader<Foo>>> {
        Ok(
            block_on(self.subscriber_async.lookup_datareader::<Foo>(topic_name))?
                .map(DataReader::from),
        )
    }

    /// This operation invokes the operation [`DataReaderListener::on_data_available`] on the listener objects attached to contained [`DataReader`]
    /// entities with a [`StatusKind::DataAvailable`] that is considered changed.
    /// This operation is typically invoked from the [`SubscriberListener::on_data_on_readers`] operation. That way the
    /// [`SubscriberListener`] can delegate to the [`DataReaderListener`] objects the handling of the data.
    #[tracing::instrument(skip(self))]
    pub fn notify_datareaders(&self) -> DdsResult<()> {
        block_on(self.subscriber_async.notify_datareaders())
    }

    /// This operation returns the [`DomainParticipant`] to which the [`Subscriber`] belongs.
    #[tracing::instrument(skip(self))]
    pub fn get_participant(&self) -> DomainParticipant {
        DomainParticipant::from(self.subscriber_async.get_participant())
    }

    /// This operation allows access to the [`SampleLostStatus`].
    #[tracing::instrument(skip(self))]
    pub fn get_sample_lost_status(&self) -> DdsResult<SampleLostStatus> {
        block_on(self.subscriber_async.get_sample_lost_status())
    }

    /// This operation deletes all the entities that were created by means of the [`Subscriber::create_datareader`] operations.
    /// That is, it deletes all contained [`DataReader`] objects.
    /// he operation will return [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError) if the any of the
    /// contained entities is in a state where it cannot be deleted.
    /// Once this operation returns successfully, the application may delete the [`Subscriber`] knowing that it has no
    /// contained [`DataReader`] objects.
    #[tracing::instrument(skip(self))]
    pub fn delete_contained_entities(&self) -> DdsResult<()> {
        block_on(self.subscriber_async.delete_contained_entities())
    }

    /// This operation sets a default value of the [`DataReaderQos`] which will be used for newly created [`DataReader`] entities in
    /// the case where the qos policies are defaulted in the [`Subscriber::create_datareader`] operation.
    /// This operation will check that the resulting policies are self consistent; if they are not, the operation will have no effect and
    /// return [`DdsError::InconsistentPolicy`](crate::infrastructure::error::DdsError).
    /// The special value [`QosKind::Default`] may be passed to this operation to indicate that the default qos should be
    /// reset back to the initial values the factory would use, that is the default value of [`DataReaderQos`].
    #[tracing::instrument(skip(self))]
    pub fn set_default_datareader_qos(&self, qos: QosKind<DataReaderQos>) -> DdsResult<()> {
        block_on(self.subscriber_async.set_default_datareader_qos(qos))
    }

    /// This operation retrieves the default value of the [`DataReaderQos`], that is, the qos policies which will be used for newly
    /// created [`DataReader`] entities in the case where the qos policies are defaulted in the [`Subscriber::create_datareader`] operation.
    /// The values retrieved by this operation will match the set of values specified on the last successful call to
    /// [`Subscriber::get_default_datareader_qos`], or else, if the call was never made, the default values of [`DataReaderQos`].
    #[tracing::instrument(skip(self))]
    pub fn get_default_datareader_qos(&self) -> DdsResult<DataReaderQos> {
        block_on(self.subscriber_async.get_default_datareader_qos())
    }

    /// This operation copies the policies in the `a_topic_qos` to the corresponding policies in the `a_datareader_qos`.
    /// This is a *convenience* operation most useful in combination with the operations [`Subscriber::get_default_datareader_qos`] and
    /// [`Topic::get_qos`]. This operation can be used to merge the [`DataReader`] default qos policies with the
    /// corresponding ones on the [`Topic`]. The resulting qos can then be used to create a new [`DataReader`], or set its qos.
    /// This operation does not check the resulting `a_datareader_qos` for consistency. This is because the merged `a_datareader_qos`
    /// may not be the final one, as the application can still modify some policies prior to applying the policies to the [`DataReader`].
    #[tracing::instrument]
    pub fn copy_from_topic_qos(
        _a_datareader_qos: &mut DataReaderQos,
        _a_topic_qos: &TopicQos,
    ) -> DdsResult<()> {
        todo!()
    }

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
    pub fn set_qos(&self, qos: QosKind<SubscriberQos>) -> DdsResult<()> {
        block_on(self.subscriber_async.set_qos(qos))
    }

    /// This operation allows access to the existing set of [`SubscriberQos`] policies.
    #[tracing::instrument(skip(self))]
    pub fn get_qos(&self) -> DdsResult<SubscriberQos> {
        block_on(self.subscriber_async.get_qos())
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
        a_listener: Option<impl SubscriberListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        block_on(self.subscriber_async.set_listener(a_listener, mask))
    }

    /// This operation allows access to the [`StatusCondition`] associated with the Entity. The returned
    /// condition can then be added to a [`WaitSet`](crate::infrastructure::wait_set::WaitSet) so that the application can wait for specific status changes
    /// that affect the Entity.
    #[tracing::instrument(skip(self))]
    pub fn get_statuscondition(&self) -> StatusCondition {
        StatusCondition::new(self.subscriber_async.get_statuscondition())
    }

    /// This operation retrieves the list of communication statuses in the Entity that are 'triggered.' That is, the list of statuses whose
    /// value has changed since the last time the application read the status.
    /// When the entity is first created or if the entity is not enabled, all communication statuses are in the *untriggered* state so the
    /// list returned by the [`Self::get_status_changes`] operation will be empty.
    /// The list of statuses returned by the [`Self::get_status_changes`] operation refers to the status that are triggered on the Entity itself
    /// and does not include statuses that apply to contained entities.
    #[tracing::instrument(skip(self))]
    pub fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        block_on(self.subscriber_async.get_status_changes())
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
        block_on(self.subscriber_async.enable())
    }

    /// This operation returns the [`InstanceHandle`] that represents the Entity.
    #[tracing::instrument(skip(self))]
    pub fn get_instance_handle(&self) -> InstanceHandle {
        self.subscriber_async.get_instance_handle()
    }
}
