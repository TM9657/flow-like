//! Discord integration for Flow-Like
//!
//! This module provides nodes for interacting with Discord using the serenity library.
//!
//! ## Usage Flow
//! 1. Receive a chat event from Discord sink (via Chat Event node)
//! 2. Use `To Discord Session` node to create a session from `global_session`
//! 3. Use various Discord operation nodes with the session

#[cfg(feature = "execute")]
pub mod channel;
#[cfg(feature = "execute")]
pub mod dm;
#[cfg(feature = "execute")]
pub mod interaction;
#[cfg(feature = "execute")]
pub mod media;
#[cfg(feature = "execute")]
pub mod message;
#[cfg(feature = "execute")]
pub mod poll;
#[cfg(feature = "execute")]
pub mod reaction;
#[cfg(feature = "execute")]
pub mod session;
#[cfg(feature = "execute")]
pub mod user;

#[cfg(feature = "execute")]
pub use interaction::{ButtonResponse, SelectMenuResponse, UserReply};
#[cfg(feature = "execute")]
pub use media::SentAttachment;
#[cfg(feature = "execute")]
pub use poll::{PollAnswerResult, PollReference, PollResults};
#[cfg(feature = "execute")]
pub use session::{CachedDiscordClient, DiscordSession};
#[cfg(feature = "execute")]
pub use user::DiscordUser;
