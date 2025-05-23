use crate::commands::TwilightError;
use crate::context::CommandContext;
use crate::framework::CommandHandler;
use thiserror::Error;
use tracing::instrument;
use twilight_gateway::error::ChannelError;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_util::builder::InteractionResponseDataBuilder;

#[derive(Debug, CreateCommand, CommandModel)]
#[command(name = "shutdown", desc = "Shut down the bot.")]
pub struct Command;

#[derive(Debug, Error)]
pub enum Error {
    // TODO: Prettier print this
    #[error("One or more channel errors occurred, shutdown might be incomplete: {0:?}")]
    Channel(Vec<ChannelError>),
    #[error("Error replying to interaction: {0}")]
    Reply(#[from] TwilightError),
}

impl CommandHandler for Command {
    type Context = CommandContext;
    type Response = ();
    type Error = Error;

    #[instrument(level = "info")]
    async fn handle(self, context: Self::Context) -> Result<Self::Response, Self::Error> {
        context.state.send_shutdown().map_err(Error::Channel)?;

        context
            .reply(
                InteractionResponseDataBuilder::new()
                    .content("Shutdown initiated.")
                    .build(),
            )
            .await
            .map_err(TwilightError::from)?;

        Ok(())
    }
}
