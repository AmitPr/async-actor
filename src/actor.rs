use either::Either;
use std::future::Future;

use crate::{ActorRef, ActorRun, Mailbox, WeakActorRef};

pub trait Actor: Send + Sized + 'static {
    type Error: Send;
    type Message: Send;

    #[allow(unused_variables)]
    /// Called when the actor is started, and before any messages are processed.
    /// Useful for initialization/setup logic that should run in the same async
    /// context as the actor, and not in the Actor struct's constructor.
    ///
    /// `this` is a [`WeakActorRef`], which can be used to send
    /// messages to itself, or to stop itself. As this is a weak reference, the
    /// actor may have already been dropped by external code, so it may not be
    /// possible to use the weak reference.
    fn on_start(
        &mut self,
        this: &WeakActorRef<Self>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }

    /// Called when a message is received by the actor. This is where the
    /// actor's main logic should be implemented.
    ///
    /// `this` is a [`WeakActorRef`], which can be used to send
    /// messages to itself, or to stop itself. As this is a weak reference, the
    /// actor may have already been dropped by external code, so it may not be
    /// possible to use the weak reference.
    fn on_msg(
        &mut self,
        this: &WeakActorRef<Self>,
        msg: Self::Message,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    #[allow(unused_variables)]
    /// Called when the actor is stopped. This is the place to perform any
    /// cleanup logic, such as closing connections, etc. This method is called
    /// after the actor has received a stop message, or after the last [`ActorRef`]
    /// to the actor is dropped.
    ///
    /// If the actor is stopped by dropping the last [`ActorRef`] to it, `stop` is [`None`].
    /// Otherwise, `stop` contains the stop message that was sent to the actor.
    fn on_stop(
        &mut self,
        stop: Option<Self::Message>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }

    /// Runs the actor with the given mailbox. Unless you have a specific reason to,
    /// the default implementation of this method should be used. You should not need to
    /// call this method directly either, see [`Actor::into_future`] instead.
    fn run_with(
        &mut self,
        mailbox: Mailbox<Self>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let this = mailbox.this.clone();
            self.on_start(&this).await?;

            loop {
                match mailbox.recv().await {
                    Either::Left(stop) => {
                        mailbox.receiver.close();
                        // Consume all remaining messages in the mailbox
                        while let Ok(msg) = mailbox.receiver.recv().await {
                            self.on_msg(&this, msg).await?;
                        }
                        self.on_stop(stop).await?;
                        break Ok(());
                    }
                    Either::Right(msg) => {
                        if let Some(msg) = msg {
                            self.on_msg(&this, msg).await?;
                        } else {
                            self.on_stop(None).await?;
                            break Ok(());
                        }
                    }
                }
            }
        }
    }

    /// Creates a future that runs the actor, and returns an [`ActorRef`] to the actor.
    ///
    /// `mailbox_size` is the size of the mailbox used by the actor. If `None`, the mailbox
    /// will be unbounded.
    fn into_future(self, mailbox_size: Option<usize>) -> (ActorRef<Self>, ActorRun<Self>) {
        ActorRun::new(self, mailbox_size)
    }
}
