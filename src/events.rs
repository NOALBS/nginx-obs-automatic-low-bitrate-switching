use serde::Serialize;

/// All events that might be send
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "event", content = "data")]
pub enum Event<'a> {
    PrefixChanged { prefix: &'a str },
    SceneSwitched { scene: &'a str },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event() {
        let event = Event::PrefixChanged { prefix: "!" };

        let json = serde_json::to_string(&event).unwrap();
        println!("{}", json);

        let expected = r#"{"event":"prefixChanged","data":{"prefix":"!"}}"#;
        assert_eq!(expected, json);
    }
}
