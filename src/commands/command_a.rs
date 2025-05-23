use super::CommandHandler;
use crate::commands::TwilightError;
use crate::context::CommandContext;
use tracing::instrument;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_util::builder::InteractionResponseDataBuilder;

#[derive(Debug, CreateCommand, CommandModel)]
#[command(name = "test-command", desc = "Just a test command tbh tbh.")]
pub struct Command;

pub type Error = TwilightError;

impl CommandHandler for Command {
    type Context = CommandContext;
    type Response = ();
    type Error = TwilightError;

    #[instrument(level = "info")]
    async fn handle(self, context: Self::Context) -> Result<Self::Response, Self::Error> {
        context
            .reply(
                InteractionResponseDataBuilder::new()
                    .content("HIIIII OMG HAII UWU UWU")
                    .build(),
            )
            .await?;
        Ok(())
    }
}
