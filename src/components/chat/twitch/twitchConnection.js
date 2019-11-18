import WebSocket from "ws";
import signale from "signale";
import events from "../../globalEvents";

signale.config({
    displayTimestamp: true,
    displayDate: true
});

const log = signale.scope("CHT");

class TwitchConnection {
    constructor(username, password) {
        this.username = username.toLowerCase();
        this.password = password;
        this.connected = false;
        this.grow = 0;

        this.open();
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

            this.ws.send("CAP REQ :twitch.tv/tags twitch.tv/commands");
            this.ws.send(`PASS ${this.password}`);
            this.ws.send(`NICK ${this.username}`);

            if (this.grow > 0) this.grow = 0;
            this.keepAlive();
            this.connected = true;
            if (!this.joinQueueRunning) this.joinLoop();
        }
    }

    onClose() {
        log.error("Disconnected from twitch server");
        this.connected = false;
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
        const seconds = 1 << (this.grow <= 6 ? this.grow++ : this.grow);
        log.info(`Trying to reconnect in ${seconds} seconds`);
        this.joinQueue = Array.from(this.channels);

        setTimeout(() => {
            log.info("Reconnecting...");
            this.open();
        }, 1000 * seconds);
    }

    onError(e) {
        log.error(new Error(e));
    }

    onMessage(message) {
        if (message !== null) {
            const parsed = this.parse(message.data);
            switch (parsed.command) {
                case "PRIVMSG":
                    events.emit("message", "twitch", parsed.channel.substring(1), parsed.username, parsed.message, !!+parsed.tags.mod);
                    // this.handleMessage(parsed);
                    break;
                // case "HOSTTARGET":
                //     if (config.twitchChat.enableAutoStopStreamOnHostOrRaid && !parsed.message.startsWith("-") && this.obsProps.bitrate != null) {
                //         log.info("Channel started hosting, stopping stream");
                //         this.stop();
                //     }
                //     break;
                case "PING":
                    this.ws.send(`PONG ${parsed.channel}`);
                    break;
                case "PONG":
                    // const ms = new Date().getTime() - this.sendPing;
                    // console.log(`Pong received after ${ms} ms`);

                    clearTimeout(this.pingTimeout);
                    this.sendPing = null;
                    break;
                case "NOTICE":
                    switch (parsed.message) {
                        case "Login authentication failed":
                            log.error("Login authentication failed. Please check your login details.");
                            process.exit();
                            break;
                        default:
                            break;
                    }
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

    // handleMessage(msg) {
    //     console.log(msg);

    // if (!msg.message.startsWith(this.prefix)) return;

    // let [commandName, ...params] = msg.message.slice(1).split(" ");

    // if (commandName in this.aliases) {
    //     const alias = this.aliases[commandName].split(" ");
    //     alias.length == 1 ? (commandName = alias[0]) : ([commandName, ...params] = alias);
    // }

    // switch (true) {
    //     case commandName == "noalbs":
    //     case config.twitchChat.adminUsers.includes(msg.username):
    //     case config.twitchChat.enableModCommands && msg.tags.mod === "1" && this.allowModsCommands.includes(commandName):
    //     case config.twitchChat.enablePublicCommands && !this.wait && this.allowAllCommands.includes(commandName):
    //     case msg.username === this.channel.substring(1):
    //         if (this.rate == 20) return;
    //         if (!this.commands.includes(commandName)) return;

    //         this[commandName](...params);
    //         log.success(`${msg.username} Executed ${commandName} command`);
    //         this.setWait();
    //         break;

    //     default:
    //         break;
    // }
    // }

    // setWait() {
    //     this.rate++;

    //     if (!this.rateInterval) {
    //         this.rateInterval = true;

    //         setTimeout(() => {
    //             this.rate = 0;
    //             this.rateInterval = false;
    //         }, 30000);
    //     }

    //     if (!this.wait) {
    //         this.wait = true;

    //         setTimeout(() => {
    //             this.wait = false;
    //         }, 2000);
    //     }
    // }
}

export default TwitchConnection;
