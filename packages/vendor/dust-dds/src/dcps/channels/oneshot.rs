use core::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use alloc::{string::String, sync::Arc};
use critical_section::Mutex;

use crate::infrastructure::error::DdsError;

#[derive(Debug)]
pub enum OneshotRecvError {
    SenderDropped,
}

impl From<OneshotRecvError> for DdsError {
    fn from(_: OneshotRecvError) -> Self {
        DdsError::Error(String::from("Internal error: Oneshot sender dropped"))
    }
}

pub fn oneshot<T>() -> (OneshotSender<T>, OneshotReceiver<T>) {
    let inner = Arc::new(Mutex::new(RefCell::new(OneshotInner {
        data: None,
        waker: None,
        has_sender: true,
    })));
    (
        OneshotSender {
            inner: inner.clone(),
        },
        OneshotReceiver { inner },
    )
}

struct OneshotInner<T> {
    data: Option<T>,
    waker: Option<Waker>,
    has_sender: bool,
}

pub struct OneshotSender<T> {
    inner: Arc<Mutex<RefCell<OneshotInner<T>>>>,
}

impl<T> OneshotSender<T> {
    pub fn send(self, value: T) {
        critical_section::with(|cs| {
            let mut inner_lock = self.inner.borrow(cs).borrow_mut();
            inner_lock.data.replace(value);
            if let Some(w) = inner_lock.waker.take() {
                w.wake()
            }
        })
    }
}

impl<T> Drop for OneshotSender<T> {
    fn drop(&mut self) {
        critical_section::with(|cs| {
            let mut inner_lock = self.inner.borrow(cs).borrow_mut();
            inner_lock.has_sender = false;
            // When the sender is dropped wake the waiting task
            // so that it can finish knowing it won't get any message
            if let Some(w) = inner_lock.waker.take() {
                w.wake()
            }
        })
    }
}

pub struct OneshotReceiver<T> {
    inner: Arc<Mutex<RefCell<OneshotInner<T>>>>,
}

impl<T> Future for OneshotReceiver<T> {
    type Output = Result<T, DdsError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        critical_section::with(|cs| {
            let mut inner_lock = self.inner.borrow(cs).borrow_mut();
            if let Some(value) = inner_lock.data.take() {
                Poll::Ready(Ok(value))
            } else if !inner_lock.has_sender {
                Poll::Ready(Err(DdsError::AlreadyDeleted))
            } else {
                inner_lock.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        })
    }
}
