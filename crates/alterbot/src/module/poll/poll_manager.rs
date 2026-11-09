use std::sync::Arc;
use tokio::time::{Duration, interval};

use chrono::Utc;
use dashmap::DashMap;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait,
    QueryFilter,
};
use twilight_model::{
    channel::message::{
        Embed,
        component::{ActionRow, Button, ButtonStyle, Component},
    },
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker, UserMarker},
    },
    util::ImageHash,
};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::{
    CTX,
    entity::{guild as server, poll, vote},
};

pub struct CreatePollParams {
    pub channel_id: Id<ChannelMarker>,
    pub guild_id: Id<GuildMarker>,
    pub author_id: Id<UserMarker>,
    pub author_name: String,
    pub author_display_name: String,
    pub author_avatar: Option<ImageHash>,
    pub question: String,
    pub description: Option<String>,
    pub end_time: chrono::DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CachedPoll {
    pub message_id: i64,
    pub channel_id: i64,
    pub end_time: chrono::DateTime<Utc>,
    pub is_closed: bool,
}

#[derive(Debug)]
pub struct PollManager {
    db: DatabaseConnection,
    polls: DashMap<i64, CachedPoll>,
}

impl PollManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            polls: DashMap::new(),
        }
    }

    pub fn start_tasks(self: Arc<Self>) {
        let manager_sec = Arc::clone(&self);
        let manager_sync = Arc::clone(&self);

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1));
            loop {
                ticker.tick().await;
                manager_sec.check_expired_polls_in_memory().await;
            }
        });

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(60));
            loop {
                ticker.tick().await;
                if let Err(err) = manager_sync.sync_active_polls_from_db().await {
                    tracing::error!("failed to sync active polls from db: {:?}", err);
                }
            }
        });
    }

    pub async fn sync_active_polls_from_db(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let active_polls = poll::Entity::find()
            .filter(poll::Column::IsClosed.eq(false))
            .all(&self.db)
            .await?;

        for p in active_polls {
            self.polls.insert(
                p.message_id,
                CachedPoll {
                    message_id: p.message_id,
                    channel_id: p.channel_id,
                    end_time: p.end_time,
                    is_closed: p.is_closed,
                },
            );
        }

        Ok(())
    }

    async fn check_expired_polls_in_memory(&self) {
        let now = Utc::now();
        let mut expired_ids = Vec::new();

        for entry in self.polls.iter() {
            let poll = entry.value();
            if !poll.is_closed && poll.end_time <= now {
                expired_ids.push(poll.message_id);
            }
        }

        for message_id in expired_ids {
            if let Some(mut cached) = self.polls.get_mut(&message_id) {
                if cached.is_closed {
                    continue;
                }
                cached.is_closed = true;
            }

            if let Err(err) = self.close_poll_by_id(message_id).await {
                tracing::error!("error closing expired poll {}: {:?}", message_id, err);
            } else {
                self.polls.remove(&message_id);
            }
        }
    }

    pub async fn create_poll(
        &self,
        params: CreatePollParams,
    ) -> Result<poll::Model, Box<dyn std::error::Error + Send + Sync>> {
        let embed = build_poll_embed(PollEmbedParams {
            question: &params.question,
            description: params.description.as_deref(),
            end_time: params.end_time,
            is_closed: false,
            yes_votes: 0,
            no_votes: 0,
            author_name: &params.author_display_name,
            author_id: params.author_id,
            author_avatar: params.author_avatar,
        });
        let components = build_poll_buttons(false);

        let msg = CTX
            .http
            .create_message(params.channel_id)
            .embeds(&[embed])
            .components(&components)
            .await?
            .model()
            .await?;

        let poll_model = poll::Entity::create_poll(
            &self.db,
            poll::CreatePoll {
                message_id: msg.id,
                channel_id: params.channel_id,
                guild_id: params.guild_id,
                author_id: params.author_id,
                question: params.question,
                description: params.description,
                end_time: params.end_time,
            },
        )
        .await?;

        self.polls.insert(
            poll_model.message_id,
            CachedPoll {
                message_id: poll_model.message_id,
                channel_id: poll_model.channel_id,
                end_time: poll_model.end_time,
                is_closed: false,
            },
        );

        Ok(poll_model)
    }

    pub async fn update_poll_embed(
        &self,
        message_id: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let poll_option = poll::Entity::find_by_id(message_id).one(&self.db).await?;
        let poll = match poll_option {
            Some(p) if !p.is_closed => p,
            _ => return Ok(()),
        };

        let author_id = poll.author_id();
        let guild_id = poll.guild_id();

        let (author_name, author_avatar) = fetch_member_display_info(guild_id, author_id).await;

        let votes = poll.find_related(vote::Entity).all(&self.db).await?;
        let yes_count = votes.iter().filter(|v| v.choice).count();
        let no_count = votes.iter().filter(|v| !v.choice).count();

        let embed = build_poll_embed(PollEmbedParams {
            question: &poll.question,
            description: poll.description.as_deref(),
            end_time: poll.end_time,
            is_closed: false,
            yes_votes: yes_count,
            no_votes: no_count,
            author_name: &author_name,
            author_id,
            author_avatar,
        });
        let components = build_poll_buttons(false);

        let _ = CTX
            .http
            .update_message(poll.channel_id(), poll.message_id())
            .embeds(Some(&[embed]))
            .components(Some(&components))
            .await;

        Ok(())
    }

    pub async fn close_poll_by_id(
        &self,
        message_id: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let poll_option = poll::Entity::find_by_id(message_id).one(&self.db).await?;
        let poll = match poll_option {
            Some(p) => p,
            None => return Ok(()),
        };

        let author_id = poll.author_id();
        let guild_id = poll.guild_id();

        let (author_name, author_avatar) = fetch_member_display_info(guild_id, author_id).await;

        let votes = poll.find_related(vote::Entity).all(&self.db).await?;
        let yes_count = votes.iter().filter(|v| v.choice).count();
        let no_count = votes.iter().filter(|v| !v.choice).count();

        let embed = build_poll_embed(PollEmbedParams {
            question: &poll.question,
            description: poll.description.as_deref(),
            end_time: poll.end_time,
            is_closed: true,
            yes_votes: yes_count,
            no_votes: no_count,
            author_name: &author_name,
            author_id,
            author_avatar,
        });
        let components = build_poll_buttons(true);

        let _ = CTX
            .http
            .update_message(poll.channel_id(), poll.message_id())
            .embeds(Some(&[embed]))
            .components(Some(&components))
            .await;

        let log_channel_id = server::Entity::find_by_id(guild_id.get() as i64)
            .one(&self.db)
            .await
            .ok()
            .flatten()
            .and_then(|s| s.poll_log_channel_id);

        if let Some(log_channel_id) = log_channel_id {
            let target_channel: Id<ChannelMarker> = Id::new(log_channel_id as u64);

            let mut yes_voters = Vec::new();
            let mut no_voters = Vec::new();

            for v in &votes {
                let user_id: Id<UserMarker> = Id::new(v.user_id as u64);
                let (voter_name, _) = fetch_member_display_info(guild_id, user_id).await;
                let entry = format!("<@{}> ({})", v.user_id, voter_name);

                if v.choice {
                    yes_voters.push(entry);
                } else {
                    no_voters.push(entry);
                }
            }

            let yes_str = if yes_voters.is_empty() {
                "*none*".to_string()
            } else {
                yes_voters.join("\n")
            };

            let no_str = if no_voters.is_empty() {
                "*none*".to_string()
            } else {
                no_voters.join("\n")
            };

            let log_desc = format!(
                "**question:** {}\n**message id:** {}\n\n{} **yes ({})**\n{}\n\n{} **no ({})**\n{}",
                poll.question,
                message_id,
                crate::emoji::YES,
                yes_count,
                yes_str,
                crate::emoji::NO,
                no_count,
                no_str
            );

            let log_embed = EmbedBuilder::new()
                .title("poll results")
                .description(log_desc)
                .color(0x5865F2)
                .footer(EmbedFooterBuilder::new(format!("poll id: {}", message_id)).build())
                .validate()
                .expect("valid embed parameters")
                .build();

            let _ = CTX
                .http
                .create_message(target_channel)
                .embeds(&[log_embed])
                .await;
        }

        let mut active_model: poll::ActiveModel = poll.into();
        active_model.is_closed = Set(true);
        active_model.update(&self.db).await?;

        self.polls.remove(&message_id);

        Ok(())
    }
}

pub struct PollEmbedParams<'a> {
    pub question: &'a str,
    pub description: Option<&'a str>,
    pub end_time: chrono::DateTime<Utc>,
    pub is_closed: bool,
    pub yes_votes: usize,
    pub no_votes: usize,
    pub author_name: &'a str,
    pub author_id: Id<UserMarker>,
    pub author_avatar: Option<ImageHash>,
}

fn build_poll_embed(params: PollEmbedParams<'_>) -> Embed {
    let mut desc = params.description.unwrap_or("").to_string();

    if params.is_closed {
        let winner = match params.yes_votes.cmp(&params.no_votes) {
            std::cmp::Ordering::Greater => format!("winner: {} Yes", crate::emoji::YES),
            std::cmp::Ordering::Less => format!("winner: {} No", crate::emoji::NO),
            std::cmp::Ordering::Equal => "it's a tie !".to_string(),
        };

        desc.push_str(&format!(
            "\n\n**poll closed!**\n{} Yes: {} | {} No: {}\n\n**{}**",
            crate::emoji::YES,
            params.yes_votes,
            crate::emoji::NO,
            params.no_votes,
            winner
        ));
    } else {
        let timestamp = params.end_time.timestamp();
        desc.push_str(&format!("\n\nends <t:{}:R>", timestamp));
    }

    let color = if params.is_closed { 0xED4245 } else { 0x57F287 };
    let total_votes = params.yes_votes + params.no_votes;

    let mut author_builder =
        EmbedAuthorBuilder::new(format!("suggested by: {}", params.author_name));

    if let Some(hash) = params.author_avatar {
        let ext = if hash.is_animated() { "gif" } else { "png" };
        let avatar_url = format!(
            "https://cdn.discordapp.com/avatars/{}/{}.{}",
            params.author_id, hash, ext
        );
        if let Ok(icon) = ImageSource::url(avatar_url) {
            author_builder = author_builder.icon_url(icon);
        }
    }

    let footer_text = if total_votes == 1 {
        "1 vote total".to_string()
    } else {
        format!("{} votes total", total_votes)
    };

    EmbedBuilder::new()
        .title(params.question)
        .description(desc)
        .color(color)
        .author(author_builder.build())
        .footer(EmbedFooterBuilder::new(footer_text).build())
        .validate()
        .expect("Valid embed parameters")
        .build()
}

fn build_poll_buttons(disabled: bool) -> Vec<Component> {
    vec![Component::ActionRow(ActionRow {
        id: None,
        components: vec![
            Component::Button(Button {
                id: None,
                custom_id: Some("poll_vote_yes".to_string()),
                disabled,
                emoji: (&crate::emoji::YES).into(),
                label: Some("Yes".to_string()),
                style: ButtonStyle::Success,
                url: None,
                sku_id: None,
            }),
            Component::Button(Button {
                id: None,
                custom_id: Some("poll_vote_no".to_string()),
                disabled,
                emoji: (&crate::emoji::NO).into(),
                label: Some("No".to_string()),
                style: ButtonStyle::Danger,
                url: None,
                sku_id: None,
            }),
        ],
    })]
}

async fn fetch_member_display_info(
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
) -> (String, Option<ImageHash>) {
    let get_member = || async {
        let member = CTX
            .http
            .guild_member(guild_id, user_id)
            .await
            .ok()?
            .model()
            .await
            .ok()?;
        let display_name = member
            .nick
            .or(member.user.global_name)
            .unwrap_or(member.user.name);
        Some((display_name, member.avatar.or(member.user.avatar)))
    };

    let get_user = || async {
        let user = CTX.http.user(user_id).await.ok()?.model().await.ok()?;
        let display_name = user.global_name.unwrap_or(user.name);
        Some((display_name, user.avatar))
    };

    if let Some(info) = get_member().await {
        return info;
    }

    get_user()
        .await
        .unwrap_or_else(|| ("Unknown User".to_string(), None))
}
