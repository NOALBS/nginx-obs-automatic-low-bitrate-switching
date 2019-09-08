import WebSocket from "ws";
import config from "../../config";
import fs from "fs";
import signale from "signale";
import { search } from "fast-fuzzy";
import fetch from "node-fetch";

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
            "rec",
            "noalbs",
            "fix"
        ];
        this.aliases = { o: "obsinfo", s: "sourceinfo", b: "bitrate", r: "refresh", ss: "switch" };
        this.allowAllCommands = config.twitchChat.publicCommands;
        this.allowModsCommands = config.twitchChat.modCommands;
        this.wait = false;
        this.rate = 0;
        this.rateInterval = false;
        this.isRefreshing = false;

        this.open();
        this.registerAliases();

        this.obsProps.on("live", this.live.bind(this));
        this.obsProps.on("normalScene", this.onNormalScene.bind(this));
        this.obsProps.on("lowBitrateScene", this.onLowBitrateScene.bind(this));
        this.obsProps.on("offlineScene", this.onOfflineScene.bind(this));
    }

    open() {
        log.info("Connecting to twitch");
        this.ws = new WebSocket("wss://irc-ws.chat.twitch.tv:443");

        this.ws.onopen = this.onOpen.bind(this);
        this.ws.onmessage = this.onMessage.bind(this);
        this.ws.onerror = this.onError.bind(this);
        this.ws.onclose = this.onClose.bind(this);
    }

    keepAlive() {
        this.interval = setInterval(() => {
            if (this.sendPing) return;

            this.ws.send("PING :tmi.twitch.tv\r\n");
            this.sendPing = new Date().getTime();

            this.pingTimeout = setTimeout(() => {
                log.error(`Didn't receive PONG in time.. reconnecting to twitch.`);
                this.close();
                this.sendPing = null;
            }, 1000 * 10);
        }, 1000 * 60 * 2);
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
            switch (parsed.command) {
                case "PRIVMSG":
                    this.handleMessage(parsed);
                    break;
                case "HOSTTARGET":
                    if (config.twitchChat.enableAutoStopStreamOnHostOrRaid && this.obsProps.bitrate != null) {
                        log.info("Channel started hosting, stopping stream");
                        this.stop();
                    }
                    break;
                case "PING":
                    this.ws.send(`PONG ${parsed.channel}`);
                    break;
                case "PONG":
                    const ms = new Date().getTime() - this.sendPing;
                    // console.log(`Pong received after ${ms} ms`);

                    clearTimeout(this.pingTimeout);
                    this.sendPing = null;
                    break;
                default:
                    break;
            }
        }
    }

    parse(message) {
        let parsedMessage = {
            tags: {},
            channel: null,
            command: null,
            username: null,
            message: null,
            raw: message
        };

        // tags
        if (message.startsWith("@")) {
            var space = message.indexOf(" ");
            const tagsRaw = message.slice(1, space);
            const tagsSplit = tagsRaw.split(";");
            tagsSplit.map(d => {
                const tagSplit = d.split("=");
                if (tagSplit[1] == "") tagSplit[1] = null;
                parsedMessage.tags[tagSplit[0]] = tagSplit[1];
            });
        }

        message = message
            .slice(space + 1)
            .trim()
            .split(" ");
        let pos = 0;

        if (message[0].startsWith(":")) {
            parsedMessage.username = message[0].substring(1, message[0].indexOf("!"));
            pos += 1;
        }

        parsedMessage.command = message[pos];
        parsedMessage.channel = message[pos + 1];

        if (!message[pos + 2] == "")
            parsedMessage.message = message
                .slice(3)
                .join(" ")
                .slice(1);

        return parsedMessage;
    }

    handleMessage(msg) {
        if (!msg.message.startsWith(this.prefix)) return;

        let [commandName, ...params] = msg.message.slice(1).split(" ");

        if (commandName in this.aliases) {
            commandName = this.aliases[commandName];
        }

        switch (true) {
            case commandName == "noalbs":
            case config.twitchChat.adminUsers.includes(msg.username):
            case config.twitchChat.enableModCommands && msg.tags.mod === "1" && this.allowModsCommands.includes(commandName):
            case config.twitchChat.enablePublicCommands && !this.wait && this.allowAllCommands.includes(commandName):
            case msg.username === this.channel.substring(1):
                if (this.rate == 20) return;
                if (!this.commands.includes(commandName)) return;

                this[commandName](...params);
                log.success(`${msg.username} Executed ${commandName} command`);
                this.setWait();
                break;

            default:
                break;
        }
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
        if (sceneName == null) return this.say(`No scene specified`);

        const res = search(sceneName, this.obsProps.scenes, { keySelector: obj => obj.name });
        const scene = res.length > 0 ? res[0].name : sceneName;

        try {
            await this.obs.setCurrentScene({
                "scene-name": scene
            });
            this.say(`Scene successfully switched to "${scene}"`);
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
        if (!this.isRefreshing) {
            try {
                const lastScene = this.obsProps.currentScene;

                if (lastScene == null) return this.say(`Error refreshing stream`);

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

    live(previous) {
        // this.ws.send(`PRIVMSG ${this.channel} :Scene switching to live`);
        this.say(`Scene switched to "${previous}"`);
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
        if (!bool) {
            this.say(`${response} is ${config.twitchChat[name] ? "enabled" : "disabled"}`);
            return;
        }

        if (bool === "on" && config.twitchChat[name] != true) {
            config.twitchChat[name] = true;
            this.handleWriteToConfig();
            this.say(`${response} enabled`);
        } else if (bool === "off" && config.twitchChat[name] != false) {
            config.twitchChat[name] = false;
            this.handleWriteToConfig();
            this.say(`${response} disabled`);
        } else {
            this.say(`${response} already ${config.twitchChat[name] ? "enabled" : "disabled"}`);
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

    noalbs(method, alias, commandName) {
        if (method === "version") this.say(`Running NOALBS v${process.env.npm_package_version}`);

        let exists = false;

        switch (method) {
            case "add":
                if (!this.commands.includes(commandName)) return this.say(`Error ${commandName} does not exist`);

                // Check if already exists to replace it
                config.twitchChat.alias.map(arr => {
                    if (arr[0] == alias) {
                        arr[1] = commandName;
                        exists = true;
                    }
                });

                this.aliases[alias] = commandName;
                if (exists) return this.writeAliasToConfig(alias);

                config.twitchChat.alias.push([alias, commandName]);
                this.writeAliasToConfig(alias);
                break;
            case "remove":
                config.twitchChat.alias.map((arr, index) => {
                    if (arr[0] == alias) {
                        config.twitchChat.alias.splice(index);
                        delete this.aliases[alias];
                        this.handleWriteToConfig();
                        this.say(`Removed alias "${alias}"`);
                        exists = true;
                    }
                });

                if (exists) return;

                this.say(`Alias "${alias}" does not exist`);
                break;
            default:
                break;
        }
    }

    writeAliasToConfig(alias) {
        this.handleWriteToConfig();
        this.say(`Added alias "${alias}"`);
    }

    async fix() {
        this.say(`Trying to fix the stream`);

        try {
            const { ip, application, key } = this.obsProps.nginxSettings;
            const response = await fetch(`http://${ip}/control/drop/subscriber?app=${application}&name=${key}`);

            if (response.ok) {
                this.say(`Successfully fixed the stream`);
            }
        } catch (e) {
            console.log(e);
            this.say(`Error fixing the stream`);
        }
    }

    registerAliases() {
        if (config.twitchChat.alias == null) return;

        for (const alias of config.twitchChat.alias) {
            this.aliases[alias[0]] = alias[1];
        }
    }
}

export default Chat;
