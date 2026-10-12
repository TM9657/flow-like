use tracing::info;

use crate::{
    dds_async::{data_reader::DataReaderAsync, data_writer::DataWriterAsync, topic::TopicAsync},
    infrastructure::status::{
        InconsistentTopicStatus, LivelinessChangedStatus, LivelinessLostStatus,
        OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus, PublicationMatchedStatus,
        RequestedDeadlineMissedStatus, RequestedIncompatibleQosStatus, SampleLostStatus,
        SampleRejectedStatus, SubscriptionMatchedStatus,
    },
};
use core::future::Future;

/// The purpose of the DomainParticipantListener is to be the listener of last resort that is notified of all status changes not
/// captured by more specific listeners attached to the DomainEntity objects. When a relevant status change occurs, the DCPS
/// Service will first attempt to notify the listener attached to the concerned DomainEntity if one is installed. Otherwise, the
/// DCPS Service will notify the Listener attached to the DomainParticipant.
pub trait DomainParticipantListener {
    /// Method that is called when any inconsistent topic is discovered in the domain participant.
    fn on_inconsistent_topic(
        &mut self,
        _the_topic: TopicAsync,
        status: InconsistentTopicStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_inconsistent_topic");
        core::future::ready(())
    }

    /// Method that is called when any writer in the domain participant reports a liveliness lost status.
    fn on_liveliness_lost(
        &mut self,
        _the_writer: DataWriterAsync<()>,
        status: LivelinessLostStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_liveliness_lost");
        core::future::ready(())
    }

    /// Method that is called when any data writer in the domain participant reports a deadline missed status.
    fn on_offered_deadline_missed(
        &mut self,
        _the_writer: DataWriterAsync<()>,
        status: OfferedDeadlineMissedStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_offered_deadline_missed");
        core::future::ready(())
    }

    /// Method that is called when any data writer in the domain participant reports an offered incompatible QoS status.
    fn on_offered_incompatible_qos(
        &mut self,
        _the_writer: DataWriterAsync<()>,
        status: OfferedIncompatibleQosStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_offered_incompatible_qos");
        core::future::ready(())
    }

    /// Method that is called when any data reader in the domain participant reports a sample lost status.
    fn on_sample_lost(
        &mut self,
        _the_reader: DataReaderAsync<()>,
        status: SampleLostStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_sample_lost");
        core::future::ready(())
    }

    /// Method that is called when any data reader in the domain participant reports a data available status.
    fn on_data_available(
        &mut self,
        _the_reader: DataReaderAsync<()>,
    ) -> impl Future<Output = ()> + Send {
        info!("on_data_available");
        core::future::ready(())
    }

    /// Method that is called when any data reader in the domain participant reports a sample rejected status.
    fn on_sample_rejected(
        &mut self,
        _the_reader: DataReaderAsync<()>,
        status: SampleRejectedStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_sample_rejected");
        core::future::ready(())
    }

    /// Method that is called when any data reader in the domain participant reports a liveliness changed status.
    fn on_liveliness_changed(
        &mut self,
        _the_reader: DataReaderAsync<()>,
        status: LivelinessChangedStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_liveliness_changed");
        core::future::ready(())
    }

    /// Method that is called when any data reader in the domain participant reports a requested deadline missed status.
    fn on_requested_deadline_missed(
        &mut self,
        _the_reader: DataReaderAsync<()>,
        status: RequestedDeadlineMissedStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_requested_deadline_missed");
        core::future::ready(())
    }

    /// Method that is called when any data reader in the domain participant reports a requested incompatible QoS status.
    fn on_requested_incompatible_qos(
        &mut self,
        _the_reader: DataReaderAsync<()>,
        status: RequestedIncompatibleQosStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_requested_incompatible_qos");
        core::future::ready(())
    }

    /// Method that is called when any data writer in the domain participant reports a publication matched status.
    fn on_publication_matched(
        &mut self,
        _the_writer: DataWriterAsync<()>,
        status: PublicationMatchedStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_publication_matched");
        core::future::ready(())
    }

    /// Method that is called when any data reader in the domain participant reports a subscription matched status.
    fn on_subscription_matched(
        &mut self,
        _the_reader: DataReaderAsync<()>,
        status: SubscriptionMatchedStatus,
    ) -> impl Future<Output = ()> + Send {
        info!(?status, "on_subscription_matched");
        core::future::ready(())
    }
}
