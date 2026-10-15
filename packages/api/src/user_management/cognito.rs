use flow_like::hub::CognitoConfig;

pub struct CognitoUserManagement {
    client: aws_sdk_cognitoidentityprovider::Client,
    pool_id: String,
}

impl CognitoUserManagement {
    pub fn new(sdk: &aws_config::SdkConfig, config: &CognitoConfig) -> Self {
        let client = aws_sdk_cognitoidentityprovider::Client::new(sdk);
        Self {
            client,
            pool_id: config.user_pool_id.clone(),
        }
    }

    pub async fn get_attribute(
        &self,
        _sub: &str,
        username: &Option<String>,
        attribute: &str,
    ) -> flow_like_types::Result<Option<String>> {
        let username = match username {
            Some(name) => name.clone(),
            None => return Ok(None),
        };

        let user = self
            .client
            .admin_get_user()
            .user_pool_id(self.pool_id.clone())
            .username(username)
            .send()
            .await?;

        if let Some(attributes) = user.user_attributes {
            for attr in attributes {
                if attr.name == attribute {
                    return Ok(attr.value);
                }
            }
        }

        Ok(None)
    }
}
