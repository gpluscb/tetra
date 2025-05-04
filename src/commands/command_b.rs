use super::CommandHandler;
use crate::TwilightError;
use crate::context::CommandContext;
use tracing::instrument;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

#[derive(Debug, CreateCommand, CommandModel)]
#[command(name = "test-command-2", desc = "Just a test command tbh tbh.")]
pub struct Command {
    /// The message to send
    message: String,
}

pub type Error = TwilightError;

impl CommandHandler for Command {
    type Context = CommandContext;
    type Response = ();
    type Error = Error;

    #[instrument(level = "info")]
    async fn handle(self, context: Self::Context) -> Result<Self::Response, Self::Error> {
        context
            .state
            .client
            .interaction(context.state.app_id)
            .create_response(
                context.interaction.id,
                &context.interaction.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(
                        InteractionResponseDataBuilder::new()
                            .content(self.message)
                            .build(),
                    ),
                },
            )
            .await?;
        Ok(())
    }
}
