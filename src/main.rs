#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

mod commands;
mod context;
mod framework;

use crate::commands::Commands;
use crate::context::{ContextFactory, State};
use crate::framework::{CommandContextFactory, ExecutableCommandService};
use context::CommandContext;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use tokio::signal;
use tower::{Service, ServiceExt};
use twilight_gateway::{Config, EventTypeFlags, Shard, StreamExt as _, create_recommended};
use twilight_http::Client;
use twilight_http::response::DeserializeBodyError;
use twilight_model::application::interaction::Interaction;
use twilight_model::gateway::Intents;
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::{ApplicationMarker, GuildMarker};

#[derive(Debug, Error)]
pub enum TwilightError {
    #[error("Http error: {0}")]
    Http(#[from] twilight_http::Error),
    #[error("Deserialize error: {0}")]
    Model(#[from] DeserializeBodyError),
}

async fn shard_runner(
    router: impl Service<
        (ContextFactory, Interaction),
        Response = (),
        Error = (),
        Future = impl Future<Output = Result<(), ()>> + Send,
    > + Clone
    + Send
    + 'static,
    context_factory: ContextFactory,
    mut shard: Shard,
) {
    fn assert_fully_processed<Fut: Future<Output = Result<(), ()>>>(it: Fut) -> Fut {
        it
    }

    while let Some(item) = shard.next_event(EventTypeFlags::INTERACTION_CREATE).await {
        let interaction = match item {
            Ok(Event::InteractionCreate(interaction_create)) => interaction_create.0,
            Ok(Event::GatewayClose(_))
                if context_factory.state.shutdown.load(Ordering::Acquire) =>
            {
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
        let context_factory = context_factory.clone();
        // TODO: Commands probably need to be abortable? Right now they'd be just cut off when the application exits
        tokio::spawn(assert_fully_processed(async move {
            router
                .ready()
                .await?
                .call((context_factory, interaction))
                .await
        }));
    }
}

fn get_command_router<TContextFactory>() -> impl Service<
    (TContextFactory, Interaction),
    Response = (),
    Error = (),
    Future = impl Future<Output = Result<(), ()>> + Send,
> + Clone
+ Send
+ 'static
where
    TContextFactory: CommandContextFactory<CommandContext = CommandContext> + Send + 'static,
{
    let service = ExecutableCommandService::<Commands>::new();
    // UFCS because the type hint for TContextFactory is required and other constructs require nightly
    <ExecutableCommandService<_> as ServiceExt<(TContextFactory, _)>>::map_err(service, |err| {
        println!("AWAWAWA THERE WAS AN ERROR :( {err}");
    })
}

async fn ctrl_c_handler(state: &State) {
    // TODO: Log these errors
    _ = signal::ctrl_c().await;
    _ = state.send_shutdown();
}

// TODO: This should probably return () after proper tracing is set up
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    _ = dotenv::dotenv();
    let token = std::env::var("DISCORD_TOKEN")?;
    let app_id: Id<ApplicationMarker> = std::env::var("APPLICATION_ID")?.parse()?;
    let admin_guild_id: Id<GuildMarker> = std::env::var("ADMIN_GUILD_ID")?.parse()?;

    let client = Client::new(token.clone());

    let config = Config::new(token, Intents::empty());
    let shards: Vec<_> = create_recommended(&client, config, |_, builder| builder.build())
        .await?
        .collect();
    let senders: Vec<_> = shards.iter().map(Shard::sender).collect();

    let interaction = client.interaction(app_id);
    Commands::update_commands(&interaction, admin_guild_id).await?;

    let router = get_command_router();
    let state = Arc::new(State {
        client,
        senders: senders.clone(),
        app_id,
        shutdown: AtomicBool::new(false),
    });
    let runners: Vec<_> = shards
        .into_iter()
        .map(|shard| {
            let router = router.clone();
            let context_factory = ContextFactory::new(state.clone());
            tokio::spawn(shard_runner(router, context_factory, shard))
        })
        .collect();

    tokio::spawn(async move {
        ctrl_c_handler(&state).await;
    });

    for runner in runners {
        if let Err(e) = runner.await {
            println!("AWAWAWA A RUNNER EXITED UNEXPECTEDLY :( {e}");
        }
    }

    Ok(())
}
