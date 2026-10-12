//! DDS topics with runtime final-struct schemas and RTPS discovery on local networks.

use crate::{Result, require};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DdsPrimitive {
    Boolean,
    #[serde(rename = "uint8")]
    UInt8,
    Int8,
    #[serde(rename = "uint16")]
    UInt16,
    Int16,
    #[serde(rename = "uint32")]
    UInt32,
    Int32,
    #[serde(rename = "uint64")]
    UInt64,
    Int64,
    Float32,
    Float64,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DdsField {
    pub name: String,
    pub data_type: DdsPrimitive,
    /// Encode an unbounded IDL sequence of this primitive.
    #[serde(default)]
    pub sequence: bool,
    #[serde(default)]
    pub key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DdsType {
    /// Exact DDS type name, including any IDL module prefixes separated by ::.
    pub name: String,
    /// Members in IDL declaration order. Only final structs with sequential member IDs are supported.
    pub fields: Vec<DdsField>,
}

impl DdsType {
    pub fn validate(&self) -> Result<()> {
        fn identifier(value: &str) -> bool {
            let mut bytes = value.bytes();
            bytes
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        }
        require(
            self.name.len() <= 256 && self.name.split("::").all(identifier),
            "DDS type name must contain valid IDL identifiers separated by :: (at most 256 bytes)",
        )?;
        require(
            !self.fields.is_empty() && self.fields.len() <= 64,
            "DDS types require between 1 and 64 fields",
        )?;
        let mut names = std::collections::HashSet::new();
        for field in &self.fields {
            require(
                field.name.len() <= 128 && identifier(&field.name),
                "DDS field names must be IDL identifiers of at most 128 bytes",
            )?;
            require(names.insert(&field.name), "DDS field names must be unique")?;
            require(
                !field.key
                    || (!field.sequence
                        && !matches!(
                            field.data_type,
                            DdsPrimitive::Float32 | DdsPrimitive::Float64
                        )),
                "DDS keys must be scalar integer, boolean, or string fields",
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DdsRole {
    #[default]
    Publisher,
    Subscriber,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DdsReliability {
    BestEffort,
    #[default]
    Reliable,
}

/// Inline DDS Security credentials and signed access-control documents.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DdsSecurity {
    pub identity_ca_pem: String,
    pub identity_certificate_pem: String,
    pub private_key_pem: String,
    #[serde(default)]
    pub private_key_password: String,
    pub permissions_ca_pem: String,
    /// S/MIME signed governance. Every rule must require authentication and encryption.
    pub governance_signed: String,
    /// S/MIME signed permissions whose subject matches the identity certificate.
    pub permissions_signed: String,
}

impl std::fmt::Debug for DdsSecurity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DdsSecurity { credentials: [REDACTED] }")
    }
}

impl DdsSecurity {
    fn validate(&self) -> Result<()> {
        for value in [
            &self.identity_ca_pem,
            &self.identity_certificate_pem,
            &self.private_key_pem,
            &self.permissions_ca_pem,
            &self.governance_signed,
            &self.permissions_signed,
        ] {
            require(
                !value.is_empty() && value.len() <= 1024 * 1024 && !value.contains('\0'),
                "DDS security documents must contain 1 to 1048576 bytes without NUL characters",
            )?;
        }
        require(
            self.private_key_password.len() <= 4096 && !self.private_key_password.contains('\0'),
            "DDS private-key password must contain at most 4096 bytes without NUL characters",
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DdsConfig {
    pub domain_id: i32,
    pub topic: String,
    pub data_type: DdsType,
    pub role: DdsRole,
    pub reliability: DdsReliability,
    /// KeepLast depth per instance. The total history has the same sample limit.
    pub history_depth: u32,
    pub timeout_ms: u64,
    /// Requires Cyclone DDS 11 and OpenSSL 3 security runtime libraries. No plaintext fallback.
    pub security: Option<DdsSecurity>,
}

impl Default for DdsConfig {
    fn default() -> Self {
        Self {
            domain_id: 0,
            topic: "Telemetry".into(),
            data_type: DdsType {
                name: "Telemetry".into(),
                fields: vec![DdsField {
                    name: "value".into(),
                    data_type: DdsPrimitive::Float64,
                    sequence: false,
                    key: false,
                }],
            },
            role: DdsRole::Publisher,
            reliability: DdsReliability::Reliable,
            history_depth: 32,
            timeout_ms: 10_000,
            security: None,
        }
    }
}

impl DdsConfig {
    pub fn validate(&self) -> Result<()> {
        require(
            (0..=232).contains(&self.domain_id),
            "DDS domain ID must be between 0 and 232",
        )?;
        require(
            !self.topic.is_empty() && self.topic.len() <= 256 && !self.topic.contains('\0'),
            "DDS topic must contain between 1 and 256 bytes without NUL characters",
        )?;
        require(
            (1..=256).contains(&self.history_depth),
            "DDS history depth must be between 1 and 256",
        )?;
        require(
            (1..=300_000).contains(&self.timeout_ms),
            "DDS timeout must be between 1 and 300000 ms",
        )?;
        if let Some(security) = &self.security {
            require(
                self.domain_id <= 230,
                "DDS Security domain ID must be between 0 and 230",
            )?;
            security.validate()?;
        }
        self.data_type.validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DdsSample {
    pub topic: String,
    pub type_name: String,
    /// None for an instance disposal or a notification that its writers disappeared.
    pub data: Option<serde_json::Value>,
    pub valid_data: bool,
    pub instance_state: String,
    pub source_timestamp_ns: Option<i64>,
}

#[cfg(any(feature = "execute", feature = "dds"))]
mod runtime {
    use super::*;
    use crate::Error;
    use dust_dds::{
        dds_async::{
            data_reader::DataReaderAsync, data_writer::DataWriterAsync,
            domain_participant::DomainParticipantAsync,
            domain_participant_factory::DomainParticipantFactoryAsync,
        },
        infrastructure::{
            error::DdsError,
            listener::NO_LISTENER,
            qos::{DataReaderQos, DataWriterQos, QosKind},
            qos_policy::{
                DataRepresentationQosPolicy, HistoryQosPolicy, HistoryQosPolicyKind, Length,
                ReliabilityQosPolicy, ReliabilityQosPolicyKind, ResourceLimitsQosPolicy,
                TypeConsistencyEnforcementQosPolicy, TypeConsistencyKind, XCDR_DATA_REPRESENTATION,
            },
            sample_info::InstanceStateKind,
            status::NO_STATUS,
            time::{Duration as DdsDuration, DurationKind},
        },
        xtypes::{
            data_storage::DataStorage,
            dynamic_type::{
                DynamicData, DynamicDataFactory, DynamicType, DynamicTypeBuilderFactory,
                ExtensibilityKind, MemberDescriptor, TryConstructKind, TypeDescriptor, TypeKind,
            },
            type_support::Type,
        },
    };
    use serde_json::Value;
    use std::{
        future::Future,
        sync::{
            Mutex, OnceLock,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    use tokio_util::sync::CancellationToken;

    const MAX_SAMPLE_BYTES: usize = 1024 * 1024;
    const MAX_SEQUENCE: usize = 65_536;
    const MAX_SCHEMAS: usize = 128;

    fn failure(error: impl std::fmt::Debug) -> Error {
        Error::Other(anyhow::anyhow!("DDS: {error:?}"))
    }

    pub(super) fn sdk_runtime() -> Result<&'static tokio::runtime::Handle> {
        static RUNTIME: OnceLock<std::result::Result<tokio::runtime::Handle, String>> =
            OnceLock::new();
        RUNTIME
            .get_or_init(|| {
                let (sender, receiver) = std::sync::mpsc::sync_channel(1);
                std::thread::Builder::new()
                    .name("flow-like-dds".into())
                    .spawn(move || {
                        let runtime = match tokio::runtime::Builder::new_current_thread()
                            .enable_time()
                            .build()
                        {
                            Ok(runtime) => runtime,
                            Err(error) => {
                                let _ = sender.send(Err(error.to_string()));
                                return;
                            }
                        };
                        let _ = sender.send(Ok(runtime.handle().clone()));
                        runtime.block_on(std::future::pending::<()>());
                    })
                    .map_err(|error| error.to_string())?;
                receiver.recv().map_err(|error| error.to_string())?
            })
            .as_ref()
            .map_err(failure)
    }

    fn primitive_type(kind: DdsPrimitive) -> DynamicType<'static> {
        match kind {
            DdsPrimitive::Boolean => bool::TYPE,
            DdsPrimitive::UInt8 => u8::TYPE,
            DdsPrimitive::Int8 => i8::TYPE,
            DdsPrimitive::UInt16 => u16::TYPE,
            DdsPrimitive::Int16 => i16::TYPE,
            DdsPrimitive::UInt32 => u32::TYPE,
            DdsPrimitive::Int32 => i32::TYPE,
            DdsPrimitive::UInt64 => u64::TYPE,
            DdsPrimitive::Int64 => i64::TYPE,
            DdsPrimitive::Float32 => f32::TYPE,
            DdsPrimitive::Float64 => f64::TYPE,
            // XTypes uses zero for an unbounded IDL string. Rust String::TYPE uses
            // u32::MAX, which produces a different type identifier in other DDS vendors.
            DdsPrimitive::String => DynamicTypeBuilderFactory::create_string_type(0).build(),
        }
    }

    pub(super) fn dynamic_type(schema: &DdsType) -> Result<DynamicType<'static>> {
        schema.validate()?;
        static TYPES: OnceLock<Mutex<Vec<(DdsType, DynamicType<'static>)>>> = OnceLock::new();
        let mut types = TYPES.get_or_init(Mutex::default).lock().map_err(failure)?;
        if let Some((_, value)) = types.iter().find(|(key, _)| key == schema) {
            return Ok(*value);
        }
        // Dust DDS retains dynamic type descriptors for the process lifetime. Reuse them
        // and cap distinct schemas so repeated workflow runs cannot grow this memory forever.
        require(
            types.len() < MAX_SCHEMAS,
            "DDS process limit of 128 distinct type schemas reached",
        )?;
        let mut builder = DynamicTypeBuilderFactory::create_type(TypeDescriptor {
            kind: TypeKind::STRUCTURE,
            name: Box::leak(schema.name.clone().into_boxed_str()),
            base_type: None,
            discriminator_type: None,
            bound: &[],
            element_type: None,
            key_element_type: None,
            extensibility_kind: ExtensibilityKind::Final,
            is_nested: false,
            is_autoid_hash: false,
        });
        for (id, field) in schema.fields.iter().enumerate() {
            let scalar = primitive_type(field.data_type);
            let field_type = if field.sequence {
                DynamicTypeBuilderFactory::create_sequence_type(scalar, 0).build()
            } else {
                scalar
            };
            builder
                .add_member(MemberDescriptor {
                    name: Box::leak(field.name.clone().into_boxed_str()),
                    id: id as u32,
                    r#type: field_type,
                    default_value: None,
                    index: id as u32,
                    label: &[],
                    try_construct_kind: TryConstructKind::Discard,
                    is_key: field.key,
                    is_optional: false,
                    is_must_understand: false,
                    is_shared: false,
                    is_default_label: false,
                    is_external: false,
                })
                .map_err(failure)?;
        }
        let value = builder.build();
        types.push((schema.clone(), value));
        Ok(value)
    }

    fn storage(field: &DdsField, value: &Value) -> Result<DataStorage> {
        if field.sequence {
            let array = value
                .as_array()
                .ok_or_else(|| failure(format!("field {} requires an array", field.name)))?;
            require(
                array.len() <= MAX_SEQUENCE,
                "DDS sequence exceeds 65536 elements",
            )?;
        }
        macro_rules! convert {
            ($ty:ty, $scalar:ident, $sequence:ident) => {{
                if field.sequence {
                    DataStorage::$sequence(
                        serde_json::from_value::<Vec<$ty>>(value.clone()).map_err(failure)?,
                    )
                } else {
                    DataStorage::$scalar(
                        serde_json::from_value::<$ty>(value.clone()).map_err(failure)?,
                    )
                }
            }};
        }
        let result = match field.data_type {
            DdsPrimitive::Boolean => convert!(bool, Boolean, SequenceBoolean),
            DdsPrimitive::UInt8 => convert!(u8, UInt8, SequenceUInt8),
            DdsPrimitive::Int8 => convert!(i8, Int8, SequenceInt8),
            DdsPrimitive::UInt16 => convert!(u16, UInt16, SequenceUInt16),
            DdsPrimitive::Int16 => convert!(i16, Int16, SequenceInt16),
            DdsPrimitive::UInt32 => convert!(u32, UInt32, SequenceUInt32),
            DdsPrimitive::Int32 => convert!(i32, Int32, SequenceInt32),
            DdsPrimitive::UInt64 => convert!(u64, UInt64, SequenceUInt64),
            DdsPrimitive::Int64 => convert!(i64, Int64, SequenceInt64),
            DdsPrimitive::Float32 => convert!(f32, Float32, SequenceFloat32),
            DdsPrimitive::Float64 => convert!(f64, Float64, SequenceFloat64),
            DdsPrimitive::String => convert!(String, String, SequenceString),
        };
        match &result {
            DataStorage::Float32(v) => {
                require(v.is_finite(), "DDS float32 is outside its finite range")?
            }
            DataStorage::SequenceFloat32(v) => require(
                v.iter().all(|n| n.is_finite()),
                "DDS sequence has a non-finite float32",
            )?,
            DataStorage::String(v) => require(
                !v.contains('\0'),
                "DDS strings cannot contain NUL characters",
            )?,
            DataStorage::SequenceString(v) => require(
                v.iter().all(|s| !s.contains('\0')),
                "DDS strings cannot contain NUL characters",
            )?,
            _ => {}
        }
        Ok(result)
    }

    pub(super) fn sample_to_dynamic(
        schema: &DdsType,
        kind: DynamicType<'static>,
        value: Value,
    ) -> Result<DynamicData<'static>> {
        require(
            serde_json::to_vec(&value).map_err(failure)?.len() <= MAX_SAMPLE_BYTES,
            "DDS JSON sample exceeds 1 MiB",
        )?;
        let object = value
            .as_object()
            .ok_or_else(|| failure("sample must be a JSON object"))?;
        require(
            object.len() == schema.fields.len(),
            "DDS sample must contain exactly the configured fields",
        )?;
        let mut data = DynamicDataFactory::create_data(kind);
        for (id, field) in schema.fields.iter().enumerate() {
            let value = object
                .get(&field.name)
                .ok_or_else(|| failure(format!("missing field {}", field.name)))?;
            data.set_value(id as u32, storage(field, value)?);
        }
        Ok(data)
    }

    fn storage_to_json(value: &DataStorage) -> Result<Value> {
        macro_rules! convert {
            ($($variant:ident),+ $(,)?) => {
                match value {
                    $(DataStorage::$variant(v) => serde_json::to_value(v).map_err(failure),)+
                    _ => Err(failure("unsupported received field type")),
                }
            };
        }
        convert!(
            UInt8,
            Int8,
            UInt16,
            Int16,
            UInt32,
            Int32,
            UInt64,
            Int64,
            Float32,
            Float64,
            Boolean,
            String,
            SequenceUInt8,
            SequenceInt8,
            SequenceUInt16,
            SequenceInt16,
            SequenceUInt32,
            SequenceInt32,
            SequenceUInt64,
            SequenceInt64,
            SequenceFloat32,
            SequenceFloat64,
            SequenceBoolean,
            SequenceString
        )
    }

    fn sample_to_json(schema: &DdsType, data: &DynamicData<'static>) -> Result<Value> {
        let mut object = serde_json::Map::new();
        for (id, field) in schema.fields.iter().enumerate() {
            object.insert(
                field.name.clone(),
                storage_to_json(data.get_value(id as u32).map_err(failure)?)?,
            );
        }
        let value = Value::Object(object);
        // Validate incoming bounds and numeric values using the same contract as publication.
        sample_to_dynamic(schema, data.r#type(), value.clone())?;
        Ok(value)
    }

    pub struct DdsClient {
        config: DdsConfig,
        participant: DomainParticipantAsync,
        writer: Option<DataWriterAsync<DynamicData<'static>>>,
        reader: Option<DataReaderAsync<DynamicData<'static>>>,
        data_type: DynamicType<'static>,
        stop: CancellationToken,
        close_started: AtomicBool,
        cleanup: tokio::sync::watch::Sender<Option<std::result::Result<(), String>>>,
        runtime: tokio::runtime::Handle,
    }

    async fn delete_participant(participant: &DomainParticipantAsync) -> Result<()> {
        let contained = participant
            .delete_contained_entities()
            .await
            .map_err(failure);
        let deleted = DomainParticipantFactoryAsync::get_instance()
            .delete_participant(participant)
            .await
            .map_err(failure);
        contained.and(deleted)
    }

    impl Drop for DdsClient {
        fn drop(&mut self) {
            self.begin_close();
            if self
                .cleanup
                .borrow()
                .as_ref()
                .is_some_and(|result| result.is_err())
            {
                let participant = self.participant.clone();
                self.runtime.spawn(async move {
                    let _ = delete_participant(&participant).await;
                });
            }
        }
    }

    impl DdsClient {
        pub async fn connect(config: DdsConfig) -> Result<Self> {
            config.validate()?;
            // A dropped caller must still let entity creation finish and drop its owner.
            let timeout = Duration::from_millis(config.timeout_ms);
            // Dust DDS reader creation borrows a non-Sync topic trait object across await.
            // Poll that setup future on a blocking worker while its IO runs on Dust's runtime.
            let runtime = sdk_runtime()?.clone();
            let task =
                sdk_runtime()?.spawn_blocking(move || runtime.block_on(Self::create(config)));
            tokio::time::timeout(timeout, task)
                .await
                .map_err(|_| Error::Timeout)?
                .map_err(failure)?
        }

        async fn create(config: DdsConfig) -> Result<Self> {
            let data_type = dynamic_type(&config.data_type)?;
            let participant = DomainParticipantFactoryAsync::get_instance()
                .create_participant(config.domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
                .await
                .map_err(failure)?;
            let mut client = Self {
                config,
                participant,
                writer: None,
                reader: None,
                data_type,
                stop: CancellationToken::new(),
                close_started: AtomicBool::new(false),
                cleanup: tokio::sync::watch::channel(None).0,
                runtime: sdk_runtime()?.clone(),
            };
            let topic = client
                .participant
                .create_dynamic_topic(
                    &client.config.topic,
                    &client.config.data_type.name,
                    QosKind::Default,
                    NO_LISTENER,
                    NO_STATUS,
                    data_type,
                )
                .await
                .map_err(failure)?;
            let reliability = ReliabilityQosPolicy {
                kind: match client.config.reliability {
                    DdsReliability::BestEffort => ReliabilityQosPolicyKind::BestEffort,
                    DdsReliability::Reliable => ReliabilityQosPolicyKind::Reliable,
                },
                max_blocking_time: DurationKind::Finite(DdsDuration::new(
                    (client.config.timeout_ms / 1000) as i32,
                    ((client.config.timeout_ms % 1000) * 1_000_000) as u32,
                )),
            };
            let depth = client.config.history_depth as i32;
            let history = HistoryQosPolicy {
                kind: HistoryQosPolicyKind::KeepLast(client.config.history_depth),
            };
            let resource_limits = ResourceLimitsQosPolicy {
                max_samples: Length::Limited(depth),
                max_instances: Length::Limited(depth),
                max_samples_per_instance: Length::Limited(depth),
            };
            let representation = DataRepresentationQosPolicy {
                value: vec![XCDR_DATA_REPRESENTATION],
            };
            match client.config.role {
                DdsRole::Publisher => {
                    let publisher = client
                        .participant
                        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
                        .await
                        .map_err(failure)?;
                    client.writer = Some(
                        publisher
                            .create_datawriter(
                                &topic,
                                QosKind::Specific(DataWriterQos {
                                    reliability,
                                    history,
                                    resource_limits,
                                    representation,
                                    ..Default::default()
                                }),
                                NO_LISTENER,
                                NO_STATUS,
                            )
                            .await
                            .map_err(failure)?,
                    );
                }
                DdsRole::Subscriber => {
                    let subscriber = client
                        .participant
                        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
                        .await
                        .map_err(failure)?;
                    client.reader = Some(
                        subscriber
                            .create_datareader(
                                &topic,
                                QosKind::Specific(DataReaderQos {
                                    type_consistency: TypeConsistencyEnforcementQosPolicy {
                                        kind: TypeConsistencyKind::DisallowTypeCoercion,
                                        force_type_validation: true,
                                        ignore_sequence_bounds: false,
                                        ignore_string_bounds: false,
                                        ignore_member_names: false,
                                        prevent_type_widening: true,
                                    },
                                    reliability,
                                    history,
                                    resource_limits,
                                    representation,
                                    ..Default::default()
                                }),
                                NO_LISTENER,
                                NO_STATUS,
                            )
                            .await
                            .map_err(failure)?,
                    );
                }
            }
            Ok(client)
        }

        async fn bounded<T>(&self, future: impl Future<Output = Result<T>>) -> Result<T> {
            tokio::select! {
                biased;
                _ = self.stop.cancelled() => Err(failure("session closed")),
                result = tokio::time::timeout(Duration::from_millis(self.config.timeout_ms), future) => result.map_err(|_| Error::Timeout)?,
            }
        }

        /// Wait until discovery finds at least one compatible remote endpoint.
        pub async fn wait_for_match(&self) -> Result<u32> {
            self.bounded(async {
                loop {
                    let count = match (&self.writer, &self.reader) {
                        (Some(writer), _) => writer
                            .get_matched_subscriptions()
                            .await
                            .map_err(failure)?
                            .len(),
                        (_, Some(reader)) => reader
                            .get_matched_publications()
                            .await
                            .map_err(failure)?
                            .len(),
                        _ => return Err(failure("endpoint is not initialized")),
                    };
                    if count > 0 {
                        return Ok(count as u32);
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
        }

        /// Require a discovered reader before writing. Reliable publication waits for acknowledgments.
        pub async fn publish(&self, value: Value) -> Result<()> {
            let writer = self
                .writer
                .as_ref()
                .ok_or_else(|| failure("publish requires a publisher session"))?;
            let data = sample_to_dynamic(&self.config.data_type, self.data_type, value)?;
            self.bounded(async {
                self.wait_for_match().await?;
                writer.write(data, None).await.map_err(failure)?;
                if self.config.reliability == DdsReliability::Reliable {
                    writer.wait_for_acknowledgments().await.map_err(failure)?;
                }
                Ok(())
            })
            .await
        }

        pub async fn receive(&self) -> Result<DdsSample> {
            let reader = self
                .reader
                .as_ref()
                .ok_or_else(|| failure("receive requires a subscriber session"))?;
            self.bounded(async {
                loop {
                    match reader.take_next_sample().await {
                        Ok(sample) => {
                            let data = sample
                                .data
                                .as_ref()
                                .map(|data| sample_to_json(&self.config.data_type, data))
                                .transpose()?;
                            let info = sample.sample_info;
                            return Ok(DdsSample {
                                topic: self.config.topic.clone(),
                                type_name: self.config.data_type.name.clone(),
                                data,
                                valid_data: info.valid_data,
                                instance_state: match info.instance_state {
                                    InstanceStateKind::Alive => "alive",
                                    InstanceStateKind::NotAliveDisposed => "disposed",
                                    InstanceStateKind::NotAliveNoWriters => "no_writers",
                                }
                                .into(),
                                source_timestamp_ns: info.source_timestamp.map(|t| {
                                    i64::from(t.sec()) * 1_000_000_000 + i64::from(t.nanosec())
                                }),
                            });
                        }
                        Err(DdsError::NoData) => {
                            tokio::time::sleep(Duration::from_millis(10)).await
                        }
                        Err(error) => return Err(failure(error)),
                    }
                }
            })
            .await
        }

        fn begin_close(&self) {
            self.stop.cancel();
            if self.close_started.swap(true, Ordering::AcqRel) {
                return;
            }
            let participant = self.participant.clone();
            let completion = self.cleanup.clone();
            // Every waiter observes the same result. Dropping a waiter cannot stop deletion.
            self.runtime.spawn(async move {
                let result = delete_participant(&participant)
                    .await
                    .map_err(|error| error.to_string());
                completion.send_replace(Some(result));
            });
        }

        pub async fn close(&self) -> Result<()> {
            self.begin_close();
            let mut result = self.cleanup.subscribe();
            tokio::time::timeout(Duration::from_millis(self.config.timeout_ms), async {
                loop {
                    if let Some(value) = result.borrow().clone() {
                        return value.map_err(failure);
                    }
                    result.changed().await.map_err(failure)?;
                }
            })
            .await
            .map_err(|_| Error::Timeout)?
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;

        fn schema() -> DdsType {
            DdsType {
                name: "Tests::Measurement".into(),
                fields: vec![
                    DdsField {
                        name: "id".into(),
                        data_type: DdsPrimitive::UInt32,
                        sequence: false,
                        key: true,
                    },
                    DdsField {
                        name: "values".into(),
                        data_type: DdsPrimitive::Int16,
                        sequence: true,
                        key: false,
                    },
                    DdsField {
                        name: "label".into(),
                        data_type: DdsPrimitive::String,
                        sequence: false,
                        key: false,
                    },
                ],
            }
        }

        #[test]
        fn dynamic_struct_preserves_keys_sequences_and_strings() {
            let schema = schema();
            let kind = dynamic_type(&schema).unwrap();
            assert!(kind.get_member(0).unwrap().descriptor.is_key);
            let value = json!({"id": 42, "values": [-32768, 32767], "label": "temperature °C"});
            let data = sample_to_dynamic(&schema, kind, value.clone()).unwrap();
            assert_eq!(sample_to_json(&schema, &data).unwrap(), value);
            assert_eq!(kind, dynamic_type(&schema).unwrap());
        }

        #[test]
        fn publication_rejects_wrong_fields_ranges_and_unbounded_samples() {
            let schema = schema();
            let kind = dynamic_type(&schema).unwrap();
            for value in [
                json!({"id": -1, "values": [], "label": "x"}),
                json!({"id": 1, "values": [32768], "label": "x"}),
                json!({"id": 1, "values": [], "extra": "x"}),
                json!({"id": 1, "values": [], "label": "x", "extra": true}),
                json!({"id": 1, "values": vec![0; MAX_SEQUENCE + 1], "label": "x"}),
                json!({"id": 1, "values": [], "label": "x".repeat(MAX_SAMPLE_BYTES)}),
            ] {
                assert!(sample_to_dynamic(&schema, kind, value).is_err());
            }
        }
    }
}

#[cfg(any(feature = "execute", feature = "dds"))]
#[path = "dds/secure.rs"]
mod secure;

#[cfg(any(feature = "execute", feature = "dds"))]
pub enum DdsClient {
    Plain(runtime::DdsClient),
    Secure(secure::SecureDdsClient),
}

#[cfg(any(feature = "execute", feature = "dds"))]
impl DdsClient {
    pub async fn connect(config: DdsConfig) -> Result<Self> {
        config.validate()?;
        if config.security.is_some() {
            secure::SecureDdsClient::connect(config)
                .await
                .map(Self::Secure)
        } else {
            runtime::DdsClient::connect(config).await.map(Self::Plain)
        }
    }
    pub async fn wait_for_match(&self) -> Result<u32> {
        match self {
            Self::Plain(c) => c.wait_for_match().await,
            Self::Secure(c) => c.wait_for_match().await,
        }
    }
    pub async fn publish(&self, value: serde_json::Value) -> Result<()> {
        match self {
            Self::Plain(c) => c.publish(value).await,
            Self::Secure(c) => c.publish(value).await,
        }
    }
    pub async fn receive(&self) -> Result<DdsSample> {
        match self {
            Self::Plain(c) => c.receive().await,
            Self::Secure(c) => c.receive().await,
        }
    }
    pub async fn close(&self) -> Result<()> {
        match self {
            Self::Plain(c) => c.close().await,
            Self::Secure(c) => c.close().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_security_is_redacted_and_old_configuration_remains_plain() {
        let secret = DdsSecurity {
            identity_ca_pem: "secret-ca".into(),
            identity_certificate_pem: "secret-cert".into(),
            private_key_pem: "secret-key".into(),
            private_key_password: "secret-password".into(),
            permissions_ca_pem: "secret-permissions-ca".into(),
            governance_signed: "secret-governance".into(),
            permissions_signed: "secret-permissions".into(),
        };
        let config = DdsConfig {
            security: Some(secret),
            ..Default::default()
        };
        assert!(!format!("{config:?}").contains("secret-"));
        assert!(
            serde_json::from_value::<DdsConfig>(serde_json::json!({}))
                .unwrap()
                .security
                .is_none()
        );
        assert!(serde_json::to_value(config).unwrap()["security"]["private_key_pem"].is_string());
    }

    #[test]
    fn unsigned_type_names_match_idl_names() {
        for (value, name) in [
            (DdsPrimitive::UInt8, "uint8"),
            (DdsPrimitive::UInt16, "uint16"),
            (DdsPrimitive::UInt32, "uint32"),
            (DdsPrimitive::UInt64, "uint64"),
        ] {
            assert_eq!(serde_json::to_value(value).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<DdsPrimitive>(serde_json::json!(name)).unwrap(),
                value
            );
        }
    }

    #[test]
    fn configuration_rejects_invalid_domains_history_and_schema() {
        let mut config = DdsConfig::default();
        assert!(config.validate().is_ok());
        config.domain_id = 233;
        assert!(config.validate().is_err());
        config.domain_id = 0;
        config.history_depth = 0;
        assert!(config.validate().is_err());
        config.history_depth = 32;
        config
            .data_type
            .fields
            .push(config.data_type.fields[0].clone());
        assert!(config.validate().is_err());
        config.data_type.fields.pop();
        config.data_type.fields[0].key = true;
        assert!(config.validate().is_err());
        config.data_type.fields[0].key = false;
        config.data_type.name = "Bad\0Name".into();
        assert!(config.validate().is_err());
    }
}
