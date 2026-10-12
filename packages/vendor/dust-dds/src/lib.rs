#![forbid(unsafe_code)]
#![forbid(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

extern crate alloc;
#[cfg(feature = "std")]
mod dds;

#[cfg(feature = "std")]
pub use dds::*;

/// Contains the async version of the DDS API.
pub mod dds_async;

/// Contains the DCPS logic which provides the behavior to the DDS API
mod dcps;

pub use dcps::{builtin_topics, infrastructure};

#[doc(hidden)]
pub mod rtps;

#[doc(hidden)]
pub mod rtps_messages;

#[cfg(feature = "rtps_udp_transport")]
#[doc(hidden)]
pub mod rtps_udp_transport;

#[doc(hidden)]
/// Contains the Dust DDS transport interface definition.
pub mod transport;

/// Contains the Dust DDS runtime abstractions.
pub mod runtime;

#[cfg(feature = "std")]
#[doc(hidden)]
pub mod std_runtime;

/// Contains the DDS XTypes standard types and methods definitions
pub mod xtypes;

// To enable using our own derive macros to allow the name dust_dds:: to be used
extern crate self as dust_dds;
