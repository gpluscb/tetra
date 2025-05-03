use crate::framework::CommandHandler;
use crate::{State, TwilightError};
use std::sync::Arc;
use thiserror::Error;
use twilight_gateway::error::ChannelError;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::interaction::Interaction;
use twilight_model::gateway::CloseFrame;
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
    type State = Arc<State>;
    type Response = ();
    type Error = Error;

    async fn handle(
        self,
        state: Self::State,
        interaction: Interaction,
    ) -> Result<Self::Response, Self::Error> {
        let close_errors: Vec<_> = state
            .senders
            .iter()
            .map(|sender| sender.close(CloseFrame::NORMAL))
            .filter_map(Result::err)
            .collect();

        if !close_errors.is_empty() {
            return Err(Error::Channel(close_errors));
        }

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
