use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use async_channel::Receiver as MultiReceiver;
use async_oneshot_channel::Receiver as OneshotReceiver;
use either::Either;

use crate::{Actor, ActorRef, WeakActorRef};

/// A mailbox for an actor, containing a receiver for messages, a receiver for stop messages,
/// and a weak reference to the actor.
///
/// Importantly, we do not store a strong [`ActorRef`] in the mailbox, as the actor would otherwise
/// keep itself alive even if all other references to it were dropped.
pub struct Mailbox<A: Actor> {
    pub receiver: MultiReceiver<A::Message>,
    pub stop: OneshotReceiver<A::Message>,
    pub this: WeakActorRef<A>,
}

impl<A: Actor> Mailbox<A> {
    pub fn new(size: Option<usize>) -> (Self, ActorRef<A>) {
        let (multi_sender, multi_receiver) = if let Some(size) = size {
            async_channel::bounded(size)
        } else {
            async_channel::unbounded()
        };
        let (stop_sender, stop_receiver) = async_oneshot_channel::oneshot();
        let actor_ref = ActorRef {
            sender: multi_sender,
            stop: stop_sender,
        };
        let mailbox = Self {
            receiver: multi_receiver,
            stop: stop_receiver,
            this: actor_ref.downgrade(),
        };
        (mailbox, actor_ref)
    }

    pub fn recv(
        &self,
    ) -> MailboxRecv<
        impl Future<Output = Option<A::Message>> + '_,
        impl Future<Output = Option<A::Message>> + '_,
    > {
        MailboxRecv {
            stop: self.stop.recv(),
            msg: async { self.receiver.recv().await.ok() },
        }
    }
}

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    /// Convenience future that polls both the stop and message receivers, prioritizing the stop receiver.
    pub struct MailboxRecv<F1, F2> {
        #[pin]
        stop: F1,
        #[pin]
        msg: F2,
    }
}

impl<T, U, F1, F2> Future for MailboxRecv<F1, F2>
where
    F1: Future<Output = T>,
    F2: Future<Output = U>,
{
    type Output = Either<T, U>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        if let Poll::Ready(t) = this.stop.poll(cx) {
            return Poll::Ready(Either::Left(t));
        }
        if let Poll::Ready(u) = this.msg.poll(cx) {
            return Poll::Ready(Either::Right(u));
        }
        Poll::Pending
    }
}
