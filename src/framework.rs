use derive_where::derive_where;
use futures::future::BoxFuture;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;
use tower::Service;
use tracing::{Instrument, instrument, trace_span};
use twilight_interactions::command::CommandModel;
use twilight_interactions::error::ParseError;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};

#[derive(Clone, PartialEq, Debug, Error)]
pub enum Error<CommandError> {
    #[error("Creating command from Interaction failed: {0}")]
    FromInteraction(#[from] CommandFromInteractionError),
    #[error("Command error: {0}")]
    Command(CommandError),
}

#[derive(Clone, PartialEq, Debug, Error)]
pub enum CommandFromInteractionError {
    #[error("Interaction was not a command interaction")]
    NotACommand(Interaction),
    #[error("Interaction kind was ApplicationCommand, but no CommandData present")]
    NoCommandData(Interaction, Option<InteractionData>),
    #[error("Getting command from interaction failed: {1}")]
    FromCommandData(Interaction, FromCommandDataError),
}

#[derive(Clone, PartialEq, Debug, Error)]
pub enum FromCommandDataError {
    #[error("Not registered command received")]
    UnknownCommand(Box<CommandData>),
    #[error("Command parse error: {0}")]
    Parse(#[from] ParseError),
}

#[derive_where(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default, Debug, Hash)]
pub struct CommandService<TCommand>(PhantomData<TCommand>);

impl<TCommand> CommandService<TCommand> {
    pub fn new() -> Self {
        CommandService::default()
    }
}

impl<TCommand, TContext> Service<(TCommand, TContext)> for CommandService<TCommand>
where
    TCommand: CommandHandler<Context = TContext>,
{
    type Response = TCommand::Response;
    type Error = TCommand::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, (command, context): (TCommand, TContext)) -> Self::Future {
        Box::pin(
            command
                .handle(context)
                .instrument(trace_span!("command handler")),
        )
    }
}

#[derive_where(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default, Debug, Hash)]
pub struct ExecutableCommandService<TCommand>(PhantomData<TCommand>);

impl<TCommand> ExecutableCommandService<TCommand> {
    pub fn new() -> Self {
        ExecutableCommandService::default()
    }
}

impl<TCommand, TContextFactory> Service<(TContextFactory, Interaction)>
    for ExecutableCommandService<TCommand>
where
    TCommand: CommandRunner<TContextFactory>,
{
    type Response = TCommand::Response;
    type Error = Error<TCommand::CommandError>;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(
        &mut self,
        (context_factory, interaction): (TContextFactory, Interaction),
    ) -> Self::Future {
        Box::pin(
            TCommand::run(context_factory, interaction).instrument(trace_span!("command runner")),
        )
    }
}

pub trait FromCommandData: Sized {
    fn from_command_data(command_data: Box<CommandData>) -> Result<Self, FromCommandDataError>;
}

impl<T> FromCommandData for T
where
    T: CommandModel,
{
    #[tracing::instrument]
    fn from_command_data(command_data: Box<CommandData>) -> Result<Self, FromCommandDataError> {
        Self::from_interaction((*command_data).into()).map_err(FromCommandDataError::from)
    }
}

pub trait CommandHandler {
    type Context;
    type Response;
    type Error;

    fn handle(
        self,
        context: Self::Context,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;
}

pub trait CommandRunner<ContextFactory> {
    type Response;
    type CommandError;

    fn run(
        context_factory: ContextFactory,
        interaction: Interaction,
    ) -> impl Future<Output = Result<Self::Response, Error<Self::CommandError>>> + Send + 'static;
}

pub trait CommandContextFactory {
    type CommandContext;

    fn create_context(self, interaction: Interaction) -> Self::CommandContext;
}

impl<F, TCommandContext> CommandContextFactory for F
where
    F: FnOnce(Interaction) -> TCommandContext,
{
    type CommandContext = TCommandContext;

    #[instrument(level = "trace", skip(self))]
    fn create_context(self, interaction: Interaction) -> Self::CommandContext {
        self(interaction)
    }
}

impl<TCommand, ContextFactory> CommandRunner<ContextFactory> for TCommand
where
    TCommand: CommandHandler + FromCommandData,
    TCommand: Sized + 'static,
    TCommand::Context: Send,
    ContextFactory: CommandContextFactory<CommandContext = TCommand::Context> + Send + 'static,
{
    type Response = TCommand::Response;
    type CommandError = <TCommand as CommandHandler>::Error;

    #[tracing::instrument(level = "debug", skip(context_factory))]
    async fn run(
        context_factory: ContextFactory,
        mut interaction: Interaction,
    ) -> Result<Self::Response, Error<Self::CommandError>> {
        if !matches!(interaction.kind, InteractionType::ApplicationCommand) {
            return Err(Error::FromInteraction(
                CommandFromInteractionError::NotACommand(interaction),
            ));
        }

        let command_data = match interaction.data.take() {
            Some(InteractionData::ApplicationCommand(command_data)) => command_data,
            data => {
                return Err(Error::FromInteraction(
                    CommandFromInteractionError::NoCommandData(interaction, data),
                ));
            }
        };

        let command = match Self::from_command_data(command_data) {
            Ok(command) => command,
            Err(error) => {
                return Err(Error::FromInteraction(
                    CommandFromInteractionError::FromCommandData(interaction, error),
                ));
            }
        };

        let context = context_factory.create_context(interaction);
        command
            .handle(context)
            .instrument(trace_span!("command handler"))
            .await
            .map_err(Error::Command)
    }
}
