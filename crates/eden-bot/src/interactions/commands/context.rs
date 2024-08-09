use crate::interactions::{util::UnknownCommandError, InteractionContext};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};

pub type CommandContext = InteractionContext<CommandData>;
impl CommandContext {
    /// Gets the actual command name including subcommands
    pub fn command_name(&self) -> String {
        use std::fmt::Write as _;

        let mut output = String::from(&self.data.name);
        for option in self.data.options.iter() {
            match &option.value {
                CommandOptionValue::SubCommand(..) => {
                    write!(&mut output, " {}", option.name).expect("could not push string");
                }
                CommandOptionValue::SubCommandGroup(value) => {
                    // TODO: Form a name from subcommand group
                    let value = value.first().map(|v| v.name.as_str()).unwrap_or("");
                    write!(&mut output, " {} {value}", option.name).expect("could not push string");
                }
                _ => {}
            }
        }
        output
    }

    /// Returns unimplemented command error.
    pub fn unimplemented_cmd(&self) -> eden_utils::Result<()> {
        Err(eden_utils::Error::context_anonymize(
            eden_utils::ErrorCategory::Unknown,
            UnknownCommandError(self.command_name()),
        ))
    }
}
