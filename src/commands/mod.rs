use crate::framework::{CommandHandler, FromCommandData, FromCommandDataError};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use thiserror::Error;
use twilight_gateway::MessageSender;
use twilight_http::Client;
use twilight_http::client::InteractionClient;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::command::Command;
use twilight_model::application::interaction::Interaction;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

mod command_a;
mod command_b;
mod shutdown;

#[derive(Clone, Debug)]
pub struct State {
    pub client: Arc<Client>,
    pub senders: Vec<MessageSender>,
    pub app_id: Id<ApplicationMarker>,
    pub shutdown: Arc<AtomicBool>,
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("command_a command error: {0}")]
    A(command_a::Error),
    #[error("command_b command error: {0}")]
    B(command_b::Error),
    #[error("shutdown command error: {0}")]
    Shutdown(shutdown::Error),
}

pub enum Commands {
    A(command_a::Command),
    B(command_b::Command),
    Shutdown(shutdown::Command),
}

impl FromCommandData for Commands {
    fn from_command_data(data: Box<CommandData>) -> Result<Self, FromCommandDataError> {
        match &*data.name {
            command_a::Command::NAME => Ok(Commands::A(command_a::Command::from_interaction(
                (*data).into(),
            )?)),
            command_b::Command::NAME => Ok(Commands::B(command_b::Command::from_interaction(
                (*data).into(),
            )?)),
            shutdown::Command::NAME => Ok(Commands::Shutdown(shutdown::Command::from_interaction(
                (*data).into(),
            )?)),
            _ => Err(FromCommandDataError::UnknownCommand(data)),
        }
    }
}

impl CommandHandler for Commands {
    type State = State;
    type Response = ();
    type Error = CommandError;

    async fn handle(
        self,
        state: Self::State,
        interaction: Interaction,
    ) -> Result<Self::Response, Self::Error> {
        match self {
            Commands::A(command) => command
                .handle(state, interaction)
                .await
                .map_err(CommandError::A),
            Commands::B(command) => command
                .handle(state, interaction)
                .await
                .map_err(CommandError::B),
            Commands::Shutdown(command) => command
                .handle(state, interaction)
                .await
                .map_err(CommandError::Shutdown),
        }
    }
}

impl Commands {
    fn global_commands() -> [Command; 2] {
        [
            command_a::Command::create_command().into(),
            command_b::Command::create_command().into(),
        ]
    }

    fn admin_commands() -> [Command; 1] {
        [shutdown::Command::create_command().into()]
    }

    pub async fn update_commands(
        client: &InteractionClient<'_>,
    ) -> Result<(), twilight_http::Error> {
        let global_commands = Self::global_commands();
        client.set_global_commands(&global_commands).await?;

        let admin_commands = Self::admin_commands();
        // TODO: .env
        client
            .set_guild_commands(Id::new(152109320375369728), &admin_commands)
            .await?;
        Ok(())
    }
}
