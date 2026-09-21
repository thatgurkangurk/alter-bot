use crate::bot::{Context, Error};
use ::serenity::all::{CreateAttachment, User};
use image::EncodableLayout;
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

    let image = match super::image::generate_quote_image(
        &avatar_url,
        &content,
        &format!("- {}", user.display_name()),
        &create_username(&user),
    )
    .await
    {
        Ok(img) => img,
        Err(e) => {
            error!("Failed to generate quote image: {e:?}");
            ctx.say("uh oh something went wrong").await?;
            return Ok(());
        }
    };

    let mut bytes: Vec<u8> = Vec::new();
    image::DynamicImage::ImageRgba8(image).write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )?;

    let attachment = CreateAttachment::bytes(bytes.as_bytes(), "quote.png");

    let message_builder = CreateReply::default().attachment(attachment);

    ctx.send(message_builder).await?;

    Ok(())
}

pub fn quote_commands(mut cmds: Vec<crate::bot::Command>) -> Vec<crate::bot::Command> {
    cmds.push(quote());

    cmds
}
