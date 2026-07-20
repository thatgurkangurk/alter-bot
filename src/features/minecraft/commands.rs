use crate::bot::{Context, Error};
use anyhow::Result;
use craftping::Response;
use craftping::tokio::ping;
use poise::serenity_prelude as serenity;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::time::{Duration, timeout};

async fn get_minecraft_server_status(hostname: &str, port: u16) -> Result<Response> {
    let connection = TcpStream::connect((hostname, port));
    let mut stream = timeout(Duration::from_secs(5), connection).await??;

    let status = ping(&mut stream, hostname, port).await?;

    Ok(status)
}

fn extract_motd(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Object(obj) => {
            let mut out = String::new();
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                out.push_str(text);
            }
            if let Some(extra) = obj.get("extra").and_then(|v| v.as_array()) {
                for e in extra {
                    out.push_str(&extract_motd(e));
                }
            }
            out
        }
        Value::Array(arr) => arr.iter().map(extract_motd).collect(),
        _ => String::new(),
    }
}

/// converts minecraft colour codes (§a, &b) into discord compatible ansi codes
fn mc_to_ansi(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars();

    while let Some(c) = chars.next() {
        if c == '§' || c == '&' {
            // take the next character safely
            if let Some(next) = chars.next() {
                let ansi_code = match next {
                    '0' | '7' | '8' => "\u{001b}[0;30m",
                    '1' | '9' => "\u{001b}[0;34m",
                    '2' | 'a' => "\u{001b}[0;32m",
                    '3' | 'b' => "\u{001b}[0;36m",
                    '4' | 'c' => "\u{001b}[0;31m",
                    '5' | 'd' => "\u{001b}[0;35m",
                    '6' | 'e' => "\u{001b}[0;33m",
                    'f' => "\u{001b}[0;37m",
                    'l' => "\u{001b}[1m",
                    'r' => "\u{001b}[0m",
                    _ => {
                        result.push(c);
                        result.push(next);
                        continue;
                    }
                };
                result.push_str(ansi_code);
            } else {
                // the string ended with a trailing § or &
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }

    result.push_str("\u{001b}[0m");
    result
}

#[poise::command(slash_command, guild_only)]
/// a command to get the current status of a minecraft java edition server
pub async fn server_status(
    ctx: Context<'_>,
    #[description = "server hostname"] hostname: String,
    #[description = "port"] port: Option<u16>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let target_port = port.unwrap_or(25565);
    let fallback_icon = include_bytes!("unknown_server.png");

    match get_minecraft_server_status(&hostname, target_port).await {
        Ok(status) => {
            let (icon_bytes, filename) = status.favicon.map_or_else(
                || (fallback_icon.to_vec(), "unknown_server.png"),
                |favicon_data| (favicon_data, "icon.png"),
            );

            let discord_attachment = serenity::CreateAttachment::bytes(icon_bytes, filename);

            let players_text = match status.sample {
                Some(players) if !players.is_empty() => {
                    let mut names: Vec<String> = players.into_iter().map(|p| p.name).collect();

                    // only the first 15 so we don't go above the 1024 character limit
                    if names.len() > 15 {
                        let remaining = names.len() - 15;
                        names.truncate(15);
                        names.push(format!("...and {remaining} more"));
                    }
                    names.join(", ")
                }
                _ => "no players online (or hidden by server)".to_string(),
            };

            let raw_motd = status
                .description
                .as_ref()
                .map_or_else(|| "no MOTD provided".to_string(), extract_motd);

            let colored_motd = mc_to_ansi(&raw_motd);

            let motd_discord_format = format!("```ansi\n{colored_motd}\n```");

            #[allow(clippy::unreadable_literal)]
            let mut embed = serenity::CreateEmbed::new()
                .title(hostname)
                .color(0x00FF00)
                .thumbnail(format!("attachment://{filename}"))
                .field(
                    "players",
                    format!("{}/{}", status.online_players, status.max_players),
                    true,
                )
                .field("version", status.version, true)
                .field("protocol", status.protocol.to_string(), true)
                .field("online players", players_text, false)
                .description(motd_discord_format);

            if let Some(mod_info) = status.mod_info {
                embed = embed.field("loader", mod_info.mod_type, true);

                if !mod_info.mod_list.is_empty() {
                    embed =
                        embed.field("mods installed", mod_info.mod_list.len().to_string(), true);
                }
            }

            if let Some(enforces_secure_chat) = status.enforces_secure_chat {
                embed = embed.field(
                    "enforces secure chat",
                    enforces_secure_chat.to_string(),
                    true,
                );
            }

            let reply = poise::CreateReply::default()
                .embed(embed)
                .attachment(discord_attachment);

            ctx.send(reply).await?;
        }
        Err(e) => {
            let discord_attachment =
                serenity::CreateAttachment::bytes(fallback_icon.to_vec(), "unknown_server.png");

            #[allow(clippy::unreadable_literal)]
            let embed = serenity::CreateEmbed::new()
                .title(hostname)
                .color(0xFF0000)
                .thumbnail("attachment://unknown_server.png")
                .field("status", "offline", false)
                .description(format!("could not connect to the server.\n*error: {e}*"));

            let reply = poise::CreateReply::default()
                .embed(embed)
                .attachment(discord_attachment);

            ctx.send(reply).await?;
        }
    }

    Ok(())
}

pub fn minecraft_commands(mut cmds: Vec<crate::bot::Command>) -> Vec<crate::bot::Command> {
    cmds.push(server_status());

    cmds
}
