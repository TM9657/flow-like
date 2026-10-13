use crate::{
    builtin_topics::SubscriptionBuiltinTopicData,
    condition::StatusCondition,
    dds_async::data_writer::DataWriterAsync,
    infrastructure::{
        error::DdsResult,
        instance::InstanceHandle,
        qos::{DataWriterQos, QosKind},
        status::{
            LivelinessLostStatus, OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus,
            PublicationMatchedStatus, StatusKind,
        },
        time::{Duration, Time},
    },
    publication::{data_writer_listener::DataWriterListener, publisher::Publisher},
    std_runtime::executor::{block_on, block_timeout},
    topic_definition::topic::Topic,
    xtypes::type_support::TypeSupport,
};
use alloc::vec::Vec;

/// The [`DataWriter`] allows the application to set the value of the
/// data to be published under a given [`Topic`].
pub struct DataWriter<Foo> {
    writer_async: DataWriterAsync<Foo>,
}

impl<Foo> From<DataWriterAsync<Foo>> for DataWriter<Foo> {
    fn from(value: DataWriterAsync<Foo>) -> Self {
        Self {
            writer_async: value,
        }
    }
}

impl<Foo> Clone for DataWriter<Foo> {
    fn clone(&self) -> Self {
        Self {
            writer_async: self.writer_async.clone(),
        }
    }
}

impl<Foo> DataWriter<Foo> {
    pub(crate) fn writer_async(&self) -> &DataWriterAsync<Foo> {
        &self.writer_async
    }
}

impl<Foo> DataWriter<Foo>
where
    Foo: TypeSupport,
{
    /// This operation informs the Service that the application will be modifying a particular instance.
    /// It gives an opportunity to the Service to pre-configure itself to improve performance. It takes
    /// as a parameter an `instance` (to get the key value) and returns an [`InstanceHandle`] that can be
    /// used in successive [`DataWriter::write`] or [`DataWriter::dispose`] operations.
    /// This operation should be invoked prior to calling any operation that modifies the instance, such as
    /// [`DataWriter::write`], [`DataWriter::write_w_timestamp`], [`DataWriter::dispose`] and [`DataWriter::dispose_w_timestamp`].
    /// The operation may return [`None`] if the Service does not want to allocate any handle for that instance.
    /// This operation may block and return [`DdsError::Timeout`](crate::infrastructure::error::DdsError) or
    /// [`DdsError::OutOfResources`](crate::infrastructure::error::DdsError) under the same circumstances
    /// described for [`DataWriter::write`].
    /// This operation is idempotent. If it is called for an already registered instance, it just returns the already
    /// allocated [`InstanceHandle`]. This may be used to lookup and retrieve the handle allocated to a given instance.
    /// The explicit use of this operation is optional as the application may call directly [`DataWriter::write`]
    /// and specify no [`InstanceHandle`] to indicate that the *key* should be examined to identify the instance.
    #[tracing::instrument(skip(self, instance))]
    pub fn register_instance(&self, instance: Foo) -> DdsResult<Option<InstanceHandle>> {
        block_on(self.writer_async.register_instance(instance))
    }

    /// This operation performs the same function and return the same values as [`DataWriter::register_instance`] and can be used instead of
    /// [`DataWriter::register_instance`] in the cases where the application desires to specify the value for the `source_timestamp`.
    /// The `source_timestamp` potentially affects the relative order in which readers observe events from multiple writers.
    /// For details see [`DestinationOrderQosPolicy`](crate::infrastructure::qos_policy::DestinationOrderQosPolicy).
    #[tracing::instrument(skip(self, instance))]
    pub fn register_instance_w_timestamp(
        &self,
        instance: Foo,
        timestamp: Time,
    ) -> DdsResult<Option<InstanceHandle>> {
        block_on(
            self.writer_async
                .register_instance_w_timestamp(instance, timestamp),
        )
    }

    /// This operation reverses the action of [`DataWriter::register_instance`]. It should only be called on an
    /// instance that is currently registered. This operation should be called just once per instance,
    /// regardless of how many times [`DataWriter::register_instance`] was called for that instance.
    /// This operation informs the Service that the [`DataWriter`] is not intending to modify any more of that
    /// data instance. This operation also indicates that the Service can locally remove all information regarding
    /// that instance. The application should not attempt to use the handle previously allocated to that instance
    /// after calling [`DataWriter::unregister_instance`].
    /// If [`None`] is used as the `handle` argument it indicates that the identity of the instance should
    /// be automatically deduced from the instance (by means of the key).
    /// If handle is any value other than [`None`], then it must correspond to the value returned by register_instance when the
    /// instance (identified by its key) was registered. Otherwise the behavior is as follows:
    /// - If the handle corresponds to an existing instance but does not correspond to the same instance referred by the 'instance'
    /// parameter, the operation fails and returns [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError).
    /// - If the handle does not correspond to an existing instance the operation fails and returns
    /// [`DdsError::BadParameter`](crate::infrastructure::error::DdsError).
    /// If after that, the application wants to modify (write or dispose) the instance, it has to register it again,
    /// or else pass [`None`] as the `handle` value of those operations.
    /// This operation does not indicate that the instance is deleted (that is the purpose of dispose). This operation
    /// just indicates that the [`DataWriter`] no longer has 'anything to say' about the instance.
    /// [`DataReader`](crate::subscription::data_reader::DataReader) entities that are reading the instance will eventually
    /// receive a sample with an [`InstanceStateKind::NotAliveNoWriter`](crate::subscription::sample_info::InstanceStateKind)
    /// if no other [`DataWriter`] entities are writing the instance.
    /// This operation can affect the ownership of the data instance as described
    /// in [`OwnershipQosPolicy`](crate::infrastructure::qos_policy::OwnershipQosPolicy).
    /// If the [`DataWriter`] was the exclusive owner of the instance, then calling [`DataWriter::unregister_instance`]
    /// will relinquish that ownership.
    /// This operation may block and return [`DdsError::Timeout`](crate::infrastructure::error::DdsError) under the
    /// same circumstances described for [`DataWriter::write`].
    #[tracing::instrument(skip(self, instance))]
    pub fn unregister_instance(
        &self,
        instance: Foo,
        handle: Option<InstanceHandle>,
    ) -> DdsResult<()> {
        block_on(self.writer_async.unregister_instance(instance, handle))
    }

    /// This operation performs the same function and returns the same values as [`DataWriter::unregister_instance`] and can
    /// be used instead of [`DataWriter::unregister_instance`] in the cases where the application desires to specify the
    /// value for the `source_timestamp`.
    /// The `source_timestamp` potentially affects the relative order in which readers observe events from multiple writers.
    /// For details see [`DestinationOrderQosPolicy`](crate::infrastructure::qos_policy::DestinationOrderQosPolicy).
    #[tracing::instrument(skip(self, instance))]
    pub fn unregister_instance_w_timestamp(
        &self,
        instance: Foo,
        handle: Option<InstanceHandle>,
        timestamp: Time,
    ) -> DdsResult<()> {
        block_on(
            self.writer_async
                .unregister_instance_w_timestamp(instance, handle, timestamp),
        )
    }

    /// This operation can be used to retrieve the instance key that corresponds to an `handle`. The operation will only fill the
    /// fields that form the key inside the `key_holder` instance.
    /// This operation returns [`DdsError::BadParameter`](crate::infrastructure::error::DdsError) if the `handle` does not
    /// correspond to an existing data object known to the [`DataWriter`].
    #[tracing::instrument(skip(self, key_holder))]
    pub fn get_key_value(&self, key_holder: &mut Foo, handle: InstanceHandle) -> DdsResult<()> {
        block_on(self.writer_async.get_key_value(key_holder, handle))
    }

    /// This operation takes as a parameter an instance and returns an [`InstanceHandle`] that can be used in subsequent operations
    /// that accept an [`InstanceHandle`] as an argument. The `instance` parameter is only used for the purpose of examining the
    /// fields that define the key.
    /// This operation does not register the instance in question. If the instance has not been previously registered, or if for any other
    /// reason the Service is unable to provide an [`InstanceHandle`], the operation will return [`None`].
    #[tracing::instrument(skip(self, instance))]
    pub fn lookup_instance(&self, instance: Foo) -> DdsResult<Option<InstanceHandle>> {
        block_on(self.writer_async.lookup_instance(instance))
    }

    /// This operation modifies the value of a data instance. When this operation is used, the Service will automatically supply the
    /// value of the source timestamp that is made available to [`DataReader`](crate::subscription::data_reader::DataReader)
    /// objects by means of the [`SampleInfo::source_timestamp`](crate::subscription::sample_info::SampleInfo).
    /// As a side effect, this operation asserts liveliness on the [`DataWriter`] itself, the [`Publisher`] and the
    /// [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant).
    /// If [`None`] is used as the `handle` argument this indicates that the identity of the instance should be automatically deduced
    /// from the `data` (by means of the key).
    /// If `handle` is any value other than [`None`], then it must correspond to the value returned by [`DataWriter::register_instance`]
    /// when the instance (identified by its key) was registered. Otherwise the behavior is as follows:
    /// - If the `handle` corresponds to an existing instance but does not correspond to the same instance referred by the 'data'
    /// parameter, the operation fails and returns [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError).
    /// - If the `handle` does not correspond to an existing instance the operation fails and returns [`DdsError::BadParameter`](crate::infrastructure::error::DdsError).
    ///
    /// If the [`ReliabilityQosPolicy`](crate::infrastructure::qos_policy::ReliabilityQosPolicyKind) is set to [`ReliabilityQosPolicyKind::Reliable`](crate::infrastructure::qos_policy::ReliabilityQosPolicyKind) this operation
    /// may block if the modification would cause data to be lost or else cause one of the limits specified in the [`ResourceLimitsQosPolicy`](crate::infrastructure::qos_policy::ResourceLimitsQosPolicy) to be exceeded.
    /// Under these circumstances, the [`ReliabilityQosPolicy::max_blocking_time`](crate::infrastructure::qos_policy::ReliabilityQosPolicy) configures the maximum time the [`DataWriter::write`] operation may block waiting for space to become
    /// available. If [`ReliabilityQosPolicy::max_blocking_time`](crate::infrastructure::qos_policy::ReliabilityQosPolicy) elapses before the [`DataWriter`] is able to store the modification without exceeding the limits,
    /// the write operation will fail and return [`DdsError::Timeout`](crate::infrastructure::error::DdsError).
    /// Specifically, the [`DataWriter::write`] operation may block in the following situations (note that the list may not be exhaustive),
    /// even if configured with [`HistoryQosPolicyKind::KeepLast`](crate::infrastructure::qos_policy::HistoryQosPolicyKind):
    /// - If ([`ResourceLimitsQosPolicy::max_samples`](crate::infrastructure::qos_policy::ResourceLimitsQosPolicy) <
    /// [`ResourceLimitsQosPolicy::max_instances`](crate::infrastructure::qos_policy::ResourceLimitsQosPolicy) * [`HistoryQosPolicy::depth`](crate::infrastructure::qos_policy::HistoryQosPolicy)), then in the
    /// situation where the [`ResourceLimitsQosPolicy::max_samples`](crate::infrastructure::qos_policy::ResourceLimitsQosPolicy) resource limit is exhausted
    /// the Service is allowed to discard samples of some other instance as long as at least one sample remains for such an instance.
    /// If it is still not possible to make space available to store the modification, the writer is allowed to block.
    /// - If ([`ResourceLimitsQosPolicy::max_samples`](crate::infrastructure::qos_policy::ResourceLimitsQosPolicy) < [`ResourceLimitsQosPolicy::max_instances`](crate::infrastructure::qos_policy::ResourceLimitsQosPolicy)),
    /// then the [`DataWriter`] may block regardless of the [`HistoryQosPolicy::depth`](crate::infrastructure::qos_policy::HistoryQosPolicy).
    ///
    /// Instead of blocking, the write operation is allowed to return immediately with the error code [`DdsError::OutOfResources`](crate::infrastructure::error::DdsError)
    /// provided that the reason for blocking would be that the [`ResourceLimitsQosPolicy`](crate::infrastructure::qos_policy::ResourceLimitsQosPolicy)
    /// is exceeded and the service determines that even waiting the [`ReliabilityQosPolicy::max_waiting_time`](crate::infrastructure::qos_policy::ReliabilityQosPolicy) has no
    /// chance of freeing the necessary resources. For example, if the only way to gain the necessary resources would be for the user to unregister an instance.
    #[tracing::instrument(skip(self, data))]
    pub fn write(&self, data: Foo, handle: Option<InstanceHandle>) -> DdsResult<()> {
        block_on(self.writer_async.write(data, handle))
    }

    /// This operation performs the same function and returns the same values as [`DataWriter::write`] and can
    /// be used instead of [`DataWriter::write`] in the cases where the application desires to specify the
    /// value for the `source_timestamp`.
    /// The `source_timestamp` potentially affects the relative order in which readers observe events from multiple writers.
    /// For details see [`DestinationOrderQosPolicy`](crate::infrastructure::qos_policy::DestinationOrderQosPolicy).
    #[tracing::instrument(skip(self, data))]
    pub fn write_w_timestamp(
        &self,
        data: Foo,
        handle: Option<InstanceHandle>,
        timestamp: Time,
    ) -> DdsResult<()> {
        block_on(self.writer_async.write_w_timestamp(data, handle, timestamp))
    }

    /// This operation requests the middleware to delete the data (the actual deletion is postponed until there is no more use for that
    /// data in the whole system). In general, applications are made aware of the deletion by means of operations on the
    /// [`DataReader`](crate::subscription::data_reader::DataReader) objects that already knew the instance.
    /// This operation does not modify the value of the instance. The `handle` parameter is passed just for the purposes of identifying
    /// the instance.
    /// When this operation is used, the Service will automatically supply the value of the source timestamp that is made available
    /// to [`DataReader`](crate::subscription::data_reader::DataReader) objects by means of the
    /// [`SampleInfo::source_timestamp`](crate::subscription::sample_info::SampleInfo).
    /// The constraints on the values of the handle parameter and the corresponding error behavior are the same specified for the
    /// [`DataWriter::unregister_instance`] operation.
    /// This operation may block and return [`DdsError::Timeout`](crate::infrastructure::error::DdsError) or
    /// [`DdsError::OutOfResources`](crate::infrastructure::error::DdsError) under the same circumstances described for [`DataWriter::write`].
    #[tracing::instrument(skip(self, data))]
    pub fn dispose(&self, data: Foo, handle: Option<InstanceHandle>) -> DdsResult<()> {
        block_on(self.writer_async.dispose(data, handle))
    }

    /// This operation performs the same function and returns the same values as [`DataWriter::dispose`] and can
    /// be used instead of [`DataWriter::dispose`] in the cases where the application desires to specify the
    /// value for the `source_timestamp`.
    /// The `source_timestamp` potentially affects the relative order in which readers observe events from multiple writers.
    /// For details see [`DestinationOrderQosPolicy`](crate::infrastructure::qos_policy::DestinationOrderQosPolicy).
    #[tracing::instrument(skip(self, data))]
    pub fn dispose_w_timestamp(
        &self,
        data: Foo,
        handle: Option<InstanceHandle>,
        timestamp: Time,
    ) -> DdsResult<()> {
        block_on(
            self.writer_async
                .dispose_w_timestamp(data, handle, timestamp),
        )
    }
}

impl<Foo> DataWriter<Foo> {
    /// This operation blocks the calling thread until either all data written by the [`DataWriter`] is acknowledged by all
    /// matched [`DataReader`](crate::subscription::data_reader::DataReader) entities that have
    /// [`ReliabilityQosPolicyKind::Reliable`](crate::infrastructure::qos_policy::ReliabilityQosPolicyKind), or else the duration
    /// specified by the `max_wait` parameter elapses, whichever happens first. A return value of [`Ok`] indicates that all the samples
    /// written have been acknowledged by all reliable matched data readers; a return value of [`DdsError::Timeout`](crate::infrastructure::error::DdsError)
    /// indicates that `max_wait` elapsed before all the data was acknowledged.
    /// This operation is intended to be used only if the DataWriter has [`ReliabilityQosPolicyKind::Reliable`](crate::infrastructure::qos_policy::ReliabilityQosPolicyKind).
    /// Otherwise the operation will return immediately with [`Ok`].
    #[tracing::instrument(skip(self))]
    pub fn wait_for_acknowledgments(&self, max_wait: Duration) -> DdsResult<()> {
        block_timeout(
            max_wait.into(),
            self.writer_async.wait_for_acknowledgments(),
        )?
    }

    /// This operation allows access to the [`LivelinessLostStatus`].
    #[tracing::instrument(skip(self))]
    pub fn get_liveliness_lost_status(&self) -> DdsResult<LivelinessLostStatus> {
        block_on(self.writer_async.get_liveliness_lost_status())
    }

    /// This operation allows access to the [`OfferedDeadlineMissedStatus`].
    #[tracing::instrument(skip(self))]
    pub fn get_offered_deadline_missed_status(&self) -> DdsResult<OfferedDeadlineMissedStatus> {
        block_on(self.writer_async.get_offered_deadline_missed_status())
    }

    /// This operation allows access to the [`OfferedIncompatibleQosStatus`].
    #[tracing::instrument(skip(self))]
    pub fn get_offered_incompatible_qos_status(&self) -> DdsResult<OfferedIncompatibleQosStatus> {
        block_on(self.writer_async.get_offered_incompatible_qos_status())
    }

    /// This operation allows access to the [`PublicationMatchedStatus`].
    #[tracing::instrument(skip(self))]
    pub fn get_publication_matched_status(&self) -> DdsResult<PublicationMatchedStatus> {
        block_on(self.writer_async.get_publication_matched_status())
    }

    /// This operation returns the [`Topic`] associated with the [`DataWriter`]. This is the same [`Topic`] that was used to create the [`DataWriter`].
    #[tracing::instrument(skip(self))]
    pub fn get_topic(&self) -> Topic {
        self.writer_async.get_topic().into()
    }

    /// This operation returns the [`Publisher`] to which the [`DataWriter`] object belongs.
    #[tracing::instrument(skip(self))]
    pub fn get_publisher(&self) -> Publisher {
        Publisher::from(self.writer_async.get_publisher())
    }

    /// This operation manually asserts the liveliness of the [`DataWriter`]. This is used in combination with the
    /// [`LivelinessQosPolicy`](crate::infrastructure::qos_policy::LivelinessQosPolicy) to indicate to the Service that the entity remains active.
    /// This operation need only be used if the [`LivelinessQosPolicy`](crate::infrastructure::qos_policy::LivelinessQosPolicy) setting is either
    /// [`LivelinessQosPolicyKind::ManualByParticipant`](crate::infrastructure::qos_policy::LivelinessQosPolicyKind) or
    /// [`LivelinessQosPolicyKind::ManualByTopic`](crate::infrastructure::qos_policy::LivelinessQosPolicyKind). Otherwise, it has no effect.
    /// NOTE: Writing data via the [`DataWriter::write`] operation asserts liveliness on the [`DataWriter`] itself and its
    /// [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant). Consequently the use of this operation is only needed
    /// if the application is not writing data regularly.
    #[tracing::instrument(skip(self))]
    pub fn assert_liveliness(&self) -> DdsResult<()> {
        block_on(self.writer_async.assert_liveliness())
    }

    /// This operation retrieves information on a subscription that is currently *associated* with the [`DataWriter`]; that is, a subscription
    /// with a matching [`Topic`] and compatible QoS that the application has not indicated should be ignored by means of the
    /// [`DomainParticipant::ignore_subscription`](crate::domain::domain_participant::DomainParticipant) operation.
    /// The `subscription_handle` must correspond to a subscription currently associated with the [`DataWriter`], otherwise the operation
    /// will fail and return [`DdsError::BadParameter`](crate::infrastructure::error::DdsError). The operation [`DataWriter::get_matched_subscriptions`]
    /// can be used to find the subscriptions that are currently matched with the [`DataWriter`].
    #[tracing::instrument(skip(self))]
    pub fn get_matched_subscription_data(
        &self,
        subscription_handle: InstanceHandle,
    ) -> DdsResult<SubscriptionBuiltinTopicData> {
        block_on(
            self.writer_async
                .get_matched_subscription_data(subscription_handle),
        )
    }

    /// This operation retrieves the list of subscriptions currently *associated* with the [`DataWriter`]]; that is, subscriptions that have a
    /// matching [`Topic`] and compatible QoS that the application has not indicated should be *ignored* by means of the
    ///  [`DomainParticipant::ignore_subscription`](crate::domain::domain_participant::DomainParticipant) operation.
    /// The handles returned are the ones that are used by the DDS implementation to locally identify the corresponding matched
    /// [`DataReader`](crate::subscription::data_reader::DataReader) entities. These handles match the ones that appear in the
    /// [`SampleInfo::instance_handle`](crate::subscription::sample_info::SampleInfo) field when reading the *DCPSSubscriptions* builtin topic.
    #[tracing::instrument(skip(self))]
    pub fn get_matched_subscriptions(&self) -> DdsResult<Vec<InstanceHandle>> {
        block_on(self.writer_async.get_matched_subscriptions())
    }
}

/// This implementation block contains the Entity operations for the [`DataWriter`].
impl<Foo> DataWriter<Foo> {
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
    pub fn set_qos(&self, qos: QosKind<DataWriterQos>) -> DdsResult<()> {
        block_on(self.writer_async.set_qos(qos))
    }

    /// This operation allows access to the existing set of [`DataWriterQos`] policies.
    #[tracing::instrument(skip(self))]
    pub fn get_qos(&self) -> DdsResult<DataWriterQos> {
        block_on(self.writer_async.get_qos())
    }

    /// This operation allows access to the [`StatusCondition`] associated with the Entity. The returned
    /// condition can then be added to a [`WaitSet`](crate::infrastructure::wait_set::WaitSet) so that the application can wait for specific status changes
    /// that affect the Entity.
    #[tracing::instrument(skip(self))]
    pub fn get_statuscondition(&self) -> StatusCondition {
        StatusCondition::new(self.writer_async.get_statuscondition())
    }

    /// This operation retrieves the list of communication statuses in the Entity that are 'triggered.' That is, the list of statuses whose
    /// value has changed since the last time the application read the status.
    /// When the entity is first created or if the entity is not enabled, all communication statuses are in the *untriggered* state so the
    /// list returned by the [`Self::get_status_changes`] operation will be empty.
    /// The list of statuses returned by the [`Self::get_status_changes`] operation refers to the status that are triggered on the Entity itself
    /// and does not include statuses that apply to contained entities.
    #[tracing::instrument(skip(self))]
    pub fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        block_on(self.writer_async.get_status_changes())
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
    /// Entities created from a factory that is disabled, are created disabled regardless of the setting of the ENTITY_FACTORY Qos
    /// policy.
    /// Calling enable on an Entity whose factory is not enabled will fail and return PRECONDITION_NOT_MET.
    /// If the `autoenable_created_entities` field of [`EntityFactoryQosPolicy`](crate::infrastructure::qos_policy::EntityFactoryQosPolicy) is set to [`true`], the [`Self::enable()`] operation on the factory will
    /// automatically enable all entities created from the factory.
    /// The Listeners associated with an entity are not called until the entity is enabled. Conditions associated with an entity that is not
    /// enabled are *inactive,* that is, the operation [`StatusCondition::get_trigger_value()`] will always return `false`.
    #[tracing::instrument(skip(self))]
    pub fn enable(&self) -> DdsResult<()> {
        block_on(self.writer_async.enable())
    }

    /// This operation returns the [`InstanceHandle`] that represents the Entity.
    #[tracing::instrument(skip(self))]
    pub fn get_instance_handle(&self) -> InstanceHandle {
        self.writer_async.get_instance_handle()
    }
}

impl<Foo> DataWriter<Foo> {
    /// This operation installs a Listener on the Entity. The listener will only be invoked on the changes of communication status
    /// indicated by the specified mask. It is permitted to use [`None`] as the value of the listener. The [`None`] listener behaves
    /// as a Listener whose operations perform no action.
    /// Only one listener can be attached to each Entity. If a listener was already set, the operation [`Self::set_listener()`] will replace it with the
    /// new one. Consequently if the value [`None`] is passed for the listener parameter to the [`Self::set_listener()`] operation, any existing listener
    /// will be removed.
    #[tracing::instrument(skip(self, a_listener))]
    pub fn set_listener(
        &self,
        a_listener: Option<impl DataWriterListener<Foo> + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        block_on(self.writer_async.set_listener(a_listener, mask))
    }
}
