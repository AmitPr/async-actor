//! This crate provides a simple, runtime-agnostic, actor framework, aimed
//! to be a minimal framework that gets out of your way.
//!
//! ```rust
//! use async_oneshot_channel::oneshot;
//! use async_actor::{Actor, ActorRef, WeakActorRef};
//!
//! struct CounterActor(usize);
//!
//! impl Actor for CounterActor {
//!     type Error = ();
//!     type Message = usize;
//!
//!     async fn on_msg(
//!         &mut self,
//!         _: &WeakActorRef<Self>,
//!         msg: Self::Message,
//!     ) -> Result<(), Self::Error> {
//!         self.0 += msg;
//!         println!("Received message: {}. Current state: {}", msg, self.0);
//!
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let actor = CounterActor(0);
//!     let (actor_ref, fut) = actor.into_future(None);
//!     let handle = tokio::spawn(fut);
//!
//!     actor_ref.send(3).await.unwrap();
//!     actor_ref.send(7).await.unwrap();
//!
//!     actor_ref.stop(0).unwrap();
//!     let res = handle.await;
//!     assert!(res.is_ok());
//! }
//! ```

mod actor;
mod actor_ref;
mod actor_run;
mod mailbox;

pub use actor::*;
pub use actor_ref::*;
pub use actor_run::*;
pub use mailbox::Mailbox;

#[cfg(test)]
mod test {
    use super::*;

    struct MyActor(usize);

    impl Actor for MyActor {
        type Error = ();
        type Message = usize;

        async fn on_msg(
            &mut self,
            _: &WeakActorRef<Self>,
            msg: Self::Message,
        ) -> Result<(), Self::Error> {
            self.0 += msg;
            println!("Received message: {}. Current state: {}", msg, self.0);

            Ok(())
        }
    }

    #[tokio::test]
    async fn test_actor() {
        let actor = MyActor(0);
        let (actor_ref, fut) = actor.into_future(None);
        let handle = tokio::spawn(fut);

        actor_ref.send(3).await.unwrap();
        actor_ref.send(7).await.unwrap();

        actor_ref.stop(0).unwrap();

        let res = handle.await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_actor_long() {
        let actor = MyActor(0);
        let (actor_ref, fut) = actor.into_future(None);
        let handle = tokio::spawn(fut);

        for i in 0..1000 {
            actor_ref.send(i).await.unwrap();
        }

        actor_ref.stop(0).unwrap();

        let res = handle.await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_drop() {
        let actor = MyActor(0);
        let (actor_ref, fut) = actor.into_future(None);
        let handle = tokio::spawn(fut);

        actor_ref.send(3).await.unwrap();
        actor_ref.send(7).await.unwrap();

        drop(actor_ref);

        let res = handle.await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_drop_partial() {
        let actor = MyActor(0);
        let (actor_ref, fut) = actor.into_future(None);
        let handle = tokio::spawn(fut);

        let ActorRef { sender, stop } = actor_ref;
        sender.send(3).await.unwrap();
        sender.send(7).await.unwrap();

        drop(stop);

        let res = handle.await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_drop_partial_sender() {
        let actor = MyActor(0);
        let (actor_ref, fut) = actor.into_future(None);
        let handle = tokio::spawn(fut);

        let ActorRef { sender, .. } = actor_ref;
        sender.send(3).await.unwrap();
        sender.send(7).await.unwrap();

        drop(sender);

        let res = handle.await;
        assert!(res.is_ok());
    }

    struct PlusOneActor;

    #[derive(Debug)]
    enum PlusOneActorMessage {
        PlusOne(usize, async_oneshot_channel::Sender<usize>),
        Stop,
    }

    impl Actor for PlusOneActor {
        type Message = PlusOneActorMessage;
        type Error = ();

        async fn on_msg(
            &mut self,
            _: &WeakActorRef<Self>,
            msg: Self::Message,
        ) -> Result<(), Self::Error> {
            match msg {
                PlusOneActorMessage::PlusOne(num, reply) => {
                    let _ = reply.send(num + 1);
                    Ok(())
                }
                _ => Err(()),
            }
        }
    }

    #[tokio::test]
    async fn test_reply() {
        let actor = PlusOneActor;
        let (actor_ref, fut) = actor.into_future(None);
        let handle = tokio::spawn(fut);

        let (reply_sender, reply_receiver) = async_oneshot_channel::oneshot();
        actor_ref
            .send(PlusOneActorMessage::PlusOne(3, reply_sender))
            .await
            .unwrap();
        let res = reply_receiver.recv().await;
        assert_eq!(res, Some(4));

        let (reply_sender, reply_receiver) = async_oneshot_channel::oneshot();
        actor_ref
            .send(PlusOneActorMessage::PlusOne(7, reply_sender))
            .await
            .unwrap();
        let res = reply_receiver.recv().await;
        assert_eq!(res, Some(8));

        actor_ref.stop(PlusOneActorMessage::Stop).unwrap();

        let res = handle.await;
        assert!(res.is_ok());
    }

    struct PingActor(ActorRef<PongActor>);

    #[derive(Debug)]
    enum PingActorMessage {
        Ping(usize),
        Stop,
    }

    impl Actor for PingActor {
        type Message = PingActorMessage;
        type Error = ();

        async fn on_msg(
            &mut self,
            this: &WeakActorRef<Self>,
            msg: Self::Message,
        ) -> Result<(), Self::Error> {
            match msg {
                PingActorMessage::Ping(num) => {
                    println!("PingActor received Ping({})", num);
                    self.0.send(PongActorMessage::Pong(num)).await.unwrap();
                    Ok(())
                }
                PingActorMessage::Stop => {
                    println!("PingActor received Stop");
                    this.stop(PingActorMessage::Stop).unwrap();
                    Ok(())
                }
            }
        }
    }

    struct PongActor;

    #[derive(Debug)]
    enum PongActorMessage {
        Pong(usize),
        Stop,
    }

    impl Actor for PongActor {
        type Message = PongActorMessage;
        type Error = ();

        async fn on_msg(
            &mut self,
            this: &WeakActorRef<Self>,
            msg: Self::Message,
        ) -> Result<(), Self::Error> {
            match msg {
                PongActorMessage::Pong(num) => {
                    println!("PongActor received Pong({})", num);
                    let _ = this.stop(PongActorMessage::Stop);
                    Ok(())
                }
                PongActorMessage::Stop => {
                    println!("PongActor received Stop");
                    Ok(())
                }
            }
        }
    }

    #[tokio::test]
    async fn test_ping_pong() {
        let pong_actor = PongActor;
        let (pong_actor_ref, pong_fut) = pong_actor.into_future(None);
        let pong_handle = tokio::spawn(pong_fut);

        let ping_actor = PingActor(pong_actor_ref);
        let (ping_actor_ref, ping_fut) = ping_actor.into_future(None);
        let ping_handle = tokio::spawn(ping_fut);

        ping_actor_ref
            .send(PingActorMessage::Ping(3))
            .await
            .unwrap();
        ping_actor_ref
            .send(PingActorMessage::Ping(7))
            .await
            .unwrap();

        ping_actor_ref.stop(PingActorMessage::Stop).unwrap();

        let res = ping_handle.await;
        assert!(res.is_ok());

        let res = pong_handle.await;
        assert!(res.is_ok());
    }
}
