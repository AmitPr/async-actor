use crate::Actor;

use async_channel::{Sender as MultiSender, WeakSender as WeakMultiSender};
use async_oneshot_channel::{Sender as OneshotSender, WeakSender as WeakOneshotSender};

#[derive(Debug)]
/// A handle to an actor, that allows messages to be sent to the actor.
///
/// As long as one ActorRef exists, the actor will continue to run.
pub struct ActorRef<A: Actor> {
    pub(crate) sender: MultiSender<A::Message>,
    pub(crate) stop: OneshotSender<A::Message>,
}

impl<A: Actor> ActorRef<A> {
    /// Sends a message to the actor. If the mailbox is full, the message will be returned in [`Err`].
    pub async fn send(&self, msg: A::Message) -> Result<(), A::Message> {
        self.sender.send(msg).await.map_err(|e| e.0)
    }

    /// Stops the actor by sending a stop message to it. If a stop message has already been sent,
    /// the stop message will be returned in [`Err`].
    pub fn stop(&self, stop: A::Message) -> Result<(), A::Message> {
        self.stop.send(stop)
    }

    /// Creates a [`WeakActorRef`] from this [`ActorRef`], which can be used as a handle to the actor that
    /// doesn't keep the actor alive, if it is the last handle to the actor.
    pub fn downgrade(&self) -> WeakActorRef<A> {
        WeakActorRef {
            sender: self.sender.downgrade(),
            stop: self.stop.downgrade(),
        }
    }
}

impl<A: Actor> Clone for ActorRef<A> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            stop: self.stop.clone(),
        }
    }
}

#[derive(Debug)]
/// A reference to an actor that allows messages to be sent to the actor.
///
/// If the actor has been dropped, this [`WeakActorRef`] will not be able to send messages to the actor,
/// and will not be able to be upgraded.
pub struct WeakActorRef<A: Actor> {
    sender: WeakMultiSender<A::Message>,
    stop: WeakOneshotSender<A::Message>,
}

impl<A: Actor> WeakActorRef<A> {
    /// Attempts to upgrade this [`WeakActorRef`] to an [`ActorRef`]. If the actor has been dropped,
    /// this will return [`None`].
    pub fn upgrade(&self) -> Option<ActorRef<A>> {
        Some(ActorRef {
            sender: self.sender.upgrade()?,
            stop: self.stop.upgrade()?,
        })
    }

    /// Sends a message to the actor. If the actor has been dropped, or the mailbox is full,
    /// the message will be returned in [`Err`].
    pub async fn send(&self, msg: A::Message) -> Result<(), A::Message> {
        match self.upgrade() {
            Some(actor_ref) => actor_ref.send(msg).await,
            None => Err(msg),
        }
    }

    /// Stops the actor by sending a stop message to it. If the actor has been dropped, or the mailbox is full,
    /// the stop message will be returned in [`Err`].
    pub fn stop(&self, stop: A::Message) -> Result<(), A::Message> {
        self.stop.send(stop)
    }
}

impl<A: Actor> Clone for WeakActorRef<A> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            stop: self.stop.clone(),
        }
    }
}

impl<A: Actor> TryInto<ActorRef<A>> for WeakActorRef<A> {
    type Error = ();

    fn try_into(self) -> Result<ActorRef<A>, Self::Error> {
        self.upgrade().ok_or(())
    }
}
