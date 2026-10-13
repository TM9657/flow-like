use core::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use alloc::sync::Arc;
use critical_section::Mutex;

use crate::infrastructure::error::DdsError;

pub fn notification() -> (NotificationSender, NotificationReceiver) {
    let inner = Arc::new(Mutex::new(RefCell::new(NotificationInner {
        notified: false,
        waker: None,
        sender_count: 1,
    })));
    (
        NotificationSender {
            inner: inner.clone(),
        },
        NotificationReceiver { inner },
    )
}

struct NotificationInner {
    notified: bool,
    waker: Option<Waker>,
    sender_count: usize,
}

pub struct NotificationSender {
    inner: Arc<Mutex<RefCell<NotificationInner>>>,
}

impl Clone for NotificationSender {
    fn clone(&self) -> Self {
        critical_section::with(|cs| {
            self.inner.borrow(cs).borrow_mut().sender_count += 1;
        });
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl NotificationSender {
    pub fn notify(&self) {
        critical_section::with(|cs| {
            let mut inner_lock = self.inner.borrow(cs).borrow_mut();
            inner_lock.notified = true;
            if let Some(w) = inner_lock.waker.take() {
                w.wake()
            }
        })
    }
}

impl Drop for NotificationSender {
    fn drop(&mut self) {
        critical_section::with(|cs| {
            let mut inner_lock = self.inner.borrow(cs).borrow_mut();
            inner_lock.sender_count -= 1;
            if inner_lock.sender_count == 0 {
                // When the last sender is dropped wake the waiting task
                // so that it can finish knowing it won't get any message
                if let Some(w) = inner_lock.waker.take() {
                    w.wake()
                }
            }
        })
    }
}

pub struct NotificationReceiver {
    inner: Arc<Mutex<RefCell<NotificationInner>>>,
}

impl Future for NotificationReceiver {
    type Output = Result<(), DdsError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        critical_section::with(|cs| {
            let mut inner_lock = self.inner.borrow(cs).borrow_mut();
            if inner_lock.notified {
                inner_lock.notified = false;
                Poll::Ready(Ok(()))
            } else if inner_lock.sender_count == 0 {
                Poll::Ready(Err(DdsError::AlreadyDeleted))
            } else {
                inner_lock.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        })
    }
}
