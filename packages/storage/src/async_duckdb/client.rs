use crate::async_duckdb::Error;
use std::{
    path::{Path, PathBuf},
    thread,
};

use crossbeam_channel::{Sender, bounded, unbounded};
use duckdb::{Config, Connection};
use futures_channel::oneshot;

/// A `ClientBuilder` can be used to create a [`Client`] with custom
/// configuration.
///
/// For more information on creating a duckdb connection, see the
/// [duckdb docs](duckdb::Connection::open()).
///
/// # Examples
///
/// ```rust
/// # use async_duckdb::ClientBuilder;
/// # async fn run() -> Result<(), async_duckdb::Error> {
/// let client = ClientBuilder::new().path("path/to/db.duckdb").open().await?;
///
/// // ...
///
/// client.close().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct ClientBuilder {
    pub(crate) path: Option<PathBuf>,
    pub(crate) flagsfn: Option<fn() -> duckdb::Result<Config>>,
}

impl ClientBuilder {
    /// Returns a new [`ClientBuilder`] with the default settings.
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

    /// Returns a new [`Client`] that uses the `ClientBuilder` configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use async_duckdb::ClientBuilder;
    /// # async fn run() -> Result<(), async_duckdb::Error> {
    /// let client = ClientBuilder::new().open().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open(self) -> Result<Client, Error> {
        Client::open_async(self).await
    }

    /// Returns a new [`Client`] that uses the `ClientBuilder` configuration,
    /// blocking the current thread.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use async_duckdb::ClientBuilder;
    /// # fn run() -> Result<(), async_duckdb::Error> {
    /// let client = ClientBuilder::new().open_blocking()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open_blocking(self) -> Result<Client, Error> {
        Client::open_blocking(self)
    }
}

enum Command {
    Func(Box<dyn FnOnce(&mut Connection) + Send>),
    Shutdown(Box<dyn FnOnce(Result<(), Error>) + Send>),
}

/// Client represents a single duckdb connection that can be used from async
/// contexts.
#[derive(Clone)]
pub struct Client {
    conn_tx: Sender<Command>,
}

impl Client {
    async fn open_async(builder: ClientBuilder) -> Result<Self, Error> {
        let (open_tx, open_rx) = oneshot::channel();
        Self::open(builder, |res| {
            _ = open_tx.send(res);
        });
        open_rx.await?
    }

    fn open_blocking(builder: ClientBuilder) -> Result<Self, Error> {
        let (conn_tx, conn_rx) = bounded(1);
        Self::open(builder, move |res| {
            _ = conn_tx.send(res);
        });
        conn_rx.recv()?
    }

    fn open<F>(builder: ClientBuilder, func: F)
    where
        F: FnOnce(Result<Self, Error>) + Send + 'static,
    {
        thread::spawn(move || {
            let (conn_tx, conn_rx) = unbounded();

            let mut conn = match Client::create_conn(builder) {
                Ok(conn) => conn,
                Err(err) => {
                    func(Err(err));
                    return;
                }
            };

            let client = Self { conn_tx };
            func(Ok(client));

            while let Ok(cmd) = conn_rx.recv() {
                match cmd {
                    Command::Func(func) => func(&mut conn),
                    Command::Shutdown(func) => match conn.close() {
                        Ok(()) => {
                            func(Ok(()));
                            return;
                        }
                        Err((c, e)) => {
                            conn = c;
                            func(Err(e.into()));
                        }
                    },
                }
            }
        });
    }

    fn create_conn(mut builder: ClientBuilder) -> Result<Connection, Error> {
        let path = builder.path.take().unwrap_or_else(|| ":memory:".into());
        let config = if let Some(flagsfn) = builder.flagsfn {
            flagsfn()?
        } else {
            Config::default()
        };
        let conn = Connection::open_with_flags(path, config)?;
        Ok(conn)
    }

    /// Invokes the provided function with a [`duckdb::Connection`].
    pub async fn conn<F, T>(&self, func: F) -> Result<T, Error>
    where
        F: FnOnce(&Connection) -> Result<T, duckdb::Error> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.conn_tx.send(Command::Func(Box::new(move |conn| {
            _ = tx.send(func(conn));
        })))?;
        Ok(rx.await??)
    }

    /// Invokes the provided function with a mutable [`duckdb::Connection`].
    pub async fn conn_mut<F, T>(&self, func: F) -> Result<T, Error>
    where
        F: FnOnce(&mut Connection) -> Result<T, duckdb::Error> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.conn_tx.send(Command::Func(Box::new(move |conn| {
            _ = tx.send(func(conn));
        })))?;
        Ok(rx.await??)
    }

    /// Closes the underlying duckdb connection.
    ///
    /// After this method returns, all calls to `self::conn()` or
    /// `self::conn_mut()` will return an [`Error::Closed`] error.
    pub async fn close(&self) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        let func = Box::new(|res| _ = tx.send(res));
        if self.conn_tx.send(Command::Shutdown(func)).is_err() {
            // If the worker thread has already shut down, return Ok here.
            return Ok(());
        }
        // If receiving fails, the connection is already closed.
        rx.await.unwrap_or(Ok(()))
    }

    /// Invokes the provided function with a [`duckdb::Connection`], blocking
    /// the current thread until completion.
    pub fn conn_blocking<F, T>(&self, func: F) -> Result<T, Error>
    where
        F: FnOnce(&Connection) -> Result<T, duckdb::Error> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = bounded(1);
        self.conn_tx.send(Command::Func(Box::new(move |conn| {
            _ = tx.send(func(conn));
        })))?;
        Ok(rx.recv()??)
    }

    /// Invokes the provided function with a mutable [`duckdb::Connection`],
    /// blocking the current thread until completion.
    pub fn conn_mut_blocking<F, T>(&self, func: F) -> Result<T, Error>
    where
        F: FnOnce(&mut Connection) -> Result<T, duckdb::Error> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = bounded(1);
        self.conn_tx.send(Command::Func(Box::new(move |conn| {
            _ = tx.send(func(conn));
        })))?;
        Ok(rx.recv()??)
    }

    /// Closes the underlying duckdb connection, blocking the current thread
    /// until complete.
    ///
    /// After this method returns, all calls to `self::conn_blocking()` or
    /// `self::conn_mut_blocking()` will return an [`Error::Closed`] error.
    pub fn close_blocking(&self) -> Result<(), Error> {
        let (tx, rx) = bounded(1);
        let func = Box::new(move |res| _ = tx.send(res));
        if self.conn_tx.send(Command::Shutdown(func)).is_err() {
            return Ok(());
        }
        // If receiving fails, the connection is already closed.
        rx.recv().unwrap_or(Ok(()))
    }
}
