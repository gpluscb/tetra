use super::CommandHandler;
use crate::TwilightError;
use crate::context::CommandContext;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::interaction::Interaction;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

#[derive(CreateCommand, CommandModel)]
#[command(name = "test-command", desc = "Just a test command tbh tbh.")]
pub struct Command;

pub type Error = TwilightError;

impl CommandHandler for Command {
    type Context = CommandContext;
    type Response = ();
    type Error = TwilightError;

    async fn handle(
        self,
        context: Self::Context,
        interaction: Interaction,
    ) -> Result<Self::Response, Self::Error> {
        context
            .state
            .client
            .interaction(context.state.app_id)
            .create_response(
                interaction.id,
                &interaction.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(
                        InteractionResponseDataBuilder::new()
                            .content("HIIIII OMG HAII UWU UWU")
                            .build(),
                    ),
                },
            )
            .await?;
        Ok(())
    }
}
