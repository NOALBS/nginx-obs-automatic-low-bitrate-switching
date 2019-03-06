import WebSocket from "ws";
import config from "../../config";
import fs from "fs";
import signale from "signale";

signale.config({
    displayTimestamp: true,
    displayDate: true
});

const log = signale.scope("CHT");

class Chat {
    constructor(username, password, channel, obs) {
        this.username = username; // username
        this.password = password; // oauth
        this.channel = `#${channel}`; // #channel
        this.obsProps = obs;
        this.obs = obs.obs;
        this.prefix = config.twitchChat.prefix;
        this.commands = [
            "host",
            "unhost",
            "start",
            "stop",
            "switch",
            "raid",
            "bitrate",
            "refresh",
            "trigger",
            "sourceinfo",
            "obsinfo",
            "public",
            "mod",
            "notify",
            "autostop",
            "rec"
        ];
        this.allowAllCommands = config.twitchChat.publicCommands;
        this.allowModsCommands = config.twitchChat.modCommands;
        this.wait = false;
        this.rate = 0;
        this.rateInterval = false;
        this.isRefreshing = false;

        this.open();

        this.obsProps.on("live", this.live.bind(this));
        this.obsProps.on("normalScene", this.onNormalScene.bind(this));
        this.obsProps.on("lowBitrateScene", this.onLowBitrateScene.bind(this));
        this.obsProps.on("offlineScene", this.onOfflineScene.bind(this));

        log.info("Connecting to twitch");
    }

    open() {
        this.ws = new WebSocket("wss://irc-ws.chat.twitch.tv:443");

        this.ws.onopen = this.onOpen.bind(this);
        this.ws.onmessage = this.onMessage.bind(this);
        this.ws.onerror = this.onError.bind(this);
        this.ws.onclose = this.onClose.bind(this);
    }

    keepAlive() {
        this.interval = setInterval(() => {
            this.ws.send("PING :tmi.twitch.tv\r\n");
        }, 2000);
    }

    onOpen() {
        if (this.ws !== null && this.ws.readyState === 1) {
            log.success("Successfully Connected");
            log.success(`Authenticating and joining channel ${this.channel}`);

            this.ws.send("CAP REQ :twitch.tv/tags twitch.tv/commands");
            this.ws.send(`PASS ${this.password}`);
            this.ws.send(`NICK ${this.username}`);
            this.ws.send(`JOIN ${this.channel}`);

            this.keepAlive();
        }
    }

    onClose() {
        log.error("Disconnected from twitch server");
        clearInterval(this.interval);
        this.ws.removeAllListeners();
        this.reconnect();
    }

    close() {
        if (this.ws) {
            this.ws.close();
        }
    }

    reconnect() {
        log.info(`Trying to reconnect in 5 seconds`);

        setTimeout(() => {
            log.info("Reconnecting...");
            this.open();
        }, 5000);
    }

    onError(e) {
        log.error(new Error(e));
    }

    onMessage(message) {
        if (message !== null) {
            const parsed = this.parse(message.data);

            if (parsed !== null) {
                if (parsed.command === "PRIVMSG") {
                    // not a command
                    if (parsed.message.substr(0, 1) !== this.prefix) return;

                    // Split the message into individual words:
                    const parse = parsed.message.slice(1).split(" ");
                    const commandName = parse[0];

                    if (
                        (config.twitchChat.adminUsers.includes(parsed.username) && this.rate != 20) ||
                        (config.twitchChat.enablePublicCommands && this.allowAllCommands.includes(commandName) && !this.wait && this.rate != 20) ||
                        (config.twitchChat.enableModCommands &&
                            parsed.tags.mod === "1" &&
                            this.allowModsCommands.includes(commandName) &&
                            this.rate != 20) ||
                        (parsed.username === this.channel.substring(1) && this.rate != 20)
                    ) {
                        if (this.commands.includes(commandName)) {
                            this[commandName](parse[1]);

                            log.success(`${parsed.username} Executed ${commandName} command`);
                            this.setWait();
                        } else {
                            log.error(`${parsed.username} Executed unknown command ${commandName}`);
                        }
                    }
                } else if (parsed.command === "PING") {
                    this.ws.send(`PONG :${parsed.message}`);
                } else if (parsed.command === "HOSTTARGET") {
                    if (parsed.message != null && config.twitchChat.enableAutoStopStreamOnHostOrRaid && this.obsProps.bitrate != null) {
                        log.info("Channel started hosting, stopping stream");
                        this.stop();
                    }
                }
            }
        }
    }

    parse(message) {
        const regex = RegExp(/([A-Z]\w*)/, "g");
        const array = regex.exec(message);

        let parsedMessage = {
            tags: {},
            channel: null,
            command: null,
            username: null,
            message: null,
            raw: message
        };

        const firstString = message.split(" ", 1)[0];

        if (message[0] === "@") {
            var space = message.indexOf(" ");
            const tagsRaw = message.slice(1, space);
            const tagsSplit = tagsRaw.split(";");

            tagsSplit.map(d => {
                const tagSplit = d.split("=");
                parsedMessage.tags[tagSplit[0]] = tagSplit[1];
            });

            const userIndex = message.indexOf("!");
            parsedMessage.username = message.slice(space + 2, userIndex);

            const commandIndex = message.indexOf(" ", userIndex);
            const channelIndex = message.indexOf("#", space);

            parsedMessage.command = message.slice(commandIndex + 1, channelIndex - 1);
            const messageIndex = message.indexOf(":", commandIndex);

            parsedMessage.channel = message.slice(channelIndex, messageIndex - 1);
            parsedMessage.message = message.slice(messageIndex + 1, message.length - 2);
        } else if (firstString === "PING") {
            parsedMessage.command = "PING";
            parsedMessage.message = message.split(":")[1];
        } else if (array[0] == "HOSTTARGET") {
            const res = message.match(/:([\w]+)/g);

            parsedMessage.command = "HOSTTARGET";
            parsedMessage.message = res[1];
        }

        return parsedMessage;
    }

    setWait() {
        this.rate++;

        if (!this.rateInterval) {
            this.rateInterval = true;

            setTimeout(() => {
                this.rate = 0;
                this.rateInterval = false;
            }, 30000);
        }

        if (!this.wait) {
            this.wait = true;

            setTimeout(() => {
                this.wait = false;
            }, 2000);
        }
    }

    host(username) {
        if (username != null) {
            this.say(`/host ${username}`);
        } else {
            this.say(`Error no username`);
            // console.log("Error executing host command no username");
        }
    }

    unhost() {
        this.say(`/unhost`);
    }

    raid(username) {
        if (username != null) {
            this.say(`/raid ${username}`);
        } else {
            this.say(`Error no username`);
            // console.log("Error executing host command no username");
        }
    }

    async start() {
        // start streaming
        try {
            await this.obs.startStreaming();
            this.say(`Successfully started stream`);
        } catch (e) {
            log.error(e);
            this.say(`Error ${e.error}`);
        }
    }

    async stop() {
        // stop streaming
        try {
            await this.obs.stopStreaming();
            this.say(`Successfully stopped stream`);
        } catch (e) {
            log.error(e.error);
            this.say(`${e.error}`);
        }
    }

    rec(bool) {
        if (!bool) {
            this.say(`[REC] ${this.obsProps.heartbeat.recording ? "started" : "stopped"}`);
            return;
        }

        switch (bool) {
            case "on":
                this.startStopRec(true);
                return;
            case "off":
                this.startStopRec(false);
                return;
            default:
                this.say(`[REC] Invalid option`);
                return;
        }
    }

    async startStopRec(bool) {
        if (bool) {
            try {
                const res = await this.obs.StartRecording();
                if (res.status === "ok") this.say(`[REC] Started`);
                log.success(`Started recording`);
            } catch (error) {
                this.say(`[REC] already started`);
            }
        } else {
            try {
                const res = await this.obs.StopRecording();
                if (res.status === "ok") this.say(`[REC] Stopped`);
                log.success(`Stopped recording`);
            } catch (error) {
                this.say(`[REC] already stopped`);
            }
        }
    }

    async switch(sceneName) {
        // switch scene
        try {
            await this.obs.setCurrentScene({
                "scene-name": sceneName
            });
            this.say(`Scene successfully switched to "${sceneName}"`);
        } catch (e) {
            log.error(e);
            this.say(e.error);
        }
    }

    bitrate() {
        if (this.obsProps.bitrate != null) {
            this.say(`Current bitrate: ${this.obsProps.bitrate} Kbps`);
        } else {
            this.say(`Current bitrate: offline`);
        }
    }

    sourceinfo() {
        if (this.obsProps.nginxVideoMeta != null) {
            const { height, frame_rate } = this.obsProps.nginxVideoMeta;

            this.say(`[SRC] R: ${height[0]} | F: ${frame_rate[0]} | B: ${this.obsProps.bitrate}`);
        } else {
            this.say(`[SRC] offline`);
        }
    }

    obsinfo() {
        if (this.obsProps.streamStatus != null) {
            const { fps, kbitsPerSec } = this.obsProps.streamStatus;

            this.say(`[OBS] S: ${this.obsProps.currentScene} | F: ${Math.round(fps)} | B: ${kbitsPerSec}`);
        } else {
            this.say(`[OBS] offline`);
        }
    }

    async refresh() {
        // switch scene
        if (!this.isRefreshing) {
            try {
                const lastScene = this.obsProps.currentScene;

                await this.obs.setCurrentScene({
                    "scene-name": config.obs.refreshScene
                });
                this.say(`Refreshing stream`);
                this.isRefreshing = true;

                setTimeout(() => {
                    this.obs.setCurrentScene({
                        "scene-name": lastScene
                    });
                    this.say(`Refreshing stream completed`);
                    this.isRefreshing = false;
                }, config.obs.refreshSceneInterval);
            } catch (e) {
                log.error(e);
            }
        }
    }

    live() {
        // this.ws.send(`PRIVMSG ${this.channel} :Scene switching to live`);
        this.say(`Scene switched to "${config.obs.lowBitrateScene}"`);
    }

    onNormalScene() {
        this.say(`Scene switched to "${config.obs.normalScene}"`);
        this.bitrate();
    }

    onLowBitrateScene() {
        this.say(`Scene switched to "${config.obs.lowBitrateScene}"`);
        this.bitrate();
    }

    onOfflineScene() {
        // this.ws.send(`PRIVMSG ${this.channel} :Stream went offline`);
        this.say(`Scene switched to "${config.obs.offlineScene}"`);
    }

    trigger(number) {
        if (number) {
            if (!isNaN(number)) {
                this.obsProps.lowBitrateTrigger = +number;
                config.obs.lowBitrateTrigger = +number;

                this.handleWriteToConfig();
                this.say(`Trigger successfully set to ${this.obsProps.lowBitrateTrigger} Kbps`);
            } else {
                this.say(`Error editing trigger ${number} is not a valid value`);
            }

            return;
        }

        this.say(`Current trigger set at ${this.obsProps.lowBitrateTrigger} Kbps`);
    }

    public(bool) {
        this.handleEnable("enablePublicCommands", bool, "Public comands");
    }

    mod(bool) {
        this.handleEnable("enableModCommands", bool, "Mod commands");
    }

    notify(bool) {
        this.handleEnable("enableAutoSwitchNotification", bool, "Auto switch notification");
    }

    autostop(bool) {
        this.handleEnable("enableAutoStopStreamOnHostOrRaid", bool, "Auto stop stream");
    }

    handleEnable(name, bool, response) {
        if (bool === "on" && config.twitchChat[name] != true) {
            config.twitchChat[name] = true;
            this.handleWriteToConfig();
            this.say(`${response} enabled`);
        } else if (bool === "off" && config.twitchChat[name] != false) {
            config.twitchChat[name] = false;
            this.handleWriteToConfig();
            this.say(`${response} disabled`);
        } else {
            this.say(`${response} already ${config.twitchChat[name] ? "on" : "off"}`);
        }
    }

    handleWriteToConfig() {
        fs.writeFile('"../../config.json', JSON.stringify(config, null, 4), err => {
            if (err) log.error(err);
        });
    }

    say(message) {
        this.ws.send(`PRIVMSG ${this.channel} :${message}`);
    }
}

export default Chat;
