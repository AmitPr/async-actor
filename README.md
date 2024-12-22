# async-actor: Simple actors for Rust

This crate provides a simple, runtime-agnostic, actor framework for Rust, aimed to be a minimal framework that gets out of your way.

This crate aims to follow Rust semantics whilst modeling actors: If the last strong reference to an actor is dropped, the actor is stopped. It does not provide any runtime and is not tied to any executor. The library consumer can decide how to run the actors, which are just Rust futures.

This crate intentionally has a minimal amount of dependencies.

```rust
use async_oneshot_channel::oneshot;
use async_actor::{Actor, ActorRef, WeakActorRef};

struct CounterActor(usize);

impl Actor for CounterActor {
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

#[tokio::main]
async fn main() {
    let actor = CounterActor(0);
    let (actor_ref, fut) = actor.into_future(None);
    let handle = tokio::spawn(fut);
    actor_ref.send(3).await.unwrap();
    actor_ref.send(7).await.unwrap();
    actor_ref.stop(0).unwrap();
    let res = handle.await;
    assert!(res.is_ok());
}
```
