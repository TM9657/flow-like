use crate::state::AppState;

#[cfg(feature = "cognito")]
pub mod cognito;

pub enum UserManagement {
    #[cfg(feature = "cognito")]
    Cognito(cognito::CognitoUserManagement),
    None,
}

impl UserManagement {
    pub async fn new(state: &AppState) -> Self {
        #[cfg(feature = "cognito")]
        {
            return Self::from_cognito_configuration(
                state.platform_config.authentication.as_ref(),
                &state.aws_client,
            );
        }

        #[cfg(not(feature = "cognito"))]
        {
            let _ = state;
            UserManagement::None
        }
    }

    #[cfg(feature = "cognito")]
    fn from_cognito_configuration(
        authentication: Option<&flow_like::hub::Authentication>,
        sdk: &aws_config::SdkConfig,
    ) -> Self {
        let Some(config) = authentication
            .and_then(|auth| auth.openid.as_ref())
            .and_then(|openid| openid.cognito.as_ref())
        else {
            return Self::None;
        };
        Self::Cognito(cognito::CognitoUserManagement::new(sdk, config))
    }

    pub async fn get_attribute(
        &self,
        sub: &str,
        username: &Option<String>,
        attribute: &str,
    ) -> flow_like_types::Result<Option<String>> {
        match self {
            #[cfg(feature = "cognito")]
            UserManagement::Cognito(cognito) => {
                cognito.get_attribute(sub, username, attribute).await
            }
            UserManagement::None => {
                let _ = (sub, username, attribute);
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "cognito")]
    #[tokio::test]
    async fn user_management_selects_only_configured_cognito() {
        use super::UserManagement;
        use flow_like::hub::{Authentication, CognitoConfig};

        let sdk = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_cognitoidentityprovider::config::Region::new(
                "eu-west-1",
            ))
            .build();
        let without_openid = Authentication {
            variant: "oauth2".into(),
            openid: None,
            oauth2: None,
        };
        let mut oidc: Authentication = serde_json::from_value(serde_json::json!({
            "variant": "openid",
            "openid": {"jwks_url": "https://identity.example.test/keys"}
        }))
        .unwrap();
        for configuration in [None, Some(&without_openid), Some(&oidc)] {
            let manager = UserManagement::from_cognito_configuration(configuration, &sdk);
            assert!(matches!(manager, UserManagement::None));
            assert_eq!(
                manager.get_attribute("user", &None, "email").await.unwrap(),
                None
            );
        }
        oidc.openid.as_mut().unwrap().cognito = Some(CognitoConfig {
            user_pool_id: "eu-west-1_example".into(),
        });
        let manager = UserManagement::from_cognito_configuration(Some(&oidc), &sdk);
        assert!(matches!(manager, UserManagement::Cognito(_)));
        assert_eq!(
            manager.get_attribute("user", &None, "email").await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn unconfigured_user_management_leaves_provider_attributes_optional() {
        assert_eq!(
            super::UserManagement::None
                .get_attribute("user", &None, "email")
                .await
                .unwrap(),
            None,
        );
    }
}
