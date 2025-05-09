use crate::framework::CommandContextFactory;
use crate::util::OmitDebug;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, instrument};
use twilight_gateway::MessageSender;
use twilight_gateway::error::ChannelError;
use twilight_http::client::InteractionClient;
use twilight_http::response::marker::EmptyBody;
use twilight_http::{Client, Response};
use twilight_model::application::interaction::Interaction;
use twilight_model::gateway::CloseFrame;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

pub struct State {
    pub client: Client,
    pub senders: Vec<MessageSender>,
    pub app_id: Id<ApplicationMarker>,
    pub shutdown: AtomicBool,
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("client", &OmitDebug)
            .field("senders", &OmitDebug)
            .field("app_id", &self.app_id)
            .field("shutdown", &self.shutdown)
            .finish()
    }
}

impl State {
    pub fn interaction_client(&self) -> InteractionClient<'_> {
        self.client.interaction(self.app_id)
    }

    #[instrument]
    pub fn send_shutdown(&self) -> Result<(), Vec<ChannelError>> {
        // Shutdown method should be idempotent
        if self.shutdown.swap(true, Ordering::AcqRel) {
            info!("Attempted to send shutdown a second time");
            return Ok(());
        }

        let close_errors: Vec<_> = self
            .senders
            .iter()
            .map(|sender| sender.close(CloseFrame::NORMAL))
            .filter_map(Result::err)
            .collect();

        if close_errors.is_empty() {
            Ok(())
        } else {
            Err(close_errors)
        }
    }
}

#[derive(Clone, Debug)]
pub struct ContextFactory {
    pub state: Arc<State>,
}

impl ContextFactory {
    pub fn new(state: Arc<State>) -> Self {
        ContextFactory { state }
    }
}

impl CommandContextFactory for ContextFactory {
    type CommandContext = CommandContext;

    #[instrument(level = "trace")]
    fn create_context(self, interaction: Interaction) -> Self::CommandContext {
        CommandContext {
            state: self.state,
            interaction,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommandContext {
    pub state: Arc<State>,
    pub interaction: Interaction,
}

impl CommandContext {
    pub async fn reply(
        &self,
        response: InteractionResponseData,
    ) -> Result<Response<EmptyBody>, twilight_http::Error> {
        self.state
            .interaction_client()
            .create_response(
                self.interaction.id,
                &self.interaction.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(response),
                },
            )
            .await
    }
}
