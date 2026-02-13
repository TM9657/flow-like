use crate::state::AppState;

#[cfg(feature = "cognito")]
pub mod cognito;

pub enum UserManagement {
    #[cfg(feature = "cognito")]
    Cognito(cognito::CognitoUserManagement),
    #[cfg(not(feature = "cognito"))]
    None,
}

impl UserManagement {
    pub async fn new(state: &AppState) -> Self {
        #[cfg(feature = "cognito")]
        return UserManagement::Cognito(
            cognito::CognitoUserManagement::new(state)
                .await
                .expect("Failed to create Cognito user management"),
        );

        #[cfg(not(feature = "cognito"))]
        {
            let _ = state;
            UserManagement::None
        }
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
            #[cfg(not(feature = "cognito"))]
            UserManagement::None => {
                let _ = (sub, username, attribute);
                Ok(None)
            }
        }
    }
}
