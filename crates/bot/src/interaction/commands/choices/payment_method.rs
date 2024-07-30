use once_cell::sync::Lazy;
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
            Self::Mynt => &*MYNT_ALIAS_NAME_IN_LOWERCASE,
            Self::PayPal => "paypal",
        }
    }
}

impl Debug for PaymentMethodOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Mynt => &*MYNT_ALIAS_NAME,
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
            name: MYNT_ALIAS_NAME.to_string(),
            name_localizations: None,
            value: CommandOptionChoiceValue::String(MYNT_ALIAS_NAME_IN_LOWERCASE.to_string()),
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

// I cannot use the name `____` inside this repository so I
// called it Mynt in the mean time
static MYNT_ALIAS_NAME: Lazy<String> = Lazy::new(|| {
    let resolved_name = eden_utils::env::var_opt("EDEN_MYNT_ALIAS")
        .ok()
        .and_then(|v| v);

    resolved_name.unwrap_or_else(|| String::from("Mynt"))
});

static MYNT_ALIAS_NAME_IN_LOWERCASE: Lazy<String> = Lazy::new(|| MYNT_ALIAS_NAME.to_lowercase());

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
            n if n == &*MYNT_ALIAS_NAME_IN_LOWERCASE => Ok(Self::Mynt),
            "paypal" => Ok(Self::PayPal),
            _ => Err(twilight_interactions::error::ParseOptionErrorType::InvalidChoice(parsed)),
        }
    }
}
