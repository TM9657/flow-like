use core::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use alloc::{collections::VecDeque, sync::Arc};

use crate::infrastructure::error::DdsError;

use critical_section::Mutex;

pub fn mpsc_channel<T>() -> (MpscSender<T>, MpscReceiver<T>) {
    let inner = Arc::new(Mutex::new(RefCell::new(MpscInner {
        data: VecDeque::with_capacity(64),
        waker: None,
        is_closed: false,
    })));
    (
        MpscSender {
            inner: inner.clone(),
        },
        MpscReceiver { inner },
    )
}

struct MpscInner<T> {
    data: VecDeque<T>,
    waker: Option<Waker>,
    is_closed: bool,
}

impl<T> core::fmt::Debug for MpscInner<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MpscInner")
            .field("data_length", &self.data.len())
            .field("waker", &self.waker)
            .finish()
    }
}

#[derive(Debug)]
pub enum MpscSenderError {
    Closed,
}

impl From<MpscSenderError> for DdsError {
    fn from(_value: MpscSenderError) -> Self {
        DdsError::AlreadyDeleted
    }
}

pub struct MpscSender<T> {
    inner: Arc<Mutex<RefCell<MpscInner<T>>>>,
}

impl<T> Clone for MpscSender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> core::fmt::Debug for MpscSender<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MpscSender")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<T> MpscSender<T> {
    pub fn send(&self, value: T) -> Result<(), MpscSenderError> {
        critical_section::with(|cs| {
            let mut inner_lock = self.inner.borrow(cs).borrow_mut();
            if inner_lock.is_closed {
                Err(MpscSenderError::Closed)
            } else {
                inner_lock.data.push_back(value);
                if let Some(w) = inner_lock.waker.take() {
                    w.wake();
                }
                Ok(())
            }
        })
    }
}

pub struct MpscReceiver<T> {
    inner: Arc<Mutex<RefCell<MpscInner<T>>>>,
}

impl<T> MpscReceiver<T> {
    pub async fn receive(&self) -> Option<T> {
        MpscReceiverFuture {
            inner: self.inner.clone(),
        }
        .await
    }
}

struct MpscReceiverFuture<T> {
    inner: Arc<Mutex<RefCell<MpscInner<T>>>>,
}

impl<T> Future for MpscReceiverFuture<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        critical_section::with(|cs| {
            let mut inner_lock = self.inner.borrow(cs).borrow_mut();
            if let Some(value) = inner_lock.data.pop_front() {
                Poll::Ready(Some(value))
            } else if inner_lock.is_closed {
                Poll::Ready(None)
            } else {
                inner_lock.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        })
    }
}
