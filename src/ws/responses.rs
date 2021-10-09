use serde::Serialize;

use crate::config;

/// Message that will be send to a client
#[derive(Serialize)]
pub struct ResponseMessage<'a> {
    #[serde(flatten)]
    pub response: Response<'a>,

    /// Nonce to send back when provided in a request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

/// All responses that are possible to be send to a client
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "data")]
pub enum Response<'a> {
    Error(ResponseError),
    SuccessfulLogin(SuccessfulLogin),
    SetPassword(SuccessfulLogin),
    Me(Me<'a>),
    UpdatedPassword,
    Logout,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "error", content = "reason")]
pub enum ResponseError {
    Deserialize(Option<String>),
    AuthFailed,
    AuthorizationRequired,
    AlreadyAuthenticated,
}

#[derive(Debug, Serialize)]
pub struct SuccessfulLogin {
    pub token: String,
}

#[derive(Serialize)]
pub struct Me<'a> {
    pub config: Config<'a>,
}

/// Config details that will be send in the response
#[derive(Serialize)]
pub struct Config<'a> {
    pub switcher: &'a config::Switcher,
    pub software: &'a config::SoftwareConnection,
    pub chat: &'a Option<config::Chat>,
    pub optional_scenes: &'a config::OptionalScenes,
    pub optional_options: &'a config::OptionalOptions,
}

impl<'a> Config<'a> {
    pub fn from(config: &'a config::Config) -> Self {
        Self {
            switcher: &config.switcher,
            software: &config.software,
            chat: &config.chat,
            optional_scenes: &config.optional_scenes,
            optional_options: &config.optional_options,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_response() {
        let message = ResponseMessage {
            response: Response::Error(ResponseError::AuthFailed),
            nonce: Some("1".to_string()),
        };

        let json = serde_json::to_string(&message).unwrap();
        println!("{}", json);

        let expected = r#"{"type":"error","data":{"error":"authFailed"},"nonce":"1"}"#;
        assert_eq!(expected, json);
    }

    #[test]
    fn error_response_with_reason() {
        let message = ResponseMessage {
            response: Response::Error(ResponseError::Deserialize(Some(
                "Couldn't deserialize...".to_string(),
            ))),
            nonce: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        println!("{}", json);

        let expected =
            r#"{"type":"error","data":{"error":"deserialize","reason":"Couldn't deserialize..."}}"#;
        assert_eq!(expected, json);
    }

    #[test]
    fn it_works() {
        let message = ResponseMessage {
            nonce: Some("2".to_string()),
            response: Response::SuccessfulLogin(SuccessfulLogin {
                token: "client token".to_string(),
            }),
        };

        let json = serde_json::to_string(&message).unwrap();
        println!("{}", json);

        let expected = r#"{"type":"successfulLogin","data":{"token":"client token"},"nonce":"2"}"#;
        assert_eq!(expected, json);
    }

    #[test]
    fn no_nonce() {
        let message = ResponseMessage {
            nonce: None,
            response: Response::SuccessfulLogin(SuccessfulLogin {
                token: "client token".to_string(),
            }),
        };

        let json = serde_json::to_string(&message).unwrap();
        println!("{}", json);

        let expected = r#"{"type":"successfulLogin","data":{"token":"client token"}}"#;
        assert_eq!(expected, json);
    }
}
