use crate::bot::{Context, Error};
use ::serenity::all::{CreateAttachment, User};
use poise::{CreateReply, serenity_prelude as serenity};
use tracing::error;

#[inline]
fn create_username(user: &User) -> String {
    if user.discriminator.is_some() {
        //? if they for SOME reason have an old username (most likely a bot)
        user.tag()
    } else {
        format!("@{}", user.name)
    }
}

#[poise::command(context_menu_command = "Quote")]
async fn quote(
    ctx: Context<'_>,
    #[description = "message to quote"] msg: serenity::Message,
) -> Result<(), Error> {
    let user = msg.author;
    let content = msg.content;

    let avatar_url = user.static_face();

    ctx.defer().await?;

    let image = match quoter::create_quote_image(
        &avatar_url,
        &content,
        &format!("- {}", user.display_name()),
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

            error!("failed to generate quote image: {err:?}");
            ctx.say(msg).await?;
            return Ok(());
        }
    };

    let attachment = CreateAttachment::bytes(image, "quote.png");

    let message_builder = CreateReply::default().attachment(attachment);

    ctx.send(message_builder).await?;

    Ok(())
}

pub fn quote_commands(mut cmds: Vec<crate::bot::Command>) -> Vec<crate::bot::Command> {
    cmds.push(quote());

    cmds
}
