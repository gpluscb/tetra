use derive_where::derive_where;
use futures::future::BoxFuture;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;
use tower::Service;
use twilight_interactions::command::CommandModel;
use twilight_interactions::error::ParseError;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::{Interaction, InteractionData};

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
    NotACommand(Interaction, Option<InteractionData>),
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

impl<TCommand, TContext> Service<(TCommand, TContext, Interaction)> for CommandService<TCommand>
where
    TCommand: CommandHandler<Context = TContext>,
{
    type Response = TCommand::Response;
    type Error = TCommand::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(
        &mut self,
        (command, context, interaction): (TCommand, TContext, Interaction),
    ) -> Self::Future {
        Box::pin(command.handle(context, interaction))
    }
}

#[derive_where(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default, Debug, Hash)]
pub struct ExecutableCommandService<TCommand>(PhantomData<TCommand>);

impl<TCommand> ExecutableCommandService<TCommand> {
    pub fn new() -> Self {
        ExecutableCommandService::default()
    }
}

impl<TCommand, TContext> Service<(TContext, Interaction)> for ExecutableCommandService<TCommand>
where
    TCommand: CommandRunner<Context = TContext>,
{
    type Response = TCommand::Response;
    type Error = Error<TCommand::CommandError>;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, (context, interaction): (TContext, Interaction)) -> Self::Future {
        Box::pin(TCommand::run(context, interaction))
    }
}

pub trait FromCommandData: Sized {
    fn from_command_data(command_data: Box<CommandData>) -> Result<Self, FromCommandDataError>;
}

impl<T> FromCommandData for T
where
    T: CommandModel,
{
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
        interaction: Interaction,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;
}

pub trait CommandRunner {
    type Context;
    type Response;
    type CommandError;

    fn run(
        context: Self::Context,
        interaction: Interaction,
    ) -> impl Future<Output = Result<Self::Response, Error<Self::CommandError>>> + Send + 'static;
}

impl<TCommand> CommandRunner for TCommand
where
    TCommand: CommandHandler + FromCommandData,
    TCommand: Sized + 'static,
    TCommand::Context: Send,
{
    type Context = TCommand::Context;
    type Response = TCommand::Response;
    type CommandError = <TCommand as CommandHandler>::Error;

    async fn run(
        context: Self::Context,
        mut interaction: Interaction,
    ) -> Result<Self::Response, Error<Self::CommandError>> {
        let command_data = match interaction.data.take() {
            Some(InteractionData::ApplicationCommand(command_data)) => command_data,
            data => {
                return Err(Error::FromInteraction(
                    CommandFromInteractionError::NotACommand(interaction, data),
                ));
            }
        };

        let command = match Self::from_command_data(command_data) {
            Ok(command) => command,
            Err(err) => {
                return Err(Error::FromInteraction(
                    CommandFromInteractionError::FromCommandData(interaction, err),
                ));
            }
        };

        command
            .handle(context, interaction)
            .await
            .map_err(Error::Command)
    }
}
