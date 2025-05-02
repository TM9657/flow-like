use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, Ordering::Relaxed},
        Arc,
    },
    thread::available_parallelism,
};

use crate::async_duckdb::{Client, ClientBuilder, Error};

use duckdb::{Config, Connection};
use futures_util::future::join_all;

/// A `PoolBuilder` can be used to create a [`Pool`] with custom
/// configuration.
///
/// See [`Client`] for more information.
///
/// # Examples
///
/// ```rust
/// # use async_duckdb::PoolBuilder;
/// # async fn run() -> Result<(), async_duckdb::Error> {
/// let pool = PoolBuilder::new().path("path/to/db.duck").open().await?;
///
/// // ...
///
/// pool.close().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct PoolBuilder {
    pub(crate) path: Option<PathBuf>,
    pub(crate) flagsfn: Option<fn() -> duckdb::Result<Config>>,
    pub(crate) num_conns: Option<usize>,
}

impl PoolBuilder {
    /// Returns a new [`PoolBuilder`] with the default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Specify the path of the duckdb database to open.
    ///
    /// By default, an in-memory database is used.
    #[must_use]
    pub fn path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = Some(path.as_ref().into());
        if self.flagsfn.is_none() {
            let cfg_fn = || Config::default().access_mode(duckdb::AccessMode::ReadOnly);
            self.flagsfn = Some(cfg_fn);
        }
        self
    }

    /// Specify the [`OpenFlags`] to use when opening a new connection.
    ///
    /// By default, [`OpenFlags::default()`] is used.
    #[must_use]
    pub fn flagsfn(mut self, flags: fn() -> duckdb::Result<Config>) -> Self {
        self.flagsfn = Some(flags);
        self
    }

    /// Specify the number of duckdb connections to open as part of the pool.
    ///
    /// Defaults to the number of logical CPUs of the current system.
    #[must_use]
    pub fn num_conns(mut self, num_conns: usize) -> Self {
        self.num_conns = Some(num_conns);
        self
    }

    /// Returns a new [`Pool`] that uses the `PoolBuilder` configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use async_duckdb::PoolBuilder;
    /// # async fn run() -> Result<(), async_duckdb::Error> {
    /// let pool = PoolBuilder::new().open().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open(self) -> Result<Pool, Error> {
        let num_conns = self.get_num_conns();
        let opens = (0..num_conns).map(|_| {
            ClientBuilder {
                path: self.path.clone(),
                flagsfn: self.flagsfn,
            }
            .open()
        });
        let clients = join_all(opens)
            .await
            .into_iter()
            .collect::<Result<Vec<Client>, Error>>()?;
        Ok(Pool {
            state: Arc::new(State {
                clients,
                counter: AtomicU32::new(0),
            }),
        })
    }

    /// Returns a new [`Pool`] that uses the `PoolBuilder` configuration,
    /// blocking the current thread.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use async_duckdb::PoolBuilder;
    /// # fn run() -> Result<(), async_duckdb::Error> {
    /// let pool = PoolBuilder::new().open_blocking()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open_blocking(self) -> Result<Pool, Error> {
        let num_conns = self.get_num_conns();
        let clients = (0..num_conns)
            .map(|_| {
                ClientBuilder {
                    path: self.path.clone(),
                    flagsfn: self.flagsfn,
                }
                .open_blocking()
            })
            .collect::<Result<Vec<Client>, Error>>()?;
        Ok(Pool {
            state: Arc::new(State {
                clients,
                counter: AtomicU32::new(0),
            }),
        })
    }

    fn get_num_conns(&self) -> usize {
        self.num_conns.unwrap_or_else(|| {
            match available_parallelism() {
                Ok(n) => n.get(),
                Err(_) => 1,
            }

            // if let Err(e)  = available_parallelism() {
            //     1
            // available_parallelism()
            //     .unwrap_or_else(|_| NonZeroUsize::new(1).unwrap())
            //     .into()
        })
    }
}

/// A simple Pool of duckdb connections.
///
/// A Pool has the same API as an individual [`Client`].
#[derive(Clone)]
pub struct Pool {
    state: Arc<State>,
}

struct State {
    clients: Vec<Client>,
    counter: AtomicU32,
}

impl Pool {
    /// Invokes the provided function with a [`duckdb::Connection`].
    pub async fn conn<F, T>(&self, func: F) -> Result<T, Error>
    where
        F: FnOnce(&Connection) -> Result<T, duckdb::Error> + Send + 'static,
        T: Send + 'static,
    {
        self.get().conn(func).await
    }

    /// Invokes the provided function with a mutable [`duckdb::Connection`].
    pub async fn conn_mut<F, T>(&self, func: F) -> Result<T, Error>
    where
        F: FnOnce(&mut Connection) -> Result<T, duckdb::Error> + Send + 'static,
        T: Send + 'static,
    {
        self.get().conn_mut(func).await
    }

    /// Closes the underlying duckdb connections.
    ///
    /// After this method returns, all calls to `self::conn()` or
    /// `self::conn_mut()` will return an [`Error::Closed`] error.
    pub async fn close(&self) -> Result<(), Error> {
        for client in &self.state.clients {
            client.close().await?;
        }
        Ok(())
    }

    /// Invokes the provided function with a [`duckdb::Connection`], blocking
    /// the current thread.
    pub fn conn_blocking<F, T>(&self, func: F) -> Result<T, Error>
    where
        F: FnOnce(&Connection) -> Result<T, duckdb::Error> + Send + 'static,
        T: Send + 'static,
    {
        self.get().conn_blocking(func)
    }

    /// Invokes the provided function with a mutable [`duckdb::Connection`],
    /// blocking the current thread.
    pub fn conn_mut_blocking<F, T>(&self, func: F) -> Result<T, Error>
    where
        F: FnOnce(&mut Connection) -> Result<T, duckdb::Error> + Send + 'static,
        T: Send + 'static,
    {
        self.get().conn_mut_blocking(func)
    }

    /// Closes the underlying duckdb connections, blocking the current thread.
    ///
    /// After this method returns, all calls to `self::conn_blocking()` or
    /// `self::conn_mut_blocking()` will return an [`Error::Closed`] error.
    pub fn close_blocking(&self) -> Result<(), Error> {
        self.state
            .clients
            .iter()
            .try_for_each(super::client::Client::close_blocking)
    }

    fn get(&self) -> &Client {
        let n = self.state.counter.fetch_add(1, Relaxed);
        &self.state.clients[n as usize % self.state.clients.len()]
    }

    /// Runs a function on all connections in the pool asynchronously.
    ///
    /// The function is executed on each connection concurrently.
    pub async fn conn_for_each<F, T>(&self, func: F) -> Vec<Result<T, Error>>
    where
        F: Fn(&Connection) -> Result<T, duckdb::Error> + Send + Sync + 'static,
        T: Send + 'static,
    {
        let func = Arc::new(func);
        let futures = self.state.clients.iter().map(|client| {
            let func = func.clone();
            async move { client.conn(move |conn| func(conn)).await }
        });
        join_all(futures).await
    }

    /// Runs a function on all connections in the pool, blocking the current thread.
    pub fn conn_for_each_blocking<F, T>(&self, func: F) -> Vec<Result<T, Error>>
    where
        F: Fn(&Connection) -> Result<T, duckdb::Error> + Send + Sync + 'static,
        T: Send + 'static,
    {
        let func = Arc::new(func);
        self.state
            .clients
            .iter()
            .map(|client| {
                let func = func.clone();
                client.conn_blocking(move |conn| func(conn))
            })
            .collect()
    }
}