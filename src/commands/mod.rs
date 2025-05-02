use crate::TwilightError;
use crate::framework::{CommandHandler, FromCommandData, FromCommandDataError};
use command_a::CommandA;
use command_b::CommandB;
use std::sync::Arc;
use twilight_http::Client;
use twilight_interactions::command::{ApplicationCommandData, CommandModel, CreateCommand};
use twilight_model::application::interaction::Interaction;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

pub mod command_a;
pub mod command_b;

#[derive(Clone, Debug)]
pub struct State {
    pub client: Arc<Client>,
    pub app_id: Id<ApplicationMarker>,
}

pub enum Commands {
    A(CommandA),
    B(CommandB),
}

impl FromCommandData for Commands {
    fn from_command_data(data: Box<CommandData>) -> Result<Self, FromCommandDataError> {
        match &*data.name {
            CommandA::NAME => Ok(Commands::A(CommandA::from_interaction((*data).into())?)),
            CommandB::NAME => Ok(Commands::B(CommandB::from_interaction((*data).into())?)),
            _ => Err(FromCommandDataError::UnknownCommand(data)),
        }
    }
}

impl CommandHandler for Commands {
    type State = State;
    type Response = ();
    type Error = TwilightError;

    async fn handle(
        self,
        state: Self::State,
        interaction: Interaction,
    ) -> Result<Self::Response, Self::Error> {
        match self {
            Commands::A(a) => a.handle(state, interaction).await,
            Commands::B(b) => b.handle(state, interaction).await,
        }
    }
}

impl Commands {
    pub fn create_commands() -> [ApplicationCommandData; 2] {
        [CommandA::create_command(), CommandB::create_command()]
    }
}
