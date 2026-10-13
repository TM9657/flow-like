use super::infrastructure::time::Time;
use core::{future::Future, pin::Pin, task::Poll};

/// Represents a clock that provides the current time.
pub trait Clock {
    /// Returns the current time.
    fn now(&self) -> Time;
}

/// Provides delay/sleep functionality in an asynchronous context.
pub trait Timer: Clone + Send + Sync + 'static {
    /// Creates a future that completes after the specified duration has elapsed.
    ///
    /// # Arguments
    /// * `duration` - The amount of time to delay for
    fn delay(&mut self, duration: core::time::Duration) -> impl Future<Output = ()> + Send;
}

/// Represents a handle to a spawned task.
pub trait TaskHandle: Send + Sync + 'static {
    /// Join the task, waiting for it to finish.
    fn join(&self);
}

/// Provides task spawning capabilities for asynchronous execution.
pub trait Spawner: Clone + Send + Sync + 'static {
    /// The handle returned when spawning a task, allowing it to be joined.
    type TaskHandle: TaskHandle;

    /// Spawns a new asynchronous task.
    ///
    /// # Arguments
    /// * `f` - Future to be spawned as a new task
    fn spawn(&self, f: impl Future<Output = ()> + Send + 'static) -> Self::TaskHandle;
}

/// Main runtime trait for the DDS system, providing access to various async primitives
/// and runtime capabilities.
///
/// This trait combines clock, timer, spawner, and channel functionality into a cohesive runtime.
pub trait DdsRuntime: Send + 'static {
    /// Type representing a clock implementation for this runtime
    type ClockHandle: Clock + Clone + Send + Sync + 'static;
    /// Type representing a timer implementation for this runtime
    type TimerHandle: Timer + Clone + Send + Sync + 'static;
    /// Type representing a spawner implementation for this runtime
    type SpawnerHandle: Spawner + Clone + Send + Sync + 'static;

    /// Returns a timer handle for this runtime
    fn timer(&self) -> Self::TimerHandle;
    /// Returns a clock handle for this runtime
    fn clock(&self) -> Self::ClockHandle;
    /// Returns a spawner handle for this runtime
    fn spawner(&self) -> Self::SpawnerHandle;
}

/// Generic either enumeration type
pub(crate) enum Either<A, B> {
    /// Variant A of either
    A(A),
    /// Variant B of either
    B(B),
}

/// Generic three-way either enumeration type
pub(crate) enum Either3<A, B, C> {
    /// Variant A of either3
    A(A),
    /// Variant B of either3
    B(B),
    /// Variant C of either3
    C(C),
}

struct Select<A, B> {
    a: A,
    b: B,
}

struct Select3<A, B, C> {
    a: A,
    b: B,
    c: C,
}

/// Run two futures in parallel and return the first one to conclude. In case both futures are ready at the same time
/// priority is given to the future A.
pub(crate) async fn select_future<A: Future, B: Future>(
    a: A,
    b: B,
) -> Either<A::Output, B::Output> {
    Select {
        a: core::pin::pin!(a),
        b: core::pin::pin!(b),
    }
    .await
}

/// Run three futures in parallel and return the first one to conclude. In case multiple futures are ready at the same time
/// priority is given in order A, B, C.
pub(crate) async fn select3_future<A: Future, B: Future, C: Future>(
    a: A,
    b: B,
    c: C,
) -> Either3<A::Output, B::Output, C::Output> {
    Select3 {
        a: core::pin::pin!(a),
        b: core::pin::pin!(b),
        c: core::pin::pin!(c),
    }
    .await
}

impl<A, B> Future for Select<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    type Output = Either<A::Output, B::Output>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if let Poll::Ready(a) = Pin::new(&mut self.a).poll(cx) {
            return Poll::Ready(Either::A(a));
        }
        if let Poll::Ready(b) = Pin::new(&mut self.b).poll(cx) {
            return Poll::Ready(Either::B(b));
        }
        Poll::Pending
    }
}

impl<A, B, C> Future for Select3<A, B, C>
where
    A: Future + Unpin,
    B: Future + Unpin,
    C: Future + Unpin,
{
    type Output = Either3<A::Output, B::Output, C::Output>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if let Poll::Ready(a) = Pin::new(&mut self.a).poll(cx) {
            return Poll::Ready(Either3::A(a));
        }
        if let Poll::Ready(b) = Pin::new(&mut self.b).poll(cx) {
            return Poll::Ready(Either3::B(b));
        }
        if let Poll::Ready(c) = Pin::new(&mut self.c).poll(cx) {
            return Poll::Ready(Either3::C(c));
        }
        Poll::Pending
    }
}
