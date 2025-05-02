use super::CommandHandler;
use crate::{State, TwilightError};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::interaction::Interaction;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

#[derive(CreateCommand, CommandModel)]
#[command(name = "test-command", desc = "Just a test command tbh tbh.")]
pub struct Command;

pub type Error = TwilightError;

impl CommandHandler for Command {
    type State = State;
    type Response = ();
    type Error = TwilightError;

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
                            .content("HIIIII OMG HAII UWU UWU")
                            .build(),
                    ),
                },
            )
            .await?;
        Ok(())
    }
}
