use crate::framework::CommandContextFactory;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, instrument};
use twilight_gateway::MessageSender;
use twilight_gateway::error::ChannelError;
use twilight_http::Client;
use twilight_model::application::interaction::Interaction;
use twilight_model::gateway::CloseFrame;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

#[derive(Debug)]
pub struct State {
    pub client: Client,
    pub senders: Vec<MessageSender>,
    pub app_id: Id<ApplicationMarker>,
    pub shutdown: AtomicBool,
}

impl State {
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
