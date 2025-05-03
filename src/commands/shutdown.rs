use crate::TwilightError;
use crate::context::CommandContext;
use crate::framework::CommandHandler;
use thiserror::Error;
use twilight_gateway::error::ChannelError;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

#[derive(CreateCommand, CommandModel)]
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

    async fn handle(self, context: Self::Context) -> Result<Self::Response, Self::Error> {
        context.state.send_shutdown().map_err(Error::Channel)?;

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
                            .content("Shutdown initiated.")
                            .build(),
                    ),
                },
            )
            .await
            .map_err(TwilightError::from)?;

        Ok(())
    }
}
