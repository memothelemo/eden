use eden_utils::aliases;
use std::fmt::Debug;
use twilight_interactions::command::{CommandOption, CreateOption};
use twilight_model::application::command::{
    CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaymentMethodOption {
    Mynt,
    PayPal,
}

impl PaymentMethodOption {
    #[must_use]
    pub fn value(&self) -> &'static str {
        match self {
            Self::Mynt => &*aliases::MYNT_NAME_LOWERCASE,
            Self::PayPal => "paypal",
        }
    }
}

impl Debug for PaymentMethodOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Mynt => &*aliases::MYNT_NAME,
            Self::PayPal => "PayPal",
        })
    }
}

impl CreateOption for PaymentMethodOption {
    fn create_option(
        data: twilight_interactions::command::internal::CreateOptionData,
    ) -> twilight_model::application::command::CommandOption {
        let mut choices = Vec::with_capacity(2);
        choices.push(CommandOptionChoice {
            name: aliases::MYNT_NAME.to_string(),
            name_localizations: None,
            value: CommandOptionChoiceValue::String(aliases::MYNT_NAME_LOWERCASE.to_string()),
        });
        choices.push(CommandOptionChoice {
            name: "PayPal".into(),
            name_localizations: None,
            value: CommandOptionChoiceValue::String("paypal".into()),
        });
        data.builder(CommandOptionType::String)
            .choices(choices)
            .build()
    }
}

impl CommandOption for PaymentMethodOption {
    fn from_option(
        value: twilight_model::application::interaction::application_command::CommandOptionValue,
        _data: twilight_interactions::command::internal::CommandOptionData,
        resolved: Option<&twilight_model::application::interaction::application_command::CommandInteractionDataResolved>,
    ) -> Result<Self, twilight_interactions::error::ParseOptionErrorType> {
        let parsed: String = twilight_interactions::command::CommandOption::from_option(
            value,
            twilight_interactions::command::internal::CommandOptionData::default(),
            resolved,
        )?;
        match parsed.as_str() {
            n if n == &*aliases::MYNT_NAME_LOWERCASE => Ok(Self::Mynt),
            "paypal" => Ok(Self::PayPal),
            _ => Err(twilight_interactions::error::ParseOptionErrorType::InvalidChoice(parsed)),
        }
    }
}
