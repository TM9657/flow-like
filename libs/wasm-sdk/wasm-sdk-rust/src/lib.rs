//! Flow-Like WASM SDK
//!
//! This SDK provides types, macros, and utilities for building WASM nodes
//! that can be executed by the Flow-Like runtime.
//!
//! # Quick Start - Single Node
//!
//! ```rust
//! use flow_like_wasm_sdk::*;
//!
//! node! {
//!     name: "my_node",
//!     friendly_name: "My Node",
//!     description: "Does something useful",
//!     category: "Custom/Example",
//!
//!     inputs: {
//!         exec: Exec,
//!         input_text: String,
//!     },
//!
//!     outputs: {
//!         exec_out: Exec,
//!         result: String,
//!     },
//! }
//!
//! run_node!(handle_run);
//!
//! fn handle_run(mut ctx: Context) -> ExecutionResult {
//!     let text = ctx.get_string("input_text").unwrap_or_default();
//!     ctx.set_output("result", text.to_uppercase());
//!     ctx.success()
//! }
//! ```
//!
//! # Quick Start - Multi-Node Package
//!
//! ```rust
//! use flow_like_wasm_sdk::*;
//!
//! package! {
//!     nodes: [
//!         {
//!             name: "add",
//!             friendly_name: "Add Numbers",
//!             description: "Adds two numbers",
//!             category: "Math/Arithmetic",
//!             inputs: { exec: Exec, a: I64 = 0, b: I64 = 0 },
//!             outputs: { exec_out: Exec, result: I64 },
//!         },
//!         {
//!             name: "subtract",
//!             friendly_name: "Subtract Numbers",
//!             description: "Subtracts two numbers",
//!             category: "Math/Arithmetic",
//!             inputs: { exec: Exec, a: I64 = 0, b: I64 = 0 },
//!             outputs: { exec_out: Exec, result: I64 },
//!         }
//!     ]
//! }
//! ```

mod context;
mod host;
mod macros;
pub mod mock;
mod types;

pub use context::*;
pub use macros::*;
pub use mock::*;
pub use serde_json::json;
pub use types::*;

// Re-export host functions under namespaces
pub mod log {
    pub use crate::host::{debug, error, info, log_json, trace, warn};
}

pub mod stream {
    pub use crate::host::{stream, stream_json, stream_progress, stream_text, stream_text_raw};
}

pub mod var {
    pub use crate::host::{delete_variable, get_variable, has_variable, set_variable};
}

pub mod util {
    pub use crate::host::{now, random, read_packed_result};
}

pub mod cache {
    pub use crate::host::{cache_delete, cache_get, cache_has, cache_set};
}

pub mod meta {
    pub use crate::host::{
        get_app_id_from_host, get_board_id_from_host, get_log_level_from_host,
        get_node_id_from_host, get_run_id_from_host, get_user_id_from_host, is_streaming_from_host,
    };
}

pub mod storage {
    pub use crate::host::{storage_read, storage_write};
}

pub mod http {
    pub use crate::host::http_request;
}

pub mod auth {
    pub use crate::host::{get_oauth_token, has_oauth_token};
}
