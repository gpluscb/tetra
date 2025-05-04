#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

mod commands;
mod context;
mod framework;
mod util;

use crate::commands::Commands;
use crate::context::{ContextFactory, State};
use crate::framework::{
    CommandContextFactory, CommandFromInteractionError, Error, ExecutableCommandService,
};
use context::CommandContext;
use serde::Deserialize;
use std::future::Future;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;
use tower::{Service, ServiceExt};
use tracing::{Instrument, debug, error, info_span, instrument, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use twilight_gateway::error::ReceiveMessageError;
use twilight_gateway::{Config, EventTypeFlags, Shard, StreamExt as _, create_recommended};
use twilight_http::Client;
use twilight_model::application::interaction::Interaction;
use twilight_model::gateway::Intents;
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::{ApplicationMarker, GuildMarker};

#[instrument(level = "info", fields(shard.id = %shard.id()), skip(router, shard))]
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
    while let Some(event) = shard.next_event(EventTypeFlags::INTERACTION_CREATE).await {
        if let ControlFlow::Break(()) =
            handle_event(router.clone(), context_factory.clone(), event).await
        {
            break;
        }
    }
}

#[instrument(level = "debug", skip(router))]
async fn handle_event(
    mut router: impl Service<
        (ContextFactory, Interaction),
        Response = (),
        Error = (),
        Future = impl Future<Output = Result<(), ()>> + Send,
    > + Clone
    + Send
    + 'static,
    context_factory: ContextFactory,
    event: Result<Event, ReceiveMessageError>,
) -> ControlFlow<()> {
    fn assert_fully_processed<Fut: Future<Output = Result<(), ()>>>(it: Fut) -> Fut {
        it
    }

    let interaction = match event {
        Ok(Event::InteractionCreate(interaction_create)) => interaction_create.0,
        Ok(Event::GatewayClose(close_frame))
            if context_factory.state.shutdown.load(Ordering::Acquire) =>
        {
            // TODO: Some kind of timeout for shutdown
            debug!(?close_frame, "GatewayClose after shutdown");
            return ControlFlow::Break(());
        }
        Err(error) => {
            warn!(%error, "Error receiving gateway event");
            return ControlFlow::Continue(());
        }
        _ => return ControlFlow::Continue(()),
    };

    // TODO: Commands probably need to be abortable? Right now they'd be just cut off when the application exits
    tokio::spawn(assert_fully_processed(
        async move {
            router
                .ready()
                .await?
                .call((context_factory, interaction))
                .await
        }
        .instrument(info_span!("command service execution")),
    ));

    ControlFlow::Continue(())
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
    <ExecutableCommandService<_> as ServiceExt<(TContextFactory, _)>>::map_err(service, |error| {
        match &error {
            Error::FromInteraction(CommandFromInteractionError::FromCommandData(_, _))
            | Error::Command(_) => error!(%error),
            Error::FromInteraction(CommandFromInteractionError::NotACommand(_, _)) => {
                debug!(%error);
            }
        }
    })
}

#[instrument]
async fn ctrl_c_handler(state: &State) {
    if let Err(error) = signal::ctrl_c().await {
        error!(%error, "Could not install ctrl-c handler, sending shutdown");
    }
    if let Err(error) = state.send_shutdown() {
        error!(?error, "Sending shutdown from ctrl-c handler failed");
    }
}

pub fn install_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "tetra=trace,twilight_gateway=debug,twilight_http=debug,twilight_model=debug,twilight_util=debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[derive(Deserialize)]
struct EnvConfig {
    pub discord_token: String,
    pub application_id: Id<ApplicationMarker>,
    pub admin_guild_id: Id<GuildMarker>,
}

// TODO: This should probably return () after proper tracing is set up
// TODO: Also break up this function also use envy
#[tokio::main]
#[instrument]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    _ = dotenv::dotenv();
    install_tracing();

    let config: EnvConfig = envy::from_env()
        .inspect_err(|error| error!(%error, "Error reading config from environment"))?;

    let client = Client::new(config.discord_token.clone());
    let interaction = client.interaction(config.application_id);
    Commands::update_commands(&interaction, config.admin_guild_id).await?;

    let shard_config = Config::new(config.discord_token, Intents::empty());
    let shards: Vec<_> = create_recommended(&client, shard_config, |_, builder| builder.build())
        .await?
        .collect();
    let senders: Vec<_> = shards.iter().map(Shard::sender).collect();

    let router = get_command_router();
    let state = Arc::new(State {
        client,
        senders: senders.clone(),
        app_id: config.application_id,
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
        if let Err(error) = runner.await {
            error!(%error, "Runner exited unexpectedly");
        }
    }

    Ok(())
}
