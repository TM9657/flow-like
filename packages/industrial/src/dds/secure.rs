use super::*;
use crate::Error;
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    ptr,
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
#[path = "ffi.rs"]
mod ffi;
#[path = "governance.rs"]
mod governance;
use ffi::*;

fn cstring(value: &str) -> Result<CString> {
    CString::new(value).map_err(|_| error("value contains NUL"))
}
fn aligned(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}
fn scalar_size(kind: DdsPrimitive) -> usize {
    match kind {
        DdsPrimitive::Boolean | DdsPrimitive::Int8 | DdsPrimitive::UInt8 => 1,
        DdsPrimitive::Int16 | DdsPrimitive::UInt16 => 2,
        DdsPrimitive::Int32 | DdsPrimitive::UInt32 | DdsPrimitive::Float32 => 4,
        _ => 8,
    }
}
fn primitive(kind: DdsPrimitive) -> i32 {
    match kind {
        DdsPrimitive::Boolean => 1,
        DdsPrimitive::UInt8 => 13,
        DdsPrimitive::Int8 => 12,
        DdsPrimitive::Int16 => 3,
        DdsPrimitive::Int32 => 4,
        DdsPrimitive::Int64 => 5,
        DdsPrimitive::UInt16 => 6,
        DdsPrimitive::UInt32 => 7,
        DdsPrimitive::UInt64 => 8,
        DdsPrimitive::Float32 => 9,
        DdsPrimitive::Float64 => 10,
        DdsPrimitive::String => 16,
    }
}
struct Layout {
    offsets: Vec<usize>,
    size: usize,
    alignment: usize,
}
impl Layout {
    fn new(schema: &DdsType) -> Self {
        let mut result = Self {
            offsets: Vec::new(),
            size: 0,
            alignment: 1,
        };
        for field in &schema.fields {
            let (size, alignment) = if field.sequence {
                (std::mem::size_of::<Sequence>(), 8)
            } else {
                let n = scalar_size(field.data_type);
                (n, n)
            };
            result.size = aligned(result.size, alignment);
            result.offsets.push(result.size);
            result.size += size;
            result.alignment = result.alignment.max(alignment);
        }
        result.size = aligned(result.size, result.alignment);
        result
    }
    fn check_descriptor(&self, descriptor: &TopicDescriptor, schema: &DdsType) -> Result<()> {
        require(
            descriptor.size as usize == self.size
                && descriptor.alignment as usize == self.alignment,
            "DDS Security runtime returned an unsupported native sample layout",
        )?;
        // A final flat struct uses ADR operations with a native offset, followed by RTS.
        // Verify every field offset before allowing any pointer-based native sample access.
        require(
            !descriptor.operations.is_null(),
            "DDS Security type operations are missing",
        )?;
        let operation_count = descriptor.operation_count as usize;
        require(
            operation_count <= 5 * schema.fields.len() + 1,
            "DDS Security type operation limit exceeded",
        )?;
        let mut operation = 0usize;
        for (field, offset) in schema.fields.iter().zip(&self.offsets) {
            require(
                operation + 1 < operation_count,
                "DDS Security type operation limit exceeded",
            )?;
            let opcode = unsafe { *descriptor.operations.add(operation) };
            let native_offset = unsafe { *descriptor.operations.add(operation + 1) };
            require(
                opcode >> 24 == 1 && native_offset as usize == *offset,
                "DDS Security runtime returned unsupported type operations",
            )?;
            operation += 2;
            // Unbounded primitive/string sequences have two-word ADR operations too.
            require(
                !field.sequence || (opcode >> 16) & 0x7f == 7,
                "DDS Security runtime returned an unsupported sequence layout",
            )?;
        }
        require(
            operation < operation_count,
            "DDS Security type terminator is missing",
        )?;
        require(
            unsafe { *descriptor.operations.add(operation) } == 0,
            "DDS Security runtime returned an unsupported final-struct layout",
        )
    }
}

fn domains() -> &'static Mutex<HashMap<i32, Weak<Domain>>> {
    static DOMAINS: OnceLock<Mutex<HashMap<i32, Weak<Domain>>>> = OnceLock::new();
    DOMAINS.get_or_init(|| Mutex::new(HashMap::new()))
}
struct Domain {
    api: Arc<Api>,
    entity: i32,
    id: i32,
    policy: [String; 3],
}
impl Domain {
    fn acquire(api: Arc<Api>, id: i32, security: &DdsSecurity) -> Result<Arc<Self>> {
        let mut domains = domains().lock().map_err(|_| error("domain lock failed"))?;
        let policy = [
            security.identity_ca_pem.clone(),
            security.permissions_ca_pem.clone(),
            security.governance_signed.clone(),
        ];
        if let Some(domain) = domains.get(&id).and_then(Weak::upgrade) {
            drop(domains);
            // Cyclone trusts in-process endpoints without a remote identity handshake.
            // Never let separate trust domains share that local matching shortcut.
            require(
                domain.policy == policy,
                "DDS Security sessions sharing a domain must use identical CA and signed governance inputs",
            )?;
            return Ok(domain);
        }
        // Discovery trace dumps full participant QoS, including private keys. An explicit
        // domain ignores CYCLONEDDS_URI and disables trace before credentials reach DDS.
        let configuration = c"<CycloneDDS><Domain><Tracing><Verbosity>none</Verbosity></Tracing></Domain></CycloneDDS>";
        let entity =
            checked(unsafe { (api.dds_create_domain)(id as u32, configuration.as_ptr()) })?;
        let domain = Arc::new(Self {
            api,
            entity,
            id,
            policy,
        });
        domains.insert(id, Arc::downgrade(&domain));
        Ok(domain)
    }
}
impl Drop for Domain {
    fn drop(&mut self) {
        // Serialize domain removal and acquisition so a new client cannot inherit old tracing.
        let mut domains = domains()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unsafe {
            (self.api.dds_delete)(self.entity);
        }
        domains.remove(&self.id);
    }
}
struct Native {
    _domain: Arc<Domain>,
    api: Arc<Api>,
    participant: i32,
    endpoint: i32,
    config: DdsConfig,
    layout: Layout,
}
impl Drop for Native {
    fn drop(&mut self) {
        if self.participant > 0 {
            unsafe {
                (self.api.dds_delete)(self.participant);
            }
        }
    }
}
struct Qos<'a>(&'a Api, Ptr);
impl Drop for Qos<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.0.dds_delete_qos)(self.1);
        }
    }
}
impl<'a> Qos<'a> {
    fn new(api: &'a Api) -> Result<Self> {
        let pointer = unsafe { (api.dds_create_qos)() };
        require(!pointer.is_null(), "DDS Security QoS allocation failed")?;
        Ok(Self(api, pointer))
    }
    fn property(&self, key: &str, value: &str) -> Result<()> {
        let key = cstring(key)?;
        let value = cstring(value)?;
        unsafe {
            (self.0.dds_qset_prop)(self.1, key.as_ptr(), value.as_ptr());
        }
        Ok(())
    }
}
struct TypeGuard<'a>(&'a Api, DynamicType);
impl Drop for TypeGuard<'_> {
    fn drop(&mut self) {
        if !self.1.pointers[0].is_null() {
            unsafe {
                (self.0.dds_dynamic_type_unref)(&mut self.1);
            }
        }
    }
}
impl TypeGuard<'_> {
    fn transfer(mut self) -> TypeSpec {
        let dynamic = std::mem::take(&mut self.1);
        TypeSpec {
            kind: 1,
            value: TypeValue { dynamic },
        }
    }
}
struct DescriptorGuard<'a>(&'a Api, *mut TopicDescriptor);
impl Drop for DescriptorGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.0.dds_delete_topic_descriptor)(self.1);
        }
    }
}

impl Native {
    fn create(config: DdsConfig) -> Result<Self> {
        let security = config
            .security
            .as_ref()
            .ok_or_else(|| error("security credentials are missing"))?;
        governance::verify(security)?;
        let api = Arc::new(Api::load()?);
        let domain = Domain::acquire(api.clone(), config.domain_id, security)?;
        let qos = Qos::new(&api)?;
        for (kind, library, init, finalize) in [
            (
                "auth",
                "dds_security_auth",
                "init_authentication",
                "finalize_authentication",
            ),
            (
                "access",
                "dds_security_ac",
                "init_access_control",
                "finalize_access_control",
            ),
            (
                "crypto",
                "dds_security_crypto",
                "init_crypto",
                "finalize_crypto",
            ),
        ] {
            for (suffix, value) in [("path", library), ("init", init), ("finalize", finalize)] {
                qos.property(&format!("dds.sec.{kind}.library.{suffix}"), value)?;
            }
        }
        for (key, value) in [
            ("dds.sec.auth.identity_ca", &security.identity_ca_pem),
            (
                "dds.sec.auth.identity_certificate",
                &security.identity_certificate_pem,
            ),
            ("dds.sec.auth.private_key", &security.private_key_pem),
            (
                "dds.sec.access.permissions_ca",
                &security.permissions_ca_pem,
            ),
            ("dds.sec.access.governance", &security.governance_signed),
            ("dds.sec.access.permissions", &security.permissions_signed),
        ] {
            qos.property(key, &format!("data:,{value}"))?;
        }
        qos.property("dds.sec.auth.password", &security.private_key_password)?;
        let participant = checked(unsafe {
            (api.dds_create_participant)(config.domain_id as u32, qos.1, ptr::null_mut())
        })?;
        drop(qos);
        let mut native = Self {
            _domain: domain,
            api: api.clone(),
            participant,
            endpoint: 0,
            layout: Layout::new(&config.data_type),
            config,
        };
        let name = cstring(&native.config.data_type.name)?;
        let mut kind = TypeGuard(&api, unsafe {
            (api.dds_dynamic_type_create)(
                participant,
                TypeDescriptor {
                    kind: 24,
                    name: name.as_ptr(),
                    ..Default::default()
                },
            )
        });
        checked(kind.1.result)?;
        checked(unsafe { (api.dds_dynamic_type_set_extensibility)(&mut kind.1, 0) })?;
        for (id, field) in native.config.data_type.fields.iter().enumerate() {
            let field_name = cstring(&field.name)?;
            // Construct all fallible Rust values before giving nested types to the C API.
            let sequence_name = cstring(&format!("flow_like_sequence_{id}"))?;
            let mut spec = if field.data_type == DdsPrimitive::String {
                let string = unsafe {
                    (api.dds_dynamic_type_create)(
                        participant,
                        TypeDescriptor {
                            kind: 16,
                            ..Default::default()
                        },
                    )
                };
                checked(string.result)?;
                TypeGuard(&api, string).transfer()
            } else {
                TypeSpec {
                    kind: 2,
                    value: TypeValue {
                        primitive: primitive(field.data_type),
                    },
                }
            };
            if field.sequence {
                let sequence = unsafe {
                    (api.dds_dynamic_type_create)(
                        participant,
                        TypeDescriptor {
                            kind: 22,
                            name: sequence_name.as_ptr(),
                            element: spec,
                            ..Default::default()
                        },
                    )
                };
                checked(sequence.result)?;
                spec = TypeGuard(&api, sequence).transfer();
            }
            checked(unsafe {
                (api.dds_dynamic_type_add_member)(
                    &mut kind.1,
                    MemberDescriptor {
                        name: field_name.as_ptr(),
                        id: id as u32,
                        kind: spec,
                        index: u32::MAX,
                        ..Default::default()
                    },
                )
            })?;
            if field.key {
                checked(unsafe { (api.dds_dynamic_member_set_key)(&mut kind.1, id as u32, true) })?;
            }
        }
        let mut info = ptr::null_mut();
        checked(unsafe { (api.dds_dynamic_type_register)(&mut kind.1, &mut info) })?;
        let mut descriptor = ptr::null_mut();
        let result =
            unsafe { (api.dds_create_topic_descriptor)(1, participant, info, 0, &mut descriptor) };
        unsafe {
            (api.dds_free_typeinfo)(info);
        }
        checked(result)?;
        let descriptor = DescriptorGuard(&api, descriptor);
        require(
            !descriptor.1.is_null(),
            "DDS Security topic descriptor is missing",
        )?;
        native
            .layout
            .check_descriptor(unsafe { &*descriptor.1 }, &native.config.data_type)?;
        let topic_name = cstring(&native.config.topic)?;
        let topic = checked(unsafe {
            (api.dds_create_topic)(
                participant,
                descriptor.1,
                topic_name.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        })?;
        let qos = Qos::new(&api)?;
        let depth = native.config.history_depth as i32;
        unsafe {
            (api.dds_qset_reliability)(
                qos.1,
                i32::from(native.config.reliability == DdsReliability::Reliable),
                50_000_000,
            );
            (api.dds_qset_history)(qos.1, 0, depth);
            (api.dds_qset_resource_limits)(qos.1, depth, depth, depth);
            (api.dds_qset_data_representation)(qos.1, 1, &0i16);
            (api.dds_qset_type_consistency)(qos.1, 0, false, false, false, true, true);
        }
        native.endpoint = checked(unsafe {
            match native.config.role {
                DdsRole::Publisher => {
                    (api.dds_create_writer)(participant, topic, qos.1, ptr::null_mut())
                }
                DdsRole::Subscriber => {
                    (api.dds_create_reader)(participant, topic, qos.1, ptr::null_mut())
                }
            }
        })?;
        // The native participant owns copies. Release the adapter's private-key strings now.
        native.config.security = None;
        Ok(native)
    }
    fn matched(&self) -> Result<u32> {
        checked(unsafe {
            match self.config.role {
                DdsRole::Publisher => {
                    (self.api.dds_get_matched_subscriptions)(self.endpoint, ptr::null_mut(), 0)
                }
                DdsRole::Subscriber => {
                    (self.api.dds_get_matched_publications)(self.endpoint, ptr::null_mut(), 0)
                }
            }
        })
        .map(|v| v as u32)
    }
    fn write(&self, value: &Value) -> Result<()> {
        let sample = NativeSample::new(&self.config.data_type, &self.layout, value)?;
        checked(unsafe { (self.api.dds_write)(self.endpoint, sample.data.as_ptr().cast()) })
            .map(|_| ())
    }
    fn ack(&self) -> Result<bool> {
        let result = unsafe { (self.api.dds_wait_for_acks)(self.endpoint, 20_000_000) };
        if result == -10 {
            Ok(false)
        } else {
            checked(result).map(|_| true)
        }
    }
    fn take(&self) -> Result<Option<DdsSample>> {
        let mut pointer = ptr::null_mut();
        let mut info = SampleInfo::default();
        let count =
            checked(unsafe { (self.api.dds_take)(self.endpoint, &mut pointer, &mut info, 1, 1) })?;
        if count == 0 {
            return Ok(None);
        }
        let data = if info.valid_data {
            unsafe { read_sample(&self.config.data_type, &self.layout, pointer) }.map(Some)
        } else {
            Ok(None)
        };
        let returned = unsafe { (self.api.dds_return_loan)(self.endpoint, &mut pointer, 1) };
        checked(returned)?;
        Ok(Some(DdsSample {
            topic: self.config.topic.clone(),
            type_name: self.config.data_type.name.clone(),
            data: data?,
            valid_data: info.valid_data,
            instance_state: match info.instance_state {
                16 => "alive",
                32 => "disposed",
                64 => "no_writers",
                _ => return Err(error("unknown DDS instance state")),
            }
            .into(),
            source_timestamp_ns: (info.source_timestamp != i64::MIN)
                .then_some(info.source_timestamp),
        }))
    }
    fn close(mut self) -> Result<()> {
        checked(unsafe { (self.api.dds_delete)(self.participant) })?;
        self.participant = 0;
        Ok(())
    }
}

// Buffers are aligned to 8 bytes and remain allocated until synchronous dds_write returns.
// Only initialized schema fields are exposed to Cyclone; pointer fields reference owned buffers.
struct NativeSample {
    data: Vec<u64>,
    sequences: Vec<Vec<u64>>,
    strings: Vec<CString>,
}
impl NativeSample {
    fn new(schema: &DdsType, layout: &Layout, value: &Value) -> Result<Self> {
        super::runtime::sample_to_dynamic(
            schema,
            super::runtime::dynamic_type(schema)?,
            value.clone(),
        )?;
        let mut sample = Self {
            data: vec![0; layout.size.div_ceil(8)],
            sequences: Vec::new(),
            strings: Vec::new(),
        };
        for (field, offset) in schema.fields.iter().zip(&layout.offsets) {
            let pointer = unsafe { sample.data.as_mut_ptr().cast::<u8>().add(*offset) };
            if field.sequence {
                let values = value[&field.name]
                    .as_array()
                    .ok_or_else(|| error("sequence must be an array"))?;
                let size = scalar_size(field.data_type);
                let mut buffer = vec![0u64; (values.len() * size).div_ceil(8).max(1)];
                for (index, value) in values.iter().enumerate() {
                    unsafe {
                        sample.write_scalar(
                            buffer.as_mut_ptr().cast::<u8>().add(index * size),
                            field.data_type,
                            value,
                        )?;
                    }
                }
                let sequence = Sequence {
                    maximum: values.len() as u32,
                    length: values.len() as u32,
                    buffer: buffer.as_mut_ptr().cast(),
                    release: false,
                };
                unsafe {
                    pointer.cast::<Sequence>().write(sequence);
                }
                sample.sequences.push(buffer);
            } else {
                unsafe {
                    sample.write_scalar(pointer, field.data_type, &value[&field.name])?;
                }
            }
        }
        Ok(sample)
    }
    unsafe fn write_scalar(
        &mut self,
        pointer: *mut u8,
        kind: DdsPrimitive,
        value: &Value,
    ) -> Result<()> {
        // The complete value was range-checked against the schema before allocating native data.
        unsafe {
            match kind {
                DdsPrimitive::Boolean => pointer.write(u8::from(value.as_bool().unwrap())),
                DdsPrimitive::UInt8 => pointer.write(value.as_u64().unwrap() as u8),
                DdsPrimitive::Int8 => pointer.cast::<i8>().write(value.as_i64().unwrap() as i8),
                DdsPrimitive::UInt16 => pointer.cast::<u16>().write(value.as_u64().unwrap() as u16),
                DdsPrimitive::Int16 => pointer.cast::<i16>().write(value.as_i64().unwrap() as i16),
                DdsPrimitive::UInt32 => pointer.cast::<u32>().write(value.as_u64().unwrap() as u32),
                DdsPrimitive::Int32 => pointer.cast::<i32>().write(value.as_i64().unwrap() as i32),
                DdsPrimitive::UInt64 => pointer.cast::<u64>().write(value.as_u64().unwrap()),
                DdsPrimitive::Int64 => pointer.cast::<i64>().write(value.as_i64().unwrap()),
                DdsPrimitive::Float32 => {
                    pointer.cast::<f32>().write(value.as_f64().unwrap() as f32)
                }
                DdsPrimitive::Float64 => pointer.cast::<f64>().write(value.as_f64().unwrap()),
                DdsPrimitive::String => {
                    let string = cstring(value.as_str().unwrap())?;
                    pointer
                        .cast::<*const std::ffi::c_char>()
                        .write(string.as_ptr());
                    self.strings.push(string);
                }
            }
        }
        Ok(())
    }
}
unsafe fn read_scalar(pointer: *const u8, kind: DdsPrimitive, budget: &mut usize) -> Result<Value> {
    require(*budget >= 8, "DDS sample exceeds native conversion limit")?;
    *budget -= 8;
    unsafe {
        Ok(match kind {
            DdsPrimitive::Boolean => Value::Bool(pointer.read() != 0),
            DdsPrimitive::UInt8 => pointer.read().into(),
            DdsPrimitive::Int8 => pointer.cast::<i8>().read().into(),
            DdsPrimitive::UInt16 => pointer.cast::<u16>().read().into(),
            DdsPrimitive::Int16 => pointer.cast::<i16>().read().into(),
            DdsPrimitive::UInt32 => pointer.cast::<u32>().read().into(),
            DdsPrimitive::Int32 => pointer.cast::<i32>().read().into(),
            DdsPrimitive::UInt64 => pointer.cast::<u64>().read().into(),
            DdsPrimitive::Int64 => pointer.cast::<i64>().read().into(),
            DdsPrimitive::Float32 => {
                serde_json::Number::from_f64(pointer.cast::<f32>().read() as f64)
                    .ok_or_else(|| error("non-finite DDS float"))?
                    .into()
            }
            DdsPrimitive::Float64 => serde_json::Number::from_f64(pointer.cast::<f64>().read())
                .ok_or_else(|| error("non-finite DDS float"))?
                .into(),
            DdsPrimitive::String => {
                let string = pointer.cast::<*const std::ffi::c_char>().read();
                require(!string.is_null(), "DDS string pointer is null")?;
                let bytes = CStr::from_ptr(string).to_bytes();
                require(bytes.len() <= *budget, "DDS string exceeds sample limit")?;
                *budget -= bytes.len();
                std::str::from_utf8(bytes)
                    .map_err(|_| error("DDS string is not UTF-8"))?
                    .into()
            }
        })
    }
}
unsafe fn read_sample(schema: &DdsType, layout: &Layout, pointer: Ptr) -> Result<Value> {
    require(!pointer.is_null(), "DDS sample pointer is null")?;
    let mut object = Map::new();
    let mut budget = 1024 * 1024;
    for (field, offset) in schema.fields.iter().zip(&layout.offsets) {
        let field_pointer = unsafe { pointer.cast::<u8>().add(*offset) };
        let value = if field.sequence {
            let sequence = unsafe { field_pointer.cast::<Sequence>().read() };
            require(
                sequence.length <= 65_536
                    && sequence.length <= sequence.maximum
                    && (sequence.length == 0 || !sequence.buffer.is_null()),
                "DDS sequence exceeds limit or is invalid",
            )?;
            let mut values = Vec::with_capacity(sequence.length as usize);
            for index in 0..sequence.length as usize {
                values.push(unsafe {
                    read_scalar(
                        sequence
                            .buffer
                            .cast::<u8>()
                            .add(index * scalar_size(field.data_type)),
                        field.data_type,
                        &mut budget,
                    )?
                });
            }
            Value::Array(values)
        } else {
            unsafe { read_scalar(field_pointer, field.data_type, &mut budget)? }
        };
        object.insert(field.name.clone(), value);
    }
    let value = Value::Object(object);
    require(
        serde_json::to_vec(&value)
            .map_err(|_| error("DDS sample encoding failed"))?
            .len()
            <= 1024 * 1024,
        "DDS sample exceeds 1048576 bytes",
    )?;
    Ok(value)
}

pub struct SecureDdsClient {
    native: Arc<Mutex<Option<Native>>>,
    stop: CancellationToken,
    timeout: Duration,
    role: DdsRole,
    close_started: AtomicBool,
    cleanup: tokio::sync::watch::Sender<Option<std::result::Result<(), String>>>,
}
impl SecureDdsClient {
    pub async fn connect(config: DdsConfig) -> Result<Self> {
        let timeout = Duration::from_millis(config.timeout_ms);
        let role = config.role;
        let task = super::runtime::sdk_runtime()?.spawn_blocking(move || {
            Native::create(config).map(|native| Self {
                native: Arc::new(Mutex::new(Some(native))),
                stop: CancellationToken::new(),
                timeout,
                role,
                close_started: AtomicBool::new(false),
                cleanup: tokio::sync::watch::channel(None).0,
            })
        });
        tokio::time::timeout(timeout, task)
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(|_| error("native worker failed"))?
    }
    async fn operation<T: Send + 'static>(
        &self,
        action: impl FnOnce(&Native) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let native = self.native.clone();
        let stop = self.stop.clone();
        let task = super::runtime::sdk_runtime()?.spawn_blocking(move || {
            let guard = native.lock().map_err(|_| error("session lock failed"))?;
            require(!stop.is_cancelled(), "DDS session is closed")?;
            action(guard.as_ref().ok_or_else(|| error("session is closed"))?)
        });
        tokio::select! { biased;
            _ = self.stop.cancelled() => Err(error("session is closed")),
            result = task => result.map_err(|_| error("native worker failed"))?,
        }
    }
    async fn bounded<T>(&self, future: impl std::future::Future<Output = Result<T>>) -> Result<T> {
        tokio::select! { biased;
            _ = self.stop.cancelled() => Err(error("session is closed")),
            result = tokio::time::timeout(self.timeout, future) => result.map_err(|_| Error::Timeout)?,
        }
    }
    pub async fn wait_for_match(&self) -> Result<u32> {
        self.bounded(async {
            loop {
                let count = self.operation(Native::matched).await?;
                if count > 0 {
                    return Ok(count);
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
    }
    pub async fn publish(&self, value: Value) -> Result<()> {
        require(
            self.role == DdsRole::Publisher,
            "publish requires a DDS publisher session",
        )?;
        self.bounded(async {
            self.wait_for_match().await?;
            self.operation(move |native| native.write(&value)).await?;
            while !self
                .operation(|native| {
                    if native.config.reliability == DdsReliability::Reliable {
                        native.ack()
                    } else {
                        Ok(true)
                    }
                })
                .await?
            {}
            Ok(())
        })
        .await
    }
    pub async fn receive(&self) -> Result<DdsSample> {
        require(
            self.role == DdsRole::Subscriber,
            "receive requires a DDS subscriber session",
        )?;
        self.bounded(async {
            loop {
                if let Some(sample) = self.operation(Native::take).await? {
                    return Ok(sample);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
    }
    fn begin_close(&self) {
        self.stop.cancel();
        if self.close_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let native = self.native.clone();
        let completion = self.cleanup.clone();
        if let Ok(runtime) = super::runtime::sdk_runtime() {
            runtime.spawn_blocking(move || {
                let result = native
                    .lock()
                    .map_err(|_| error("session lock failed"))
                    .and_then(|mut guard| match guard.take() {
                        Some(native) => native.close(),
                        None => Ok(()),
                    });
                completion.send_replace(Some(result.map_err(|e| e.to_string())));
            });
        }
    }
    pub async fn close(&self) -> Result<()> {
        self.begin_close();
        let mut completion = self.cleanup.subscribe();
        tokio::time::timeout(self.timeout, async {
            loop {
                if let Some(result) = completion.borrow().clone() {
                    return result.map_err(|_| error("native participant cleanup failed"));
                }
                completion
                    .changed()
                    .await
                    .map_err(|_| error("cleanup worker ended"))?;
            }
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}
impl Drop for SecureDdsClient {
    fn drop(&mut self) {
        self.begin_close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn native_layout_round_trip_preserves_all_supported_scalar_and_sequence_types() {
        let values = [
            (DdsPrimitive::Boolean, json!(true)),
            (DdsPrimitive::UInt8, json!(255)),
            (DdsPrimitive::Int8, json!(-128)),
            (DdsPrimitive::UInt16, json!(65535)),
            (DdsPrimitive::Int16, json!(-32768)),
            (DdsPrimitive::UInt32, json!(u32::MAX)),
            (DdsPrimitive::Int32, json!(i32::MIN)),
            (DdsPrimitive::UInt64, json!(u64::MAX)),
            (DdsPrimitive::Int64, json!(i64::MIN)),
            (DdsPrimitive::Float32, json!(1.5)),
            (DdsPrimitive::Float64, json!(-2.25)),
            (DdsPrimitive::String, json!("°C temperature")),
        ];
        let mut schema = DdsType {
            name: "Tests::NativeSample".into(),
            fields: Vec::new(),
        };
        let mut object = Map::new();
        for (index, (data_type, value)) in values.into_iter().enumerate() {
            for sequence in [false, true] {
                let name = format!("field_{index}_{sequence}");
                schema.fields.push(DdsField {
                    name: name.clone(),
                    data_type,
                    sequence,
                    key: false,
                });
                object.insert(
                    name,
                    if sequence {
                        json!([value.clone(), value.clone()])
                    } else {
                        value.clone()
                    },
                );
            }
        }
        let value = Value::Object(object);
        let layout = Layout::new(&schema);
        let sample = NativeSample::new(&schema, &layout, &value).unwrap();
        let decoded =
            unsafe { read_sample(&schema, &layout, sample.data.as_ptr().cast_mut().cast()) }
                .unwrap();
        assert_eq!(decoded, value);
    }
    #[test]
    fn descriptor_validation_checks_native_operation_bounds_before_dereference() {
        let schema = DdsConfig::default().data_type;
        let layout = Layout::new(&schema);
        let operations = [0x01040000u32, 0, 0];
        let mut descriptor = TopicDescriptor {
            size: 8,
            alignment: 8,
            flags: 0,
            key_count: 0,
            name: ptr::null(),
            keys: ptr::null_mut(),
            operation_count: 3,
            operations: ptr::null(),
        };
        assert!(layout.check_descriptor(&descriptor, &schema).is_err());
        descriptor.operations = operations.as_ptr();
        descriptor.operation_count = 1;
        assert!(layout.check_descriptor(&descriptor, &schema).is_err());
        descriptor.operation_count = 2;
        assert!(layout.check_descriptor(&descriptor, &schema).is_err());
        descriptor.operation_count = 3;
        assert!(layout.check_descriptor(&descriptor, &schema).is_ok());
    }
}
