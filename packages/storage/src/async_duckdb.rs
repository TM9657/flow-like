// From https://github.com/jessekrubin/async-duckdb
pub mod client;
pub mod error;
pub mod pool;

pub use duckdb;
pub use duckdb::{Config, Connection};

pub use client::{Client, ClientBuilder};
pub use error::Error;
pub use pool::{Pool, PoolBuilder};