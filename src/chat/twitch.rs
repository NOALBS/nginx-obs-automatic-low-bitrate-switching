pub struct Twitch {}

impl Twitch {
    pub fn send_message(&self, message: &str) {
        println!("sending message: {}", message);
    }
}
