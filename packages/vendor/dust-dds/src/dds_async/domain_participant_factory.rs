// Modified for Flow-Like: reply through each request's private response sender.
use alloc::{borrow::ToOwned, boxed::Box, sync::Arc};
use core::ops::DerefMut;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

use super::domain_participant::DomainParticipantAsync;
use crate::{
    dcps::{
        channels::rpc::{RpcClient, RpcMailbox},
        dcps_mail::{CreateParticipantMail, DcpsMail, ParticipantFactoryMail, WireMail},
        listeners::domain_participant_listener::DcpsDomainParticipantListener,
    },
    dds_async::{
        configuration::DustDdsConfiguration, domain_participant_listener::DomainParticipantListener,
    },
    infrastructure::{
        domain::DomainId,
        error::DdsResult,
        instance::InstanceHandle,
        qos::{DomainParticipantFactoryQos, DomainParticipantQos, QosKind},
        status::StatusKind,
    },
    runtime::{
        Clock, DdsRuntime, Either, Either3, Spawner, TaskHandle, Timer, select_future,
        select3_future,
    },
    transport::{
        interface::{TransportDataReceiver, TransportParticipantFactory},
        types::{ENTITYID_PARTICIPANT, Guid, GuidPrefix},
    },
};

const WIRE_CHANNEL_SIZE: usize = 256;

#[doc(hidden)]
pub type DcpsSender = RpcClient;

#[doc(hidden)]
pub type WireChannel = embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    WireMail,
    WIRE_CHANNEL_SIZE,
>;

#[doc(hidden)]
#[derive(Clone)]
pub struct WireSender {
    channel: Arc<WireChannel>,
}

impl WireSender {
    pub async fn send(&self, message: WireMail) {
        self.channel.send(message).await;
    }
}

#[doc(hidden)]
pub struct WireReceiver {
    channel: Arc<WireChannel>,
}

impl WireReceiver {
    pub async fn receive(&self) -> WireMail {
        self.channel.receive().await
    }
}

/// Async version of [`DomainParticipantFactory`](crate::domain::domain_participant_factory::DomainParticipantFactory).
/// Unlike the sync version, the [`DomainParticipantFactoryAsync`] is not a singleton and can be created by means of
/// a constructor by passing a DDS runtime. This allows the factory
/// to spin tasks on an existing runtime which can be shared with other things outside Dust DDS.
pub struct DomainParticipantFactoryAsync<T: TransportParticipantFactory> {
    dcps_sender: DcpsSender,
    wire_sender: WireSender,
    entity_counter: core::sync::atomic::AtomicU32,
    app_id: [u8; 4],
    host_id: [u8; 4],
    transport: embassy_sync::mutex::Mutex<CriticalSectionRawMutex, T>,
    configuration: embassy_sync::mutex::Mutex<CriticalSectionRawMutex, DustDdsConfiguration>,
    worker_task: alloc::boxed::Box<dyn TaskHandle>,
    run_loop: Arc<core::sync::atomic::AtomicBool>,
}

impl<T: TransportParticipantFactory> DomainParticipantFactoryAsync<T> {
    /// Async version of [`create_participant`](crate::domain::domain_participant_factory::DomainParticipantFactory::create_participant).
    pub async fn create_participant(
        &self,
        domain_id: DomainId,
        qos: QosKind<DomainParticipantQos>,
        a_listener: Option<impl DomainParticipantListener + Send + 'static>,
        mask: &[StatusKind],
    ) -> DdsResult<DomainParticipantAsync> {
        let configuration = self.configuration.lock().await;
        let guid_prefix = self.create_new_guid_prefix();
        let participant_handle = InstanceHandle::from(Guid::new(guid_prefix, ENTITYID_PARTICIPANT));
        let transport_participant = self.transport.lock().await.create_participant(
            domain_id,
            TransportDataReceiver::new(participant_handle, self.wire_sender.clone()),
        );

        let domain_tag = configuration.domain_tag().to_owned();
        let participant_announcement_interval = configuration.participant_announcement_interval();
        let enable_type_information = configuration.enable_type_information();
        let listener_mask = mask.iter().collect();
        let dcps_listener = a_listener.map(DcpsDomainParticipantListener::new);

        let reply = self
            .dcps_sender
            .call(DcpsMail::ParticipantFactory(
                ParticipantFactoryMail::CreateParticipant(Box::new(CreateParticipantMail {
                    guid_prefix,
                    domain_id,
                    qos,
                    dcps_listener,
                    listener_mask,
                    transport_participant,
                    domain_tag,
                    participant_announcement_interval,
                    enable_type_information,
                })),
            ))
            .await?;

        let participant_handle = reply.expect_instance_handle()?;

        let domain_participant =
            DomainParticipantAsync::new(self.dcps_sender.clone(), domain_id, participant_handle);

        Ok(domain_participant)
    }

    /// Async version of [`delete_participant`](crate::domain::domain_participant_factory::DomainParticipantFactory::delete_participant).
    pub async fn delete_participant(&self, participant: &DomainParticipantAsync) -> DdsResult<()> {
        let participant_handle = participant.get_instance_handle();

        self.dcps_sender
            .call(DcpsMail::ParticipantFactory(
                ParticipantFactoryMail::DeleteParticipant { participant_handle },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`lookup_participant`](crate::domain::domain_participant_factory::DomainParticipantFactory::lookup_participant).
    pub async fn lookup_participant(
        &self,
        domain_id: DomainId,
    ) -> DdsResult<Option<DomainParticipantAsync>> {
        let reply = self
            .dcps_sender
            .call(DcpsMail::ParticipantFactory(
                ParticipantFactoryMail::LookupParticipant { domain_id },
            ))
            .await?;
        Ok(reply
            .expect_option_instance_handle()?
            .map(|handle| DomainParticipantAsync::new(self.dcps_sender.clone(), domain_id, handle)))
    }

    /// Async version of [`set_default_participant_qos`](crate::domain::domain_participant_factory::DomainParticipantFactory::set_default_participant_qos).
    pub async fn set_default_participant_qos(
        &self,
        qos: QosKind<DomainParticipantQos>,
    ) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::ParticipantFactory(
                ParticipantFactoryMail::SetDefaultParticipantQos { qos: Box::new(qos) },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_default_participant_qos`](crate::domain::domain_participant_factory::DomainParticipantFactory::get_default_participant_qos).
    pub async fn get_default_participant_qos(&self) -> DdsResult<DomainParticipantQos> {
        self.dcps_sender
            .call(DcpsMail::ParticipantFactory(
                ParticipantFactoryMail::GetDefaultParticipantQos,
            ))
            .await?
            .expect_participant_qos()
    }

    /// Async version of [`set_qos`](crate::domain::domain_participant_factory::DomainParticipantFactory::set_qos).
    pub async fn set_qos(&self, qos: QosKind<DomainParticipantFactoryQos>) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::ParticipantFactory(
                ParticipantFactoryMail::SetQos { qos: Box::new(qos) },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_qos`](crate::domain::domain_participant_factory::DomainParticipantFactory::get_qos).
    pub async fn get_qos(&self) -> DdsResult<DomainParticipantFactoryQos> {
        self.dcps_sender
            .call(DcpsMail::ParticipantFactory(ParticipantFactoryMail::GetQos))
            .await?
            .expect_factory_qos()
    }

    /// Get a mutable reference to the transport object
    pub async fn get_mut_transport(&self) -> impl DerefMut<Target = T> + '_ {
        self.transport.lock().await
    }

    /// Get a mutable reference to the configuration object
    pub async fn get_mut_configuration(&self) -> impl DerefMut<Target = DustDdsConfiguration> + '_ {
        self.configuration.lock().await
    }
}

#[cfg(feature = "std")]
impl
    DomainParticipantFactoryAsync<
        crate::rtps_udp_transport::udp_transport::RtpsUdpTransportParticipantFactory,
    >
{
    /// This operation returns the [`DomainParticipantFactoryAsync`] singleton. The operation is idempotent, that is, it can be called multiple
    /// times without side-effects and it will return the same [`DomainParticipantFactoryAsync`] instance.
    #[tracing::instrument]
    pub fn get_instance() -> &'static Self {
        use core::net::IpAddr;
        use network_interface::{Addr, NetworkInterface, NetworkInterfaceConfig};
        use std::sync::OnceLock;
        use tracing::warn;

        static PARTICIPANT_FACTORY_ASYNC: OnceLock<
            DomainParticipantFactoryAsync<
                crate::rtps_udp_transport::udp_transport::RtpsUdpTransportParticipantFactory,
            >,
        > = OnceLock::new();
        PARTICIPANT_FACTORY_ASYNC.get_or_init(|| {

            let runtime = crate::std_runtime::StdRuntime::default();
            let interface_address = NetworkInterface::show()
                .expect("Could not scan interfaces")
                .into_iter()
                .flat_map(|i| {
                    i.addr
                        .into_iter()
                        .filter(|a| matches!(a, Addr::V4(v4) if !v4.ip.is_loopback()))
                })
                .next();
            let host_id = if let Some(interface) = interface_address {
                match interface.ip() {
                    IpAddr::V4(a) => a.octets(),
                    IpAddr::V6(_) => unimplemented!("IPv6 not yet implemented"),
                }
            } else {
                warn!("Failed to get Host ID from IP address, use 0 instead");
                [0; 4]
            };

            let app_id = std::process::id().to_ne_bytes();
            let transport = crate::rtps_udp_transport::udp_transport::RtpsUdpTransportParticipantFactory::default();
            let configuration = Default::default();
            Self::new(runtime, app_id, host_id, transport, configuration)
        })
    }
}

impl<T: TransportParticipantFactory> DomainParticipantFactoryAsync<T> {
    #[doc(hidden)]
    pub fn new<R: DdsRuntime>(
        runtime: R,
        app_id: [u8; 4],
        host_id: [u8; 4],
        transport: T,
        configuration: DustDdsConfiguration,
    ) -> Self {
        let rpc_mailbox = Arc::new(RpcMailbox::new());
        let dcps_sender = RpcClient::new(rpc_mailbox.clone());
        let wire_channel = Arc::new(WireChannel::new());
        let wire_sender = WireSender {
            channel: wire_channel.clone(),
        };
        let wire_receiver = WireReceiver {
            channel: wire_channel,
        };
        let spawner_handle = runtime.spawner();
        let mut timer_handle = runtime.timer();

        let mut domain_participant_factory =
            crate::dcps::dcps_participant_factory::DcpsParticipantFactory::new(
                runtime,
                dcps_sender.clone(),
            );
        let run_loop = Arc::new(core::sync::atomic::AtomicBool::new(true));
        let run_loop_clone = run_loop.clone();
        let worker_task = spawner_handle.spawn(async move {
            let span = tracing::trace_span!("dds_actor_loop");
            let _enter = span.enter();
            while run_loop_clone.load(core::sync::atomic::Ordering::Relaxed) {
                let now = domain_participant_factory.runtime.clock().now();
                let next_task_time = domain_participant_factory.time_until_next_event(now);

                if let Some(next_task_time) = next_task_time {
                    match select3_future(
                        rpc_mailbox.receive_request(),
                        timer_handle.delay(next_task_time.into()),
                        wire_receiver.receive(),
                    )
                    .await
                    {
                        Either3::A((user_mail, reply_sender)) => {
                            let now = domain_participant_factory.runtime.clock().now();
                            let reply = domain_participant_factory.handle(user_mail, now);
                            reply_sender.send(reply);
                        }
                        Either3::B(_) => {
                            let now = domain_participant_factory.runtime.clock().now();
                            for dp in &mut domain_participant_factory.domain_participant_list {
                                dp.remove_stale_participants(now);
                                dp.check_missed_reader_deadline(now);
                                dp.check_missed_writer_deadline(now);
                                dp.remove_stale_writer_samples(now);
                                dp.remove_stale_reader_samples(now);
                                dp.check_pending_writer_sample_timeout(now);
                                dp.process_pending_write_samples(now);
                                dp.announce_participant_if_needed(now);
                                dp.notify_find_topic_senders(now);
                                dp.poke(now);
                            }
                        }
                        Either3::C(wire_mail) => {
                            let now = domain_participant_factory.runtime.clock().now();
                            if let Some(dp) = domain_participant_factory
                                .domain_participant_list
                                .iter_mut()
                                .find(|x| x.get_instance_handle() == &wire_mail.participant_handle)
                            {
                                dp.handle_data(&wire_mail.data_message, now);
                                dp.process_builtin_cache_changes(now);
                                dp.process_user_defined_received_cache_changes(now);
                                dp.request_topic_type_representation(now);
                            }
                        }
                    };
                } else {
                    match select_future(rpc_mailbox.receive_request(), wire_receiver.receive())
                        .await
                    {
                        Either::A((user_mail, reply_sender)) => {
                            let now = domain_participant_factory.runtime.clock().now();
                            let reply = domain_participant_factory.handle(user_mail, now);
                            reply_sender.send(reply);
                        }
                        Either::B(wire_mail) => {
                            let now = domain_participant_factory.runtime.clock().now();
                            if let Some(dp) = domain_participant_factory
                                .domain_participant_list
                                .iter_mut()
                                .find(|x| x.get_instance_handle() == &wire_mail.participant_handle)
                            {
                                dp.handle_data(&wire_mail.data_message, now);
                                dp.process_builtin_cache_changes(now);
                                dp.process_user_defined_received_cache_changes(now);
                                dp.request_topic_type_representation(now);
                            }
                        }
                    };
                }
            }
        });
        Self {
            dcps_sender,
            wire_sender,
            app_id,
            host_id,
            entity_counter: core::sync::atomic::AtomicU32::new(0),
            transport: Mutex::new(transport),
            configuration: Mutex::new(configuration),
            worker_task: Box::new(worker_task),
            run_loop,
        }
    }

    fn create_new_guid_prefix(&self) -> GuidPrefix {
        let instance_id = self
            .entity_counter
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed)
            .to_ne_bytes();

        [
            self.host_id[0],
            self.host_id[1],
            self.host_id[2],
            self.host_id[3], // Host ID
            self.app_id[0],
            self.app_id[1],
            self.app_id[2],
            self.app_id[3], // App ID
            instance_id[0],
            instance_id[1],
            instance_id[2],
            instance_id[3], // Instance ID
        ]
    }

    #[doc(hidden)]
    pub fn shutdown(&self) {
        self.run_loop
            .store(false, core::sync::atomic::Ordering::Relaxed);
        self.worker_task.join();
    }
}
