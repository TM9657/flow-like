// Modified for Flow-Like: route each RPC response to its own caller so cancellation
// cannot deliver a late response to a subsequent request.
use alloc::sync::Arc;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

use crate::{
    dcps::{
        channels::oneshot::{OneshotSender, oneshot},
        dcps_mail::{DcpsMail, DcpsReply},
    },
    infrastructure::error::DdsResult,
};

/// Mailbox owned by the participant factory actor loop to receive requests and send responses.
pub struct RpcMailbox {
    request_channel: Channel<CriticalSectionRawMutex, (DcpsMail, OneshotSender<DcpsReply>), 1>,
}

impl RpcMailbox {
    /// Creates a new RPC mailbox.
    pub fn new() -> Self {
        Self {
            request_channel: Channel::new(),
        }
    }

    /// Receives a request and its caller's response sender.
    pub async fn receive_request(&self) -> (DcpsMail, OneshotSender<DcpsReply>) {
        self.request_channel.receive().await
    }
}

impl Default for RpcMailbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Client handle held by async entities to perform request-response RPC calls with the backend.
#[derive(Clone)]
pub struct RpcClient {
    mailbox: Arc<RpcMailbox>,
}

impl RpcClient {
    /// Creates a new RPC client wrapping a shared mailbox.
    pub fn new(mailbox: Arc<RpcMailbox>) -> Self {
        Self { mailbox }
    }

    /// Sends an RPC request to the backend actor and awaits its response.
    pub async fn call(&self, mail: DcpsMail) -> DdsResult<DcpsReply> {
        let (reply_sender, reply_receiver) = oneshot();
        self.mailbox
            .request_channel
            .send((mail, reply_sender))
            .await;
        reply_receiver.await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dcps::dcps_mail::ParticipantFactoryMail;
    use alloc::boxed::Box;
    use core::{
        future::Future,
        pin::{Pin, pin},
        task::{Context, Poll, Waker},
    };

    fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
        future.poll(&mut Context::from_waker(Waker::noop()))
    }

    fn request() -> DcpsMail {
        DcpsMail::ParticipantFactory(ParticipantFactoryMail::LookupParticipant { domain_id: 0 })
    }

    fn receive(mailbox: &RpcMailbox) -> OneshotSender<DcpsReply> {
        match poll_once(pin!(mailbox.receive_request())) {
            Poll::Ready((_, reply_sender)) => reply_sender,
            Poll::Pending => panic!("request must already be queued"),
        }
    }

    #[test]
    fn cancelled_call_cannot_deliver_late_reply_to_next_call() {
        let mailbox = Arc::new(RpcMailbox::new());
        let client = RpcClient::new(mailbox.clone());
        let mut cancelled = Box::pin(client.call(request()));
        assert!(poll_once(cancelled.as_mut()).is_pending());
        let cancelled_reply = receive(&mailbox);

        // A timeout drops the request future while the actor is still working.
        drop(cancelled);
        let mut next = pin!(client.call(request()));
        assert!(poll_once(next.as_mut()).is_pending());

        cancelled_reply.send(DcpsReply::Ok(Ok(())));
        assert!(poll_once(next.as_mut()).is_pending());
        receive(&mailbox).send(DcpsReply::OptionInstanceHandle(Ok(None)));
        assert!(matches!(
            poll_once(next.as_mut()),
            Poll::Ready(Ok(DcpsReply::OptionInstanceHandle(Ok(None))))
        ));
    }

    #[test]
    fn abandoned_queued_request_does_not_block_later_calls() {
        let mailbox = Arc::new(RpcMailbox::new());
        let client = RpcClient::new(mailbox.clone());
        let mut cancelled = Box::pin(client.call(request()));
        assert!(poll_once(cancelled.as_mut()).is_pending());
        drop(cancelled);

        let mut next = pin!(client.call(request()));
        assert!(poll_once(next.as_mut()).is_pending());
        receive(&mailbox).send(DcpsReply::Ok(Ok(())));
        assert!(poll_once(next.as_mut()).is_pending());
        receive(&mailbox).send(DcpsReply::OptionInstanceHandle(Ok(None)));
        assert!(matches!(
            poll_once(next.as_mut()),
            Poll::Ready(Ok(DcpsReply::OptionInstanceHandle(Ok(None))))
        ));
    }

    #[test]
    fn concurrent_calls_receive_their_own_replies() {
        let mailbox = Arc::new(RpcMailbox::new());
        let client = RpcClient::new(mailbox.clone());
        let mut first = pin!(client.call(request()));
        assert!(poll_once(first.as_mut()).is_pending());
        let first_reply = receive(&mailbox);
        let mut second = pin!(client.call(request()));
        assert!(poll_once(second.as_mut()).is_pending());
        let second_reply = receive(&mailbox);

        second_reply.send(DcpsReply::OptionInstanceHandle(Ok(None)));
        assert!(poll_once(first.as_mut()).is_pending());
        assert!(matches!(
            poll_once(second.as_mut()),
            Poll::Ready(Ok(DcpsReply::OptionInstanceHandle(Ok(None))))
        ));
        first_reply.send(DcpsReply::Ok(Ok(())));
        assert!(matches!(
            poll_once(first.as_mut()),
            Poll::Ready(Ok(DcpsReply::Ok(Ok(()))))
        ));
    }
}
