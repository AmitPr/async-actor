use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Actor, ActorRef, Mailbox};

/// A future that drives an actor from start to completion.
/// Once awaited, it will run the actor, process all messages,
/// and eventually resolve with either the actor (on success) or an error.
pub struct ActorRun<A: Actor> {
    future: Pin<Box<dyn Future<Output = Result<A, A::Error>> + Send>>,
}

impl<A: Actor + Send + 'static> ActorRun<A> {
    /// Creates a new [`ActorRef`] and [`ActorRun`] future for `actor` with optional mailbox size.
    pub fn new(mut actor: A, mailbox_size: Option<usize>) -> (ActorRef<A>, Self) {
        let (mailbox, actor_ref) = Mailbox::new(mailbox_size);

        let future = Box::pin(async move {
            actor.run_with(mailbox).await?;
            Ok(actor)
        });

        (actor_ref, ActorRun { future })
    }
}

impl<A: Actor + Send + 'static> Future for ActorRun<A> {
    type Output = Result<A, A::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}
