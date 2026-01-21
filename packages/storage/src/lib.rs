pub mod arrow_utils;
pub mod databases;
pub mod files;

pub use arrow;
pub use arrow_array;
pub use arrow_schema;
pub use blake3;
pub use datafusion;
pub use lancedb;
pub use num_cpus;
pub use object_store;
pub use object_store::path::Path;
pub use serde_arrow;

// Re-export data lake formats
#[cfg(feature = "delta")]
pub use deltalake;

#[cfg(feature = "iceberg")]
pub use iceberg;

#[cfg(feature = "iceberg")]
pub use iceberg_datafusion;

// Federation support for query push-down to remote databases
#[cfg(feature = "federation")]
pub use datafusion_federation;
