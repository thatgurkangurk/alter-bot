use crate::{
    module::{CommandScope, Module, ModuleCommand, ModuleEntry},
    util::get_avatar_url,
};
use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::application_command::CommandData,
    },
    gateway::payload::incoming::InteractionCreate,
    http::{
        attachment::Attachment,
        interaction::{InteractionResponse, InteractionResponseType},
    },
    id::{Id, marker::MessageMarker},
    user::User,
};
use twilight_util::builder::command::CommandBuilder;

pub struct QuoteModule;

pub fn command() -> Command {
    CommandBuilder::new("Quote", "", CommandType::Message).build()
}

fn create_username(user: &User) -> String {
    if user.discriminator != 0 {
        //? if they for SOME reason have an old username (most likely a bot)
        format!("{}#{:04}", user.name, user.discriminator)
    } else {
        format!("@{}", user.name)
    }
}

pub async fn run(
    interaction: Box<InteractionCreate>,
    data: Box<CommandData>,
) -> anyhow::Result<()> {
    let interaction_id = interaction.id;
    let token = &interaction.token;

    crate::CTX
        .http
        .interaction(interaction.application_id)
        .create_response(
            interaction_id,
            token,
            &InteractionResponse {
                kind: InteractionResponseType::DeferredChannelMessageWithSource,
                data: None,
            },
        )
        .await?;

    let target_id: Id<MessageMarker> = match data.target_id {
        Some(id) => id.cast(),
        None => anyhow::bail!("no target message provided"),
    };

    let message = data
        .resolved
        .as_ref()
        .and_then(|resolved| resolved.messages.get(&target_id))
        .ok_or_else(|| anyhow::anyhow!("target message not found"))?
        .clone();

    let content = message.content;
    let user = message.author;

    let avatar_url = get_avatar_url(user.avatar, user.id);

    let image = match quoter::create_quote_image(
        &avatar_url,
        &content,
        &format!("- {}", user.name),
        &create_username(&user),
        quoter::ImageFormat::Png,
    )
    .await
    {
        Ok(img) => img,
        Err(err) => {
            let msg = match err.downcast_ref::<quoter::QuoterError>() {
                Some(quoter_err) => quoter_err.to_string(),
                None => "an unexpected error occurred while generating the quote image".to_string(),
            };

            tracing::error!("failed to generate quote image: {err:?}");

            crate::CTX
                .interaction()
                .update_response(token)
                .content(Some(&msg))
                .await?;

            return Ok(());
        }
    };

    let attachment = Attachment::from_bytes("quote.png".to_string(), image, 1);

    crate::CTX
        .interaction()
        .update_response(token)
        .attachments(&[attachment])
        .await?;

    Ok(())
}

#[async_trait::async_trait]
impl Module for QuoteModule {
    fn name(&self) -> &'static str {
        "quote"
    }

    fn commands(&self) -> Vec<ModuleCommand> {
        vec![ModuleCommand {
            name: "quote",
            scope: CommandScope::Global,
            builder: command,
            run: |i, d| Box::pin(run(i, d)),
            autocomplete: None,
        }]
    }
}

inventory::submit! {
    ModuleEntry {
        module: &QuoteModule
    }
}
