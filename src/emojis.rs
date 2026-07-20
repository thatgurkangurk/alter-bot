#![allow(clippy::unreadable_literal)]

use poise::serenity_prelude as serenity;

pub struct BotEmoji {
    #[allow(dead_code)]
    pub id: serenity::EmojiId,
    pub text: &'static str,
}

pub const YES: BotEmoji = BotEmoji {
    id: serenity::EmojiId::new(1501867651511746622),
    text: "<a:yes_name:1501867651511746622>", // 'a' for animated
};

pub const NO: BotEmoji = BotEmoji {
    id: serenity::EmojiId::new(1501867649611858041),
    text: "<:no_name:1501867649611858041>",
};

pub const HARD_NO: BotEmoji = BotEmoji {
    id: serenity::EmojiId::new(1501867648294715412),
    text: "<:hard_no_name:1501867648294715412>",
};
