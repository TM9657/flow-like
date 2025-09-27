use std::sync::Arc;

use crate::{
    entity::{membership, pat, prelude::*, role, sea_orm_active_enums, technical_user, user},
    error::{ApiError, AuthorizationError},
    permission::{
        global_permission::GlobalPermission,
        role_permission::{RolePermissions, has_role_permission},
    },
};
use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use flow_like::hub::UserTier;
use flow_like_types::Result;
use flow_like_types::anyhow;
use hyper::header::AUTHORIZATION;
use sea_orm::{
    ColumnTrait, EntityTrait, JoinType, QueryFilter, QuerySelect, RelationTrait,
    sqlx::types::chrono,
};

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct OpenIDUser {
    pub sub: String,
    pub username: String,
    pub preferred_username: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PATUser {
    pub pat: String,
    pub sub: String,
}

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub key_id: String,
    pub api_key: String,
    pub app_id: String,
}

#[derive(Debug, Clone)]
pub enum AppUser {
    OpenID(OpenIDUser),
    PAT(PATUser),
    APIKey(ApiKey),
    Unauthorized,
}

pub struct AppPermissionResponse {
    pub state: AppState,
    pub permissions: RolePermissions,
    pub role: Arc<role::Model>,
    pub sub: Option<String>,
    pub identifier: String,
}

impl AppPermissionResponse {
    pub fn has_permission(&self, permission: RolePermissions) -> bool {
        has_role_permission(&self.permissions, permission)
    }

    pub fn sub(&self) -> Result<String> {
        self.sub.clone().ok_or_else(|| anyhow!("No sub available"))
    }

    /// Either returns the sub if available or in case of API keys it returns the key ID.
    /// This is useful for identifying the user in logs or other contexts where a unique identifier is needed.
    pub fn identifier(&self) -> String {
        self.identifier.clone()
    }
}

impl AppUser {
    pub fn sub(&self) -> Result<String, AuthorizationError> {
        match self {
            AppUser::OpenID(user) => Ok(user.sub.clone()),
            AppUser::PAT(user) => Ok(user.sub.clone()),
            AppUser::APIKey(_) => Err(AuthorizationError::from(anyhow!(
                "APIKey user does not have a sub"
            ))),
            AppUser::Unauthorized => Err(AuthorizationError::from(anyhow!(
                "Unauthorized user does not have a sub"
            ))),
        }
    }

    pub async fn tracking_id(
        &self,
        state: &AppState,
    ) -> Result<Option<String>, AuthorizationError> {
        let sub = self.sub()?;
        let user = user::Entity::find_by_id(&sub)
            .one(&state.db)
            .await?
            .ok_or_else(|| AuthorizationError::from(anyhow!("User not found")))?;
        Ok(user.tracking_id)
    }

    pub async fn tier(&self, state: &AppState) -> Result<UserTier, AuthorizationError> {
        let sub = self.sub()?;
        let user = user::Entity::find_by_id(&sub)
            .one(&state.db)
            .await?
            .ok_or_else(|| AuthorizationError::from(anyhow!("User not found")))?;

        let db_tier = match user.tier {
            sea_orm_active_enums::UserTier::Free => "FREE",
            sea_orm_active_enums::UserTier::Premium => "PREMIUM",
            sea_orm_active_enums::UserTier::Pro => "PRO",
            sea_orm_active_enums::UserTier::Enterprise => "ENTERPRISE",
        };

        let tier = state
            .platform_config
            .tiers
            .get(db_tier)
            .cloned()
            .ok_or_else(|| AuthorizationError::from(anyhow!("Tier not found")))?;
        Ok(tier)
    }

    pub async fn get_user(&self, state: &AppState) -> Result<user::Model, AuthorizationError> {
        let sub = self.sub()?;
        user::Entity::find_by_id(&sub)
            .one(&state.db)
            .await?
            .ok_or_else(|| AuthorizationError::from(anyhow!("User not found")))
    }

    pub fn email(&self) -> Option<String> {
        match self {
            AppUser::OpenID(user) => user.email.clone(),
            AppUser::PAT(_) => None,
            AppUser::APIKey(_) => None,
            AppUser::Unauthorized => None,
        }
    }

    pub fn username(&self) -> Option<String> {
        match self {
            AppUser::OpenID(user) => Some(user.username.clone()),
            AppUser::PAT(_) => None,
            AppUser::APIKey(_) => None,
            AppUser::Unauthorized => None,
        }
    }

    pub fn preferred_username(&self) -> Option<String> {
        match self {
            AppUser::OpenID(user) => Some(user.preferred_username.clone()),
            AppUser::PAT(_) => None,
            AppUser::APIKey(_) => None,
            AppUser::Unauthorized => None,
        }
    }

    pub async fn global_permission(&self, state: AppState) -> Result<GlobalPermission, ApiError> {
        let sub = self.sub()?;
        let user = user::Entity::find_by_id(&sub)
            .one(&state.db)
            .await?
            .ok_or_else(|| anyhow!("User not found"))?;
        let permission = GlobalPermission::from_bits(user.permission)
            .ok_or_else(|| anyhow!("Invalid permission bits"))?;
        Ok(permission)
    }

    pub async fn check_global_permission(
        &self,
        state: &AppState,
        permission: GlobalPermission,
    ) -> Result<GlobalPermission, ApiError> {
        let global_permission = self.global_permission(state.clone()).await?;
        let has_permission = global_permission.contains(permission)
            || global_permission.contains(GlobalPermission::Admin);
        if has_permission {
            Ok(global_permission)
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub async fn app_permission(
        &self,
        app_id: &str,
        state: &AppState,
    ) -> Result<AppPermissionResponse, ApiError> {
        let sub = self.sub();
        if let Ok(sub) = sub {
            let cached_permission = state.permission_cache.get(&sub);

            if let Some(role_model) = cached_permission {
                let permissions = RolePermissions::from_bits(role_model.permissions)
                    .ok_or_else(|| anyhow!("Invalid role permission bits"))?;
                return Ok(AppPermissionResponse {
                    state: state.clone(),
                    permissions,
                    role: role_model.clone(),
                    sub: Some(sub.clone()),
                    identifier: sub,
                });
            }

            let role_model = role::Entity::find()
                .join(JoinType::InnerJoin, role::Relation::Membership.def())
                .filter(
                    membership::Column::UserId
                        .eq(&sub)
                        .and(membership::Column::AppId.eq(app_id)),
                )
                .one(&state.db)
                .await?
                .ok_or_else(|| {
                    tracing::error!("Role not found for user {} in app {}", sub, app_id);
                    ApiError::from(anyhow!("Role not found for user {sub} in app {app_id}"))
                })?;

            let permissions = RolePermissions::from_bits(role_model.permissions)
                .ok_or_else(|| anyhow!("Invalid role permission bits"))?;

            state
                .permission_cache
                .insert(sub.clone(), Arc::new(role_model.clone()));

            return Ok(AppPermissionResponse {
                state: state.clone(),
                permissions,
                role: Arc::new(role_model),
                sub: Some(sub.clone()),
                identifier: sub,
            });
        }

        if let AppUser::APIKey(api_key) = self {
            let role_model = role::Entity::find()
                .join(JoinType::InnerJoin, role::Relation::TechnicalUser.def())
                .filter(
                    technical_user::Column::AppId
                        .eq(&api_key.app_id)
                        .and(technical_user::Column::Key.eq(&api_key.api_key)),
                )
                .one(&state.db)
                .await?
                .ok_or_else(|| ApiError::from(anyhow!("Technical user not found for API Key")))?;

            let permissions = RolePermissions::from_bits(role_model.permissions)
                .ok_or_else(|| anyhow!("Invalid role permission bits"))?;

            return Ok(AppPermissionResponse {
                state: state.clone(),
                permissions,
                role: Arc::new(role_model),
                sub: None,
                identifier: api_key.key_id.clone(),
            });
        }

        Err(ApiError::from(anyhow!(
            "User does not have app permissions"
        )))
    }
}

pub async fn jwt_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response<Body>, AuthorizationError> {
    let mut request = request;
    if let Some(auth_header) = request.headers().get(AUTHORIZATION)
        && let Ok(token) = auth_header.to_str()
    {
        let token = if token.starts_with("Bearer ") {
            &token[7..]
        } else {
            token
        };

        let token = token.trim();
        let claims = state.validate_token(token)?;
        let sub = claims.get("sub").ok_or(anyhow!("sub not found"))?;
        let sub = sub.as_str().ok_or(anyhow!("sub not a string"))?;
        let email = claims
            .get("email")
            .and_then(|v| v.as_str())
            .map(String::from);
        let username = claims
            .get("username")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                claims
                    .get("cognito:username")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
            .unwrap_or_else(|| sub.to_string());
        let preferred_username = claims
            .get("preferred_username")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| username.clone());
        let user = AppUser::OpenID(OpenIDUser {
            sub: sub.to_string(),
            username,
            email,
            preferred_username,
        });
        request.extensions_mut().insert::<AppUser>(user);
        return Ok(next.run(request).await);
    }

    if let Some(pat_header) = request.headers().get("x-pat")
        && let Ok(pat_str) = pat_header.to_str()
    {
        let db_pat = Pat::find()
            .filter(pat::Column::Key.eq(pat_str))
            .one(&state.db)
            .await?;
        if let Some(pat) = db_pat {
            let pat_user = AppUser::PAT(PATUser {
                pat: pat_str.to_string(),
                sub: pat.user_id.clone(),
            });
            request.extensions_mut().insert::<AppUser>(pat_user);
            return Ok(next.run(request).await);
        }
    }

    if let Some(api_key_header) = request.headers().get("x-api-key")
        && let Ok(api_key_str) = api_key_header.to_str()
    {
        let db_app = TechnicalUser::find()
            .filter(technical_user::Column::Key.eq(api_key_str))
            .one(&state.db)
            .await?;

        if let Some(app) = db_app {
            if let Some(valid_until) = app.valid_until {
                let now = chrono::Utc::now().naive_utc();
                if valid_until < now {
                    return Err(AuthorizationError::from(anyhow!("API Key is expired")));
                }
            }

            let app_user = AppUser::APIKey(ApiKey {
                key_id: app.id.clone(),
                api_key: api_key_str.to_string(),
                app_id: app.app_id.clone(),
            });
            request.extensions_mut().insert::<AppUser>(app_user);
            return Ok(next.run(request).await);
        }
    }

    request
        .extensions_mut()
        .insert::<AppUser>(AppUser::Unauthorized);
    Ok(next.run(request).await)
}
