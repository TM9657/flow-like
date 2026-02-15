//! Telegram integration for Flow-Like
//!
//! This module provides nodes for interacting with Telegram using the teloxide library.
//!
//! ## Usage Flow
//! 1. Receive a chat event from Telegram sink (via Chat Event node)
//! 2. Use `To Telegram Session` node to create a session from `global_session`
//! 3. Use various Telegram operation nodes with the session

#[cfg(feature = "execute")]
pub mod bot;
#[cfg(feature = "execute")]
pub mod business;
#[cfg(feature = "execute")]
pub mod chat;
#[cfg(feature = "execute")]
pub mod commands;
#[cfg(feature = "execute")]
pub mod files;
#[cfg(feature = "execute")]
pub mod forum;
#[cfg(feature = "execute")]
pub mod games;
#[cfg(feature = "execute")]
pub mod gifts;
#[cfg(feature = "execute")]
pub mod inline;
#[cfg(feature = "execute")]
pub mod interaction;
#[cfg(feature = "execute")]
pub mod interactive;
#[cfg(feature = "execute")]
pub mod invite;
#[cfg(feature = "execute")]
pub mod media;
#[cfg(feature = "execute")]
pub mod member;
#[cfg(feature = "execute")]
pub mod message;
#[cfg(feature = "execute")]
pub mod payments;
#[cfg(feature = "execute")]
pub mod poll;
#[cfg(feature = "execute")]
pub mod session;
#[cfg(feature = "execute")]
pub mod stickers;
#[cfg(feature = "execute")]
pub mod stories;
#[cfg(feature = "execute")]
pub mod user;

#[cfg(feature = "execute")]
pub use bot::BotInfo;
#[cfg(feature = "execute")]
pub use business::{BusinessConnection, StarBalance};
#[cfg(feature = "execute")]
pub use commands::{AdminRights, BotCommandInfo};
#[cfg(feature = "execute")]
pub use files::{FileInfo, PhotoInfo, UserProfilePhotosResult};
#[cfg(feature = "execute")]
pub use forum::{ForumTopicInfo, StickerInfo};
#[cfg(feature = "execute")]
pub use games::GameHighScore;
#[cfg(feature = "execute")]
pub use gifts::GiftInfo;
#[cfg(feature = "execute")]
pub use inline::SentWebAppMessageInfo;
#[cfg(feature = "execute")]
pub use interaction::{CallbackResponse, UserReply};
#[cfg(feature = "execute")]
pub use invite::ChatInviteLink;
#[cfg(feature = "execute")]
pub use member::{AdminInfo, ChatMemberInfo};
#[cfg(feature = "execute")]
pub use payments::{InvoiceLink, LabeledPrice, StarTransaction};
#[cfg(feature = "execute")]
pub use poll::{PollReference, PollResults};
#[cfg(feature = "execute")]
pub use session::{CachedTelegramBot, TelegramSession};
#[cfg(feature = "execute")]
pub use stickers::{MaskPositionInfo, StickerInfo as StickerInfoFull, StickerSetInfo};
#[cfg(feature = "execute")]
pub use stories::StoryInfo;
#[cfg(feature = "execute")]
pub use user::TelegramUser;
