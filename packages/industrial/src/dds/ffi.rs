//! The Cyclone DDS 11 public C ABI used by the optional security backend.
use crate::{Error, Result};
use libloading::Library;
use std::ffi::{c_char, c_void};

pub type Ptr = *mut c_void;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DynamicType {
    pub pointers: [Ptr; 2],
    pub result: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub union TypeValue {
    pub dynamic: DynamicType,
    pub primitive: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TypeSpec {
    pub kind: i32,
    pub value: TypeValue,
}
impl Default for TypeSpec {
    fn default() -> Self {
        Self {
            kind: 0,
            value: TypeValue {
                dynamic: DynamicType::default(),
            },
        }
    }
}
#[repr(C)]
#[derive(Default)]
pub struct TypeDescriptor {
    pub kind: i32,
    pub name: *const c_char,
    pub base: TypeSpec,
    pub discriminator: TypeSpec,
    pub num_bounds: u32,
    pub bounds: *const u32,
    pub element: TypeSpec,
    pub key_element: TypeSpec,
}
#[repr(C)]
#[derive(Default)]
pub struct MemberDescriptor {
    pub name: *const c_char,
    pub id: u32,
    pub kind: TypeSpec,
    pub default_value: *mut c_char,
    pub index: u32,
    pub num_labels: u32,
    pub labels: *mut i32,
    pub default_label: bool,
}
// Only the stable leading fields are read; DDS owns and frees the full descriptor.
#[repr(C)]
pub struct TopicDescriptor {
    pub size: u32,
    pub alignment: u32,
    pub flags: u32,
    pub key_count: u32,
    pub name: *const c_char,
    pub keys: Ptr,
    pub operation_count: u32,
    pub operations: *const u32,
}
#[repr(C)]
#[derive(Default)]
pub struct SampleInfo {
    pub sample_state: i32,
    pub view_state: i32,
    pub instance_state: i32,
    pub valid_data: bool,
    pub source_timestamp: i64,
    pub instance_handle: u64,
    pub publication_handle: u64,
    pub disposed_generation_count: u32,
    pub no_writers_generation_count: u32,
    pub sample_rank: u32,
    pub generation_rank: u32,
    pub absolute_generation_rank: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sequence {
    pub maximum: u32,
    pub length: u32,
    pub buffer: Ptr,
    pub release: bool,
}

pub fn error(message: &'static str) -> Error {
    Error::Other(anyhow::anyhow!("DDS Security: {message}"))
}
pub fn checked(value: i32) -> Result<i32> {
    if value < 0 {
        Err(Error::Other(anyhow::anyhow!(
            "DDS Security operation failed (DDS code {value})"
        )))
    } else {
        Ok(value)
    }
}

macro_rules! api {
    ($($name:ident: ($($arg:ty),*) -> $ret:ty),* $(,)?) => {
        pub struct Api { $(pub $name: unsafe extern "C" fn($($arg),*) -> $ret,)* _library: Library }
        impl Api {
            pub fn load() -> Result<Self> {
                #[cfg(target_os = "windows")]
                const NAMES: &[&str] = &["ddsc.dll"];
                #[cfg(target_os = "macos")]
                const NAMES: &[&str] = &["libddsc.11.dylib"];
                #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                const NAMES: &[&str] = &["libddsc.so.11"];
                require_64_bit()?;
                // The loader searches only fixed platform library names. No workflow can supply a library path.
                let library = NAMES.iter().find_map(|name| unsafe { Library::new(name).ok() })
                    .ok_or_else(|| error("Cyclone DDS 11 runtime library is not installed or cannot be loaded"))?;
                unsafe { let api = Self { $($name: *library.get(concat!(stringify!($name), "\0").as_bytes())
                    .map_err(|_| error("installed Cyclone DDS runtime does not provide the required version 11 API"))?,)* _library: library };
                    Ok(api) }
            }
        }
    }
}
fn require_64_bit() -> Result<()> {
    crate::require(
        std::mem::size_of::<usize>() == 8,
        "DDS Security requires a 64-bit Cyclone DDS 11 runtime",
    )
}
api! {
    dds_create_domain: (u32, *const c_char) -> i32,
    dds_create_qos: () -> Ptr,
    dds_delete_qos: (Ptr) -> (),
    dds_qset_prop: (Ptr, *const c_char, *const c_char) -> (),
    dds_qset_reliability: (Ptr, i32, i64) -> (),
    dds_qset_history: (Ptr, i32, i32) -> (),
    dds_qset_resource_limits: (Ptr, i32, i32, i32) -> (),
    dds_qset_data_representation: (Ptr, u32, *const i16) -> (),
    dds_qset_type_consistency: (Ptr, i32, bool, bool, bool, bool, bool) -> (),
    dds_create_participant: (u32, Ptr, Ptr) -> i32,
    dds_create_topic: (i32, *const TopicDescriptor, *const c_char, Ptr, Ptr) -> i32,
    dds_create_writer: (i32, i32, Ptr, Ptr) -> i32,
    dds_create_reader: (i32, i32, Ptr, Ptr) -> i32,
    dds_delete: (i32) -> i32,
    dds_dynamic_type_create: (i32, TypeDescriptor) -> DynamicType,
    dds_dynamic_type_set_extensibility: (*mut DynamicType, i32) -> i32,
    dds_dynamic_type_add_member: (*mut DynamicType, MemberDescriptor) -> i32,
    dds_dynamic_member_set_key: (*mut DynamicType, u32, bool) -> i32,
    dds_dynamic_type_register: (*mut DynamicType, *mut Ptr) -> i32,
    dds_dynamic_type_unref: (*mut DynamicType) -> i32,
    dds_free_typeinfo: (Ptr) -> i32,
    dds_create_topic_descriptor: (i32, i32, Ptr, i64, *mut *mut TopicDescriptor) -> i32,
    dds_delete_topic_descriptor: (*mut TopicDescriptor) -> i32,
    dds_get_matched_subscriptions: (i32, *mut u64, usize) -> i32,
    dds_get_matched_publications: (i32, *mut u64, usize) -> i32,
    dds_write: (i32, *const c_void) -> i32,
    dds_wait_for_acks: (i32, i64) -> i32,
    dds_take: (i32, *mut Ptr, *mut SampleInfo, usize, u32) -> i32,
    dds_return_loan: (i32, *mut Ptr, i32) -> i32,
}

// Paired with the independently compiled Cyclone header assertions in the E2E fixture.
#[cfg(target_pointer_width = "64")]
const _: () = {
    use std::mem::{align_of, offset_of, size_of};
    assert!(size_of::<DynamicType>() == 24 && offset_of!(DynamicType, result) == 16);
    assert!(size_of::<TypeSpec>() == 32 && offset_of!(TypeSpec, value) == 8);
    assert!(size_of::<TypeDescriptor>() == 160 && align_of::<TypeDescriptor>() == 8);
    assert!(offset_of!(TypeDescriptor, name) == 8 && offset_of!(TypeDescriptor, base) == 16);
    assert!(
        offset_of!(TypeDescriptor, discriminator) == 48
            && offset_of!(TypeDescriptor, num_bounds) == 80
    );
    assert!(offset_of!(TypeDescriptor, bounds) == 88 && offset_of!(TypeDescriptor, element) == 96);
    assert!(offset_of!(TypeDescriptor, key_element) == 128);
    assert!(size_of::<MemberDescriptor>() == 80 && align_of::<MemberDescriptor>() == 8);
    assert!(offset_of!(MemberDescriptor, id) == 8 && offset_of!(MemberDescriptor, kind) == 16);
    assert!(
        offset_of!(MemberDescriptor, default_value) == 48
            && offset_of!(MemberDescriptor, index) == 56
    );
    assert!(
        offset_of!(MemberDescriptor, num_labels) == 60
            && offset_of!(MemberDescriptor, labels) == 64
    );
    assert!(offset_of!(MemberDescriptor, default_label) == 72);
    assert!(size_of::<TopicDescriptor>() == 48 && align_of::<TopicDescriptor>() == 8);
    assert!(offset_of!(TopicDescriptor, alignment) == 4 && offset_of!(TopicDescriptor, flags) == 8);
    assert!(
        offset_of!(TopicDescriptor, key_count) == 12 && offset_of!(TopicDescriptor, name) == 16
    );
    assert!(
        offset_of!(TopicDescriptor, keys) == 24
            && offset_of!(TopicDescriptor, operation_count) == 32
    );
    assert!(offset_of!(TopicDescriptor, operations) == 40);
    assert!(size_of::<SampleInfo>() == 64 && align_of::<SampleInfo>() == 8);
    assert!(offset_of!(SampleInfo, view_state) == 4 && offset_of!(SampleInfo, instance_state) == 8);
    assert!(
        offset_of!(SampleInfo, valid_data) == 12 && offset_of!(SampleInfo, source_timestamp) == 16
    );
    assert!(
        offset_of!(SampleInfo, instance_handle) == 24
            && offset_of!(SampleInfo, publication_handle) == 32
    );
    assert!(
        offset_of!(SampleInfo, disposed_generation_count) == 40
            && offset_of!(SampleInfo, no_writers_generation_count) == 44
    );
    assert!(
        offset_of!(SampleInfo, sample_rank) == 48 && offset_of!(SampleInfo, generation_rank) == 52
    );
    assert!(offset_of!(SampleInfo, absolute_generation_rank) == 56);
    assert!(size_of::<Sequence>() == 24 && align_of::<Sequence>() == 8);
    assert!(
        offset_of!(Sequence, length) == 4
            && offset_of!(Sequence, buffer) == 8
            && offset_of!(Sequence, release) == 16
    );
};
