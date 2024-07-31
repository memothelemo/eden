use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};

use crate::interaction::InteractionContext;

pub type CommandContext<'a> = InteractionContext<'a, CommandData>;
impl<'a> CommandContext<'a> {
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
}
