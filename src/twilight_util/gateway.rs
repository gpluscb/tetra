use crate::State;
use std::sync::atomic::Ordering;
use twilight_gateway::error::ChannelError;
use twilight_model::gateway::CloseFrame;

pub fn send_shutdown(state: &State) -> Result<(), Vec<ChannelError>> {
    // Shutdown method should be idempotent
    if state.shutdown.swap(true, Ordering::AcqRel) {
        return Ok(());
    }

    let close_errors: Vec<_> = state
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
