use super::{CommandHandler, State};
use crate::TwilightError;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::interaction::Interaction;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

#[derive(CreateCommand, CommandModel)]
#[command(name = "test-command-2", desc = "Just a test command tbh tbh.")]
pub struct Command {
    /// The message to send
    message: String,
}

pub type Error = TwilightError;

impl CommandHandler for Command {
    type State = State;
    type Response = ();
    type Error = Error;

    async fn handle(
        self,
        state: Self::State,
        interaction: Interaction,
    ) -> Result<Self::Response, Self::Error> {
        state
            .client
            .interaction(state.app_id)
            .create_response(
                interaction.id,
                &interaction.token,
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
