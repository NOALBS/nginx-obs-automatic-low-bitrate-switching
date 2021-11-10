use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time;
use tracing::{debug, info};

use crate::chat::{self, HandleMessage};
use crate::{config, error, events, state, switcher, user_manager, Noalbs};

pub struct ChatHandler {
    chat_handler_rx: mpsc::Receiver<super::HandleMessage>,
    user_manager: user_manager::UserManager,
    chat_senders: HashMap<chat::ChatPlatform, Arc<dyn chat::ChatLogic>>,

    timeouts: HashMap<chat::ChatPlatform, Vec<Timeout>>,
    default_commands: HashMap<chat::Command, config::CommandInfo>,
}

impl ChatHandler {
    pub fn new(
        chat_handler_rx: mpsc::Receiver<super::HandleMessage>,
        user_manager: user_manager::UserManager,
    ) -> Self {
        let mut timeouts = HashMap::new();
        timeouts.insert(chat::ChatPlatform::Twitch, Vec::new());

        Self {
            chat_handler_rx,
            user_manager,
            chat_senders: HashMap::new(),
            default_commands: Self::default_command_settings(),
            timeouts,
        }
    }

    // TODO: Should there also be default alias?
    fn default_command_settings() -> HashMap<chat::Command, config::CommandInfo> {
        let mut default = HashMap::new();

        use chat::Command;
        use chat::Permission;

        default.insert(
            Command::Switch,
            config::CommandInfo {
                permission: None,
                alias: None,
            },
        );

        default.insert(
            Command::Bitrate,
            config::CommandInfo {
                permission: Some(Permission::Public),
                alias: None,
            },
        );

        default.insert(
            Command::Fix,
            config::CommandInfo {
                permission: Some(Permission::Mod),
                alias: None,
            },
        );

        default.insert(
            Command::Obsinfo,
            config::CommandInfo {
                permission: Some(Permission::Mod),
                alias: None,
            },
        );

        default.insert(
            Command::Otrigger,
            config::CommandInfo {
                permission: Some(Permission::Mod),
                alias: None,
            },
        );

        default.insert(
            Command::Refresh,
            config::CommandInfo {
                permission: Some(Permission::Mod),
                alias: None,
            },
        );

        default.insert(
            Command::Rtrigger,
            config::CommandInfo {
                permission: Some(Permission::Mod),
                alias: None,
            },
        );

        default.insert(
            Command::Sourceinfo,
            config::CommandInfo {
                permission: Some(Permission::Mod),
                alias: None,
            },
        );

        default.insert(
            Command::Trigger,
            config::CommandInfo {
                permission: Some(Permission::Mod),
                alias: None,
            },
        );

        default.insert(
            Command::Version,
            config::CommandInfo {
                permission: Some(Permission::Public),
                alias: None,
            },
        );

        default
    }

    pub fn add_chat_sender(
        &mut self,
        platform: chat::ChatPlatform,
        connection: Arc<dyn chat::ChatLogic>,
    ) {
        self.chat_senders.insert(platform, connection);
    }

    pub async fn handle_messages(&mut self) {
        while let Some(message) = self.chat_handler_rx.recv().await {
            debug!("Handle this message: {:?}", message);

            match message {
                HandleMessage::ChatMessage(msg) => {
                    if msg.message.is_empty() {
                        continue;
                    }

                    self.handle_chat_message(msg).await;
                }
                HandleMessage::StartedHosting(host) => {
                    self.handle_hosting(host).await;
                }
                HandleMessage::AutomaticSwitchingScene(ss) => {
                    self.handle_automatic_switching_message(ss).await;
                }
            }
        }
    }

    pub async fn handle_automatic_switching_message(
        &self,
        ss: chat::AutomaticSwitchingScene,
    ) -> Option<()> {
        let sender = self.chat_senders.get(&ss.platform)?;

        let mut msg = format!(r#"Scene switched to "{}""#, ss.scene);

        use switcher::SwitchType::*;
        match ss.switch_type {
            Normal | Low => {
                let user = self
                    .user_manager
                    .get_user_by_chat_platform(&ss.channel, &ss.platform)
                    .await?;

                let bitrate = bitrate_msg(&user).await;
                msg += &format!(" | {}", bitrate);
            }
            Previous | Offline => {}
        }

        sender.send_message(ss.channel, msg).await;

        Some(())
    }

    pub async fn handle_chat_message(&mut self, msg: chat::ChatMessage) -> Option<()> {
        let user = self
            .user_manager
            .get_user_by_chat_platform(&msg.channel, &msg.platform)
            .await?;

        let command = self.get_command(&user, &msg).await?;

        if msg.permission == chat::Permission::Public
            && self.handle_timeout(&msg.platform, &msg.channel).await
        {
            debug!("Timeout");
            return None;
        }

        match command {
            chat::Command::Unknown(_) => {}
            _ => {
                info!(
                    "[{}] {} executed command: {:?}",
                    msg.channel, msg.sender, command
                );
            }
        }

        let dc = DispatchCommand {
            user: user.clone(),
            chat_sender: self.chat_senders.get(&msg.platform)?.clone(),
            command,
            chat_message: msg,
        };

        tokio::spawn(async move { dc.run_command().await });

        Some(())
    }

    // TODO: refactor this
    /// Returns the command if allowed
    async fn get_command(&self, user: &Noalbs, msg: &chat::ChatMessage) -> Option<chat::Command> {
        let state = user.state.read().await;
        let chat = state.config.chat.as_ref()?;
        let prefix = &chat.prefix;

        let mut message = msg.message.split_ascii_whitespace();
        let command = message.next().unwrap().strip_prefix(prefix)?;
        let mut command = super::Command::from(command);

        if let super::Command::Unknown(ref cmd) = command {
            if let Some(cmd_from_alias) =
                try_get_command_from_alias(&chat.commands, &self.default_commands, &cmd)
            {
                command = cmd_from_alias;
            }
        }

        debug!("Found command: {:?}", command);

        if !(msg.permission == chat::Permission::Admin || chat.admins.contains(&msg.sender)) {
            debug!("Not an admin checking permission");

            if !is_allowed_to_run_command(
                &chat.commands,
                &self.default_commands,
                &command,
                &msg.permission,
            ) {
                debug!("{} not allowed to use command {:?}", msg.sender, command);
                return None;
            }
        }

        Some(command)
    }

    pub async fn handle_hosting(&self, host: chat::StartedHosting) -> Option<()> {
        let user = self
            .user_manager
            .get_user_by_chat_platform(&host.channel, &host.platform)
            .await?;

        let state = user.state.read().await;

        if !state
            .config
            .chat
            .as_ref()?
            .enable_auto_stop_stream_on_host_or_raid
        {
            return None;
        }

        let bs = &state.broadcasting_software;
        if !bs.is_streaming || bs.last_stream_started_at.elapsed().as_secs() < 60 {
            return None;
        }

        info!(
            "Channel started hosting, stopping the stream ({:?}) {}",
            host.platform, host.channel
        );

        let dc = DispatchCommand {
            user: user.clone(),
            chat_sender: self.chat_senders.get(&host.platform)?.clone(),
            command: chat::Command::Stop,
            chat_message: chat::ChatMessage {
                platform: host.platform,
                permission: chat::Permission::Admin,
                channel: host.channel,
                sender: "NOALBSbot".to_string(),
                message: "Hi your channel started hosting and I would like to stop it for you"
                    .to_string(),
            },
        };

        tokio::spawn(async move { dc.run_command().await });

        Some(())
    }

    // TODO: Maybe remove when timeout passed
    pub async fn handle_timeout(&mut self, platform: &chat::ChatPlatform, channel: &str) -> bool {
        let platform_timeouts = self.timeouts.get_mut(&platform).unwrap();
        let channel_timeout = platform_timeouts.iter_mut().find(|x| x.channel == channel);

        if let Some(timeout) = channel_timeout {
            let delta = timeout.time.elapsed().as_secs();

            if delta <= 5 {
                return true;
            } else {
                timeout.time = std::time::Instant::now();
            }
        } else {
            platform_timeouts.push(Timeout {
                channel: channel.to_owned(),
                time: std::time::Instant::now(),
            });
        }

        false
    }
}

fn is_allowed_to_run_command(
    user_commands: &Option<HashMap<chat::Command, config::CommandInfo>>,
    default_commands: &HashMap<chat::Command, config::CommandInfo>,
    command: &chat::Command,
    user_permission: &chat::Permission,
) -> bool {
    if let Some(command) = match user_commands {
        Some(user_commands) => user_commands.get(&command),
        None => None,
    } {
        if let Some(permission) = &command.permission {
            if permission_is_allowed(&permission, &user_permission) {
                return true;
            }
        }
    }

    if let Some(command) = default_commands.get(&command) {
        if let Some(permission) = &command.permission {
            if permission_is_allowed(&permission, &user_permission) {
                return true;
            }
        }
    }

    false
}

fn permission_is_allowed(
    permission: &chat::Permission,
    user_permission: &chat::Permission,
) -> bool {
    permission == &chat::Permission::Public
        || (permission == &chat::Permission::Mod && user_permission == &chat::Permission::Mod)
}

fn try_get_command_from_alias(
    user_commands: &Option<HashMap<chat::Command, config::CommandInfo>>,
    default_commands: &HashMap<chat::Command, config::CommandInfo>,
    potential_command: &str,
) -> Option<chat::Command> {
    // check if user defined alias
    if let Some(user_cmd) = user_commands {
        if let Some(cmd) = get_command_from_alias_string(user_cmd, &potential_command) {
            return Some(cmd);
        }
    }

    if let Some(cmd) = get_command_from_alias_string(&default_commands, &potential_command) {
        return Some(cmd);
    }

    // TODO: check if platform specific?

    None
}

// TODO: This could be better
pub fn get_command_from_alias_string(
    commands: &HashMap<chat::Command, config::CommandInfo>,
    alias: &str,
) -> Option<chat::Command> {
    commands.iter().find_map(|(key, value)| {
        if let Some(aliases) = &value.alias {
            if aliases.iter().any(|x| x == alias) {
                return Some(key.to_owned());
            }
        }

        None
    })
}

pub struct DispatchCommand {
    user: Arc<Noalbs>,
    chat_sender: Arc<dyn chat::ChatLogic>,
    command: chat::Command,
    chat_message: chat::ChatMessage,
}

impl DispatchCommand {
    pub async fn run_command(&self) {
        let mut params = self.chat_message.message.split_whitespace();
        params.next();

        match self.command {
            chat::Command::Alias => self.alias(params).await,
            chat::Command::Autostop => self.autostop(params.next()).await,
            chat::Command::Bitrate => self.bitrate().await,
            chat::Command::Fix => self.fix().await,
            chat::Command::Refresh => self.fix().await,
            chat::Command::Noalbs => self.noalbs(params.next(), params).await,
            chat::Command::Notify => self.notify(params.next()).await,
            chat::Command::Rec => self.record().await,
            chat::Command::Start => self.start().await,
            chat::Command::Stop => self.stop().await,
            chat::Command::Switch => self.switch(params.next()).await,
            chat::Command::Trigger => {
                self.trigger(switcher::TriggerType::Low, params.next())
                    .await
            }
            chat::Command::Otrigger => {
                self.trigger(switcher::TriggerType::Offline, params.next())
                    .await
            }
            chat::Command::Rtrigger => {
                self.trigger(switcher::TriggerType::Rtt, params.next())
                    .await
            }
            chat::Command::Version => self.version().await,
            chat::Command::PrivacyScene => self.privacy_scene().await,
            chat::Command::StartingScene => self.starting_scene().await,
            chat::Command::EndingScene => self.ending_scene().await,
            chat::Command::LiveScene => self.live_scene().await,
            chat::Command::Obsinfo => {}
            chat::Command::Mod => {}
            chat::Command::Public => {}
            chat::Command::Sourceinfo => self.source_info(params.next()).await,
            chat::Command::Unknown(_) => {}
        };
    }

    async fn alias<'a, I>(&self, args: I)
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut args = args.into_iter();
        let a1 = args.next();
        let a2 = args.next();

        if a1.is_none() || a2.is_none() {
            self.send("Error incorrect arguments given".to_string())
                .await;
            return;
        }

        let a1 = a1.unwrap();
        let a2 = a2.unwrap();

        // remove alias
        if a1 == "rem" {
            if !&self.user.contains_alias(a2).await.unwrap() {
                self.send(format!("Alias {} doesn't exist", a2)).await;
                return;
            }

            if let Ok(success) = self.user.remove_alias(a2).await {
                if success {
                    self.user.save_config().await;
                    self.send(format!("Alias {} removed", a2)).await;
                }
            }

            return;
        }

        // add alias
        if self.user.contains_alias(a1).await.unwrap() {
            self.send(format!("{} already used as alias", a1)).await;
            return;
        }

        let command = super::Command::from(a2);

        if let chat::Command::Unknown(_) = command {
            self.send(format!("Command {} doesn't exist", a2)).await;
            return;
        }

        if self.user.add_alias(a1.to_string(), command).await.is_ok() {
            self.user.save_config().await;
            self.send(format!("Added alias {} -> {}", a1, a2)).await;
        }
    }

    async fn bitrate(&self) {
        let msg = bitrate_msg(&self.user).await;

        self.send(msg).await;
    }

    // TODO: more than one word?
    async fn switch(&self, name: Option<&str>) {
        let name = match name {
            Some(name) => name,
            None => {
                self.send("No scene specified".to_string()).await;
                return;
            }
        };

        let state = self.user.state.read().await;
        let bsc = &state.broadcasting_software.connection;

        if bsc.is_none() {
            return;
        }

        let msg = match bsc.as_ref().unwrap().switch_scene(name).await {
            Ok(scene) => format!("Scene successfully switched to \"{}\"", scene),
            Err(e) => format!("Error can't switch to scene \"{}\", {}", name, e),
        };

        self.send(msg).await;
    }

    async fn start(&self) {
        let state = self.user.state.read().await;

        let bsc = match &state.broadcasting_software.connection {
            Some(b) => b,
            None => return,
        };

        let options = &state.config.optional_options;

        if self.chat_message.platform == chat::ChatPlatform::Twitch
            && options.twitch_transcoding_check
        {
            self.send("Trying to start the stream with transcoding".to_string())
                .await;

            let retry = &options.twitch_transcoding_retries;
            let delay = &options.twitch_transcoding_delay_seconds;

            let mut attempts = 0;

            for i in 0..*retry {
                debug!("[{}] Starting stream", i);
                if let Err(e) = bsc.start_streaming().await {
                    self.send(format!("Error can't start the stream, {}", e))
                        .await;
                    return;
                };

                time::sleep(time::Duration::from_secs(*delay)).await;

                if let Ok(true) = check_if_transcoding(&self.chat_message.channel).await {
                    attempts = i + 1;
                    break;
                }

                if i == retry - 1 {
                    debug!("[{}] Can't get transcoding", i);
                    self.send("Successfully started the stream without transcoding".to_string())
                        .await;
                    return;
                }

                debug!("[{}] Stopping stream", i);
                if let Err(e) = bsc.stop_streaming().await {
                    self.send(format!("Error can't start the stream, {}", e))
                        .await;
                    return;
                };

                time::sleep(time::Duration::from_secs(5)).await;
            }

            let mut msg = "Started stream with transcoding".to_string();

            if attempts > 1 {
                msg += &format!(", took {} attempts", attempts,);
            }

            self.send(msg).await;

            return;
        }

        let msg = match bsc.start_streaming().await {
            Ok(_) => "Successfully started the stream".to_string(),
            Err(e) => format!("Error can't start the stream, {}", e),
        };

        self.send(msg).await;
    }

    async fn stop(&self) {
        let state = self.user.state.read().await;

        let bsc = match &state.broadcasting_software.connection {
            Some(b) => b,
            None => return,
        };

        let msg = match bsc.stop_streaming().await {
            Ok(_) => "Successfully stopped the stream".to_string(),
            Err(e) => format!("Error can't stop the stream, {}", e),
        };

        self.send(msg).await;
    }

    async fn trigger(&self, kind: switcher::TriggerType, value_string: Option<&str>) {
        let value = match value_string {
            Some(name) => name,
            None => {
                let msg = match &self.user.get_trigger_by_type(kind).await {
                    Some(bitrate) => format!("Current trigger set at {} Kbps", bitrate),
                    None => "Current trigger is disabled".to_string(),
                };

                self.send(msg).await;
                return;
            }
        };

        let value = match value.parse::<u32>() {
            Ok(v) => v,
            Err(_) => {
                let msg = format!("Error editing trigger {} is not a valid value", value);
                self.send(msg).await;
                return;
            }
        };

        let msg = match &self.user.update_trigger(kind, value).await {
            Some(value) => format!("Trigger successfully set to {:?} Kbps", value),
            None => "Trigger successfully disabled".to_string(),
        };

        self.user.save_config().await;
        self.send(msg).await;
    }

    async fn notify(&self, enabled: Option<&str>) {
        if let Some(enabled) = enabled {
            if let Ok(b) = enabled_to_bool(enabled) {
                self.user.set_notify(b).await;
                self.user.save_config().await;
            }
        }

        let msg = generate_enable_string("Auto switch notification", self.user.get_notify().await);
        self.send(msg).await;
    }

    async fn autostop(&self, enabled: Option<&str>) {
        if let Some(enabled) = enabled {
            if let Ok(b) = enabled_to_bool(enabled) {
                self.user.set_autostop(b).await.unwrap();
                self.user.save_config().await;
            }
        }

        let msg =
            generate_enable_string("Auto stop stream", self.user.get_autostop().await.unwrap());
        self.send(msg).await;
    }

    async fn fix(&self) {
        let state = self.user.state.read().await;

        let bsc = match &state.broadcasting_software.connection {
            Some(b) => b,
            None => return,
        };

        let msg = match bsc.fix().await {
            Ok(_) => "Trying to fix the stream".to_string(),
            Err(_) => "Error fixing the stream".to_string(),
        };

        self.send(msg).await;
    }

    // Record is a toggle
    async fn record(&self) {
        let state = self.user.state.read().await;

        let bsc = match &state.broadcasting_software.connection {
            Some(b) => b,
            None => return,
        };

        let is_recording = match bsc.is_recording().await {
            Ok(status) => status,
            Err(_) => {
                self.send("Error getting recording status".to_string())
                    .await;
                return;
            }
        };

        if bsc.toggle_recording().await.is_err() {
            self.send("Error toggling recording".to_string()).await;
            return;
        }

        if is_recording {
            self.send("Recording stopped".to_string()).await;
            return;
        }

        self.send("Recording started".to_string()).await;
    }

    pub async fn version(&self) {
        let msg = format!("Running NOALBS v{}", crate::VERSION);
        self.send(msg).await;
    }

    pub async fn noalbs<'a, I>(&self, command: Option<&str>, args: I)
    where
        I: IntoIterator<Item = &'a str>,
    {
        let command = match command {
            Some(command) => command,
            None => return,
        };

        let mut args = args.into_iter();

        let msg = match command {
            "version" => {
                self.version().await;
                return;
            }
            "prefix" => {
                if let Some(prefix) = args.next() {
                    let _ = self.user.set_prefix(prefix.to_owned()).await;
                    self.user.save_config().await;

                    self.user
                        .send_event(events::Event::PrefixChanged { prefix })
                        .await;

                    format!("NOALBS prefix updated to {}", prefix)
                } else {
                    "Can't update NOALBS prefix".to_string()
                }
            }
            "start" => {
                self.user.set_bitrate_switcher_state(true).await;
                self.user.save_config().await;
                "Successfully enabled the switcher".to_string()
            }
            "stop" => {
                self.user.set_bitrate_switcher_state(false).await;
                self.user.save_config().await;
                "Successfully disabled the switcher".to_string()
            }
            "instant" => {
                let toggle = self.user.set_instantly_switch_on_recover().await;
                self.user.save_config().await;
                generate_enable_string("Instant switch on recover", toggle)
            }
            "retry" => self.set_retry_attempts(args.next()).await,
            _ => String::new(),
        };

        if !msg.is_empty() {
            self.send(msg).await;
        }
    }

    async fn set_retry_attempts(&self, value_string: Option<&str>) -> String {
        let value = match value_string {
            Some(name) => name,
            None => {
                let current_attempts = &self.user.get_retry_attempts().await;

                return format!("Current retry set at {}", current_attempts);
            }
        };

        let value = match value.parse::<u8>() {
            Ok(v) => v,
            Err(_) => {
                return format!(
                    "Error editing retry attempts {} is not a valid value",
                    value
                );
            }
        };

        self.user.set_retry_attempts(value).await;
        self.user.save_config().await;

        format!("Retry attempts set to {}", value)
    }

    async fn privacy_scene(&self) {
        let state = self.user.state.read().await;
        if let Some(scene) = &state.config.optional_scenes.privacy {
            self.send("Switching to privacy scene".to_string()).await;
            self.switch(Some(&scene)).await;
        } else {
            self.send("No privacy scene set".to_string()).await;
        }
    }

    async fn starting_scene(&self) {
        let state = self.user.state.read().await;
        if let Some(scene) = &state.config.optional_scenes.starting {
            self.send("Switching to starting scene".to_string()).await;
            self.switch(Some(&scene)).await;
        } else {
            self.send("No starting scene set".to_string()).await;
        }
    }

    async fn ending_scene(&self) {
        let state = self.user.state.read().await;
        if let Some(scene) = &state.config.optional_scenes.ending {
            self.send("Switching to ending scene".to_string()).await;
            self.switch(Some(&scene)).await;
        } else {
            self.send("No ending scene set".to_string()).await;
        }
    }

    // TODO: Actually switch to the right scene
    async fn live_scene(&self) {
        let state = self.user.state.read().await;
        let scene = &state.config.switcher.switching_scenes.normal;

        self.send("Switching to live scene".to_string()).await;
        self.switch(Some(&scene)).await;
    }

    async fn source_info(&self, server_name: Option<&str>) {
        let state = &self.user.state.read().await;
        let stream_servers = &state.config.switcher.stream_servers;

        let no_info = "No information".to_string();

        if let Some(name) = server_name {
            let server = match stream_servers.iter().find(|s| s.name == name) {
                Some(s) => s,
                None => {
                    let msg = format!("Error no server found with the name: {}", name);
                    self.send(msg).await;

                    return;
                }
            };

            let info = match server.stream_server.source_info().await {
                Some(i) => i,
                None => no_info,
            };
            self.send(format!("{}: {}", name, info)).await;

            return;
        }

        let mut msg = Vec::new();

        for s in stream_servers {
            let info = s.stream_server.source_info().await;

            if let Some(info) = info {
                msg.push(format!("{}: {}", s.name, info));
            }
        }

        if msg.is_empty() {
            self.send(no_info).await;

            return;
        }

        self.send(msg.join(" - ")).await;
    }

    async fn send(&self, message: String) {
        self.chat_sender
            .send_message(self.chat_message.channel.to_owned(), message)
            .await;
    }
}

fn generate_enable_string(res: &str, condition: bool) -> String {
    format!(
        "{} is {}",
        res,
        if condition { "enabled" } else { "disabled" }
    )
}

fn enabled_to_bool(enabled: &str) -> Result<bool, error::Error> {
    if enabled.to_lowercase() == "on" {
        return Ok(true);
    }

    if enabled.to_lowercase() == "off" {
        return Ok(false);
    }

    Err(error::Error::EnabledToBoolConversionError)
}

async fn bitrate_msg(user: &Noalbs) -> String {
    let mut msg = String::new();

    let state = &user.state.read().await;
    let servers = &state.config.switcher.stream_servers;

    for (i, s) in servers.iter().enumerate() {
        let t = s.stream_server.bitrate().await;
        let sep = if i == 0 || msg.is_empty() { "" } else { " - " };

        if let Some(bitrate_message) = t.message {
            msg += &format!("{}{}: {}", sep, s.name, bitrate_message);
        }
    }

    if msg.is_empty() {
        return "No connection :(".to_string();
    }

    msg
}

#[derive(Debug)]
pub struct Timeout {
    pub channel: String,
    pub time: std::time::Instant,
}

const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
const USHER_BASE: &str = "https://usher.ttvnw.net";
const GQL_BASE: &str = "https://gql.twitch.tv/gql";

// TODO: Check if not an ad?
async fn check_if_transcoding(channel: &str) -> Result<bool, error::Error> {
    let req_string = r#"{"query": "{streamPlaybackAccessToken(channelName: \"%USER%\",params: {platform: \"web\",playerBackend: \"mediaplayer\",playerType: \"site\"}){value signature}}"}"#;
    let req_string = req_string.replace("%USER%", channel);

    let client = reqwest::Client::new();
    let res = client
        .post(GQL_BASE)
        .header("Client-ID", CLIENT_ID)
        .body(req_string)
        .send()
        .await?;

    let json = res.json::<serde_json::Value>().await?;
    let json = json["data"]["streamPlaybackAccessToken"].to_owned();
    let json: StreamPlaybackAccessToken = serde_json::from_value(json)?;

    use rand::Rng;
    let rng = rand::thread_rng().gen_range(1000000..10000000);
    let query = M3u8Query {
        allow_source: String::from("true"),
        allow_audio_only: String::from("true"),
        allow_spectre: String::from("true"),
        p: rng,
        player: String::from("twitchweb"),
        playlist_include_framerate: String::from("true"),
        segment_preference: String::from("4"),
        sig: json.signature,
        token: json.value,
    };

    let res = client
        .get(format!("{}/api/channel/hls/{}.m3u8", USHER_BASE, channel))
        .header("Client-ID", CLIENT_ID)
        .query(&query)
        .send()
        .await?;

    let text = res.text().await?;
    // println!("Response:\n{}", text);

    if text.contains("TRANSCODESTACK=\"transmux\"")
        || text.contains("Can not find channel")
        || text.contains("transcode_does_not_exist")
    {
        return Ok(false);
    }

    Ok(true)
}

#[derive(Debug, Serialize)]
struct M3u8Query {
    allow_source: String,
    allow_audio_only: String,
    allow_spectre: String,
    p: u32,
    player: String,
    playlist_include_framerate: String,
    segment_preference: String,
    sig: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct StreamPlaybackAccessToken {
    value: String,
    signature: String,
}
