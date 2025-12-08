use serde::{Deserialize, Serialize};

pub mod admin;
pub mod app;
pub mod auth;
pub mod bit;
pub mod chat;
pub mod health;
pub mod info;
pub mod oauth;
pub mod profile;
pub mod solution;
pub mod store;
pub mod tmp;
pub mod user;
pub mod webhook;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct LanguageParams {
    pub language: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct PaginationParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}
