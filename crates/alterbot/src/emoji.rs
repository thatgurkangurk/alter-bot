use twilight_model::{
    channel::message::EmojiReactionType,
    id::{Id, marker::EmojiMarker},
};

pub struct CustomEmoji {
    pub id: Id<EmojiMarker>,
    pub name: &'static str,
    pub animated: bool,
}

impl CustomEmoji {
    pub const fn new(id: u64, name: &'static str, animated: bool) -> Self {
        Self {
            id: Id::new(id),
            name,
            animated,
        }
    }

    pub fn reaction(&self) -> EmojiReactionType {
        EmojiReactionType::Custom {
            animated: self.animated,
            id: self.id,
            name: Some(self.name.to_string()),
        }
    }
}

impl std::fmt::Display for CustomEmoji {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.animated {
            write!(f, "<a:{}:{}>", self.name, self.id)
        } else {
            write!(f, "<:{}:{}>", self.name, self.id)
        }
    }
}

impl From<&CustomEmoji> for Option<EmojiReactionType> {
    fn from(emoji: &CustomEmoji) -> Self {
        Some(emoji.reaction())
    }
}

pub const YES: CustomEmoji = CustomEmoji::new(
    if cfg!(debug_assertions) {
        1501867651511746622
    } else {
        1569369144955109517
    },
    "yes",
    true,
);

pub const NO: CustomEmoji = CustomEmoji::new(
    if cfg!(debug_assertions) {
        1501867649611858041
    } else {
        1569369148163629066
    },
    "no",
    false,
);
