#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

mod commands;
mod framework;

use crate::commands::Commands;
use crate::framework::ExecutableCommandService;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use tower::{Service, ServiceExt};
use twilight_gateway::{
    Config, EventTypeFlags, MessageSender, Shard, StreamExt as _, create_recommended,
};
use twilight_http::Client;
use twilight_http::response::DeserializeBodyError;
use twilight_model::application::interaction::Interaction;
use twilight_model::gateway::Intents;
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

#[derive(Debug)]
pub struct State {
    pub client: Client,
    pub senders: Vec<MessageSender>,
    pub app_id: Id<ApplicationMarker>,
    pub shutdown: AtomicBool,
}

#[derive(Debug, Error)]
pub enum TwilightError {
    #[error("Http error: {0}")]
    Http(#[from] twilight_http::Error),
    #[error("Deserialize error: {0}")]
    Model(#[from] DeserializeBodyError),
}

async fn shard_runner(
    router: impl Service<
        (Arc<State>, Interaction),
        Response = (),
        Error = (),
        Future = impl Future<Output = Result<(), ()>> + Send,
    > + Clone
    + Send
    + 'static,
    state: Arc<State>,
    mut shard: Shard,
) {
    async fn assert_fully_processed(it: impl Future<Output = Result<(), ()>>) {
        _ = it.await;
    }

    while let Some(item) = shard.next_event(EventTypeFlags::INTERACTION_CREATE).await {
        let interaction = match item {
            Ok(Event::InteractionCreate(interaction_create)) => interaction_create.0,
            Ok(Event::GatewayClose(_)) if state.shutdown.load(Ordering::Acquire) => {
                // TODO: Some kind of timeout for shutdown
                println!("SHUTDOWNNNNN; Gateway Closed");
                break;
            }
            Err(e) => {
                println!("AWAWAWA RECEIVE MESSAGE ERROR {e}");
                continue;
            }
            _ => continue,
        };

        let mut router = router.clone();
        let state = state.clone();
        tokio::spawn(assert_fully_processed(async move {
            router.ready().await?.call((state, interaction)).await
        }));
    }
}

fn get_command_router() -> impl Service<
    (Arc<State>, Interaction),
    Response = (),
    Error = (),
    Future = impl Future<Output = Result<(), ()>> + Send,
> + Clone
+ Send
+ 'static {
    ExecutableCommandService::<Commands>::new()
        .map_err(|err| println!("AWAWAWA THERE WAS AN ERROR :( {err}"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    _ = dotenv::dotenv();
    let token = std::env::var("DISCORD_TOKEN")?;
    let app_id: Id<ApplicationMarker> = std::env::var("APPLICATION_ID")?.parse()?;
    let client = Client::new(token.clone());

    let config = Config::new(token, Intents::empty());
    let shards: Vec<_> = create_recommended(&client, config, |_, builder| builder.build())
        .await?
        .collect();
    let senders = shards.iter().map(Shard::sender).collect();

    let interaction = client.interaction(app_id);
    Commands::update_commands(&interaction).await?;

    let router = get_command_router();
    let state = Arc::new(State {
        client,
        app_id,
        shutdown: AtomicBool::new(false),
        senders,
    });
    let runners: Vec<_> = shards
        .into_iter()
        .map(|shard| {
            let router = router.clone();
            let state = state.clone();
            tokio::spawn(shard_runner(router, state, shard))
        })
        .collect();

    for runner in runners {
        // TODO: Allow other runners to do their thing even if singular runner failed
        runner.await?;
    }

    Ok(())
}
