use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use twilight_gateway::MessageSender;
use twilight_gateway::error::ChannelError;
use twilight_http::Client;
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
    pub fn send_shutdown(&self) -> Result<(), Vec<ChannelError>> {
        // Shutdown method should be idempotent
        if self.shutdown.swap(true, Ordering::AcqRel) {
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
pub struct CommandContext {
    pub state: Arc<State>,
}
