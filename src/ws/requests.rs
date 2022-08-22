use serde::{Deserialize, Serialize};

/// Message that will be received from a client
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestMessage {
    #[serde(flatten)]
    pub request: Request,

    /// Nonce to send back when provided in a request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

/// All requests available to a WS client
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum Request {
    Auth(Auth),
    SetPassword(SetPassword),
    Me,
    Logout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Auth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPassword {
    pub password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let request = r#"{
            "type": "auth",
            "username": "test_user",
            "password": "hunter2"
        }"#;

        let parsed = serde_json::from_str::<RequestMessage>(request).unwrap();

        println!("{:#?}", parsed);

        let expected = RequestMessage {
            request: Request::Auth(Auth {
                username: "test_user".to_string(),
                password: "hunter2".to_string(),
            }),
            nonce: None,
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn error_while_parsing() {
        let request = r#"{
            "type": "nonexistingrequest",
        }"#;

        let parsed = serde_json::from_str::<Request>(request);

        println!("{:#?}", parsed);

        assert!(parsed.is_err());
    }

    #[test]
    fn request_with_nonce() {
        let request = r#"{
            "type": "auth",
            "username": "test_user",
            "password": "hunter2",
            "nonce": "randomnonce"
        }"#;

        let parsed = serde_json::from_str::<RequestMessage>(request).unwrap();

        println!("{:#?}", parsed);

        let expected = RequestMessage {
            request: Request::Auth(Auth {
                username: "test_user".to_string(),
                password: "hunter2".to_string(),
            }),
            nonce: Some("randomnonce".to_string()),
        };

        assert_eq!(expected, parsed);
    }
}
