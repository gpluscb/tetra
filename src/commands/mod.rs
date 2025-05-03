use crate::context::CommandContext;
use crate::framework::{CommandHandler, FromCommandData, FromCommandDataError};
use std::error::Error;
use std::fmt::{Display, Formatter};
use twilight_http::client::InteractionClient;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::command::Command;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;

mod command_a;
mod command_b;
mod shutdown;

macro_rules! commands_collection {
    (Create collection $collection_name:ident
    with error type $error_name:ident
    with visibility $vis:vis
    with context $context:ty;
    from commands: {
        $($command_name:ident
        at $command_type:path;
        with error type $command_error_type:path,)*
    }) => {
        #[derive(Debug)]
        $vis enum $error_name {
            $($command_name($command_error_type),
            )*
        }

        impl Error for $error_name {}
        impl Display for $error_name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($error_name::$command_name(inner) => write!(f, "Command {} had error: {inner}", stringify!($command_name)),
                    )*
                }
            }
        }

        $vis enum $collection_name {
            $($command_name($command_type),
            )*
        }

        impl FromCommandData for $collection_name {
            fn from_command_data(data: Box<CommandData>) -> Result<Self, FromCommandDataError> {
                match &*data.name {
                    $(<$command_type>::NAME => Ok($collection_name::$command_name(<$command_type>::from_interaction(
                        (*data).into(),
                    )?)),
                    )*
                    _ => Err(FromCommandDataError::UnknownCommand(data)),
                }
            }
        }

        impl CommandHandler for $collection_name {
            type Context = $context;
            type Response = ();
            type Error = $error_name;

            async fn handle(
                self,
                context: Self::Context,
            ) -> Result<Self::Response, Self::Error> {
                match self {
                    $($collection_name::$command_name(command) => command
                        .handle(context)
                        .await
                        .map_err($error_name::$command_name),
                    )*
                }
            }
        }
    };
}
commands_collection! {
    Create collection Commands
    with error type CommandError
    with visibility pub
    with context CommandContext;
    from commands: {
        A at command_a::Command; with error type command_a::Error,
        B at command_b::Command; with error type command_b::Error,
        Shutdown at shutdown::Command; with error type shutdown::Error,
    }
}

impl Commands {
    fn global_commands() -> [Command; 2] {
        [
            command_a::Command::create_command().into(),
            command_b::Command::create_command().into(),
        ]
    }

    fn admin_guild_commands() -> [Command; 1] {
        [shutdown::Command::create_command().into()]
    }

    pub async fn update_commands(
        client: &InteractionClient<'_>,
        admin_guild_id: Id<GuildMarker>,
    ) -> Result<(), twilight_http::Error> {
        let global_commands = Self::global_commands();
        client.set_global_commands(&global_commands).await?;

        let admin_commands = Self::admin_guild_commands();
        client
            .set_guild_commands(admin_guild_id, &admin_commands)
            .await?;
        Ok(())
    }
}
