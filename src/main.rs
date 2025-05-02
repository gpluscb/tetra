#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

mod commands;
mod framework;
mod util;

use crate::commands::{Commands, State};
use crate::framework::ExecutableCommandService;
use crate::util::state_service::StateLayer;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use tokio_stream::{Stream, StreamExt as _, StreamMap};
use tower::{Layer, Service, ServiceExt};
use twilight_gateway::error::ReceiveMessageError;
use twilight_gateway::{Config, EventTypeFlags, Message, StreamExt as _, create_recommended};
use twilight_http::Client;
use twilight_http::response::DeserializeBodyError;
use twilight_model::application::interaction::Interaction;
use twilight_model::gateway::Intents;
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

#[derive(Debug, Error)]
pub enum TwilightError {
    #[error("Http error: {0}")]
    Http(#[from] twilight_http::Error),
    #[error("Deserialize error: {0}")]
    Model(#[from] DeserializeBodyError),
}

async fn handle_events(
    router: impl Service<
        Interaction,
        Response = (),
        Error = (),
        Future = impl Future<Output = Result<(), ()>> + Send,
    > + Clone
    + Send
    + 'static,
    state: &State,
    mut events: impl Stream<Item = Result<Message, ReceiveMessageError>> + Unpin,
) -> Result<(), TwilightError> {
    async fn assert_fully_processed(it: impl Future<Output = Result<(), ()>>) {
        _ = it.await;
    }

    while let Some(item) = events.next_event(EventTypeFlags::INTERACTION_CREATE).await {
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
        tokio::spawn(assert_fully_processed(async move {
            router.ready().await?.call(interaction).await
        }));
    }

    Ok(())
}

fn get_command_router(
    state: State,
) -> impl Service<
    Interaction,
    Response = (),
    Error = (),
    Future = impl Future<Output = Result<(), ()>> + Send,
> + Clone
+ Send
+ 'static {
    let service = ExecutableCommandService::<Commands>::new()
        .map_err(|err| println!("AWAWAWA THERE WAS AN ERROR :( {err}"));

    StateLayer::new(state).layer(service)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    _ = dotenv::dotenv();
    let token = std::env::var("DISCORD_TOKEN")?;
    let app_id: Id<ApplicationMarker> = std::env::var("APPLICATION_ID")?.parse()?;
    let client = Arc::new(Client::new(token.clone()));

    let config = Config::new(token, Intents::empty());
    let shards = create_recommended(&client, config, |_, builder| builder.build()).await?;
    let (senders, shards): (_, Vec<_>) = shards.map(|shard| (shard.sender(), shard)).unzip();
    let shard_stream: StreamMap<_, _> = shards
        .into_iter()
        .map(|shard| (shard.id(), shard))
        .collect();

    let state = State {
        client: client.clone(),
        app_id,
        shutdown: Arc::new(AtomicBool::new(false)),
        senders,
    };

    let interaction = client.interaction(app_id);
    Commands::update_commands(&interaction).await?;
    let router = get_command_router(state.clone());
    handle_events(router, &state, shard_stream.map(|(_, shard)| shard))
        .await
        .unwrap();

    Ok(())
}
