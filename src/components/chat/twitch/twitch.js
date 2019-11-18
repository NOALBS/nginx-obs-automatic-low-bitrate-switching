import TwitchConnection from "./twitchConnection";
import events from "../../globalEvents";

class Chat extends TwitchConnection {
    constructor(config) {
        super(config.username, config.oauth);

        this.queue = [];
        this.channels = [];
        this.joinQueue = [];
        this.queueRunning = false;
        this.ratelimitHandlerRunning = false;
        this.joinQueueRunning = false;
        this.messagesSend = 0;
        this.maxMessages = 0;

        this.setMaxMessages(config.type);
        events.on("join:twitch", this.enqueueJoin.bind(this));
        events.on("send:twitch", this.enqueue.bind(this));
    }

    join(channel) {
        console.log("Joining channel", channel);
        this.ws.send(`JOIN #${channel.toLowerCase()}`);
    }

    enqueueJoin(user) {
        this.channels.push(user);
        this.joinQueue.push(user);
        if (!this.joinQueueRunning) this.joinLoop();
    }

    joinLoop() {
        this.joinQueueRunning = true;

        const interval = setInterval(() => {
            if (this.connected) this.join(this.joinQueue.shift());

            if (this.joinQueue.length == 0) {
                this.joinQueueRunning = false;
                clearInterval(interval);
            }
        }, 500);
    }

    enqueue(channel, message) {
        if (this.messagesSend < this.maxMessages && this.queue.length == 0 && this.connected) {
            this.sendMessage(channel, message);
            if (!this.ratelimitHandlerRunning) this.ratelimitHandler();
        } else {
            this.queue.push({ channel, message });
            if (!this.queueRunning) this.sendLoop();
        }
    }

    sendLoop() {
        this.queueRunning = true;

        const interval = setInterval(() => {
            if (this.connected) {
                this.dequeueAndSendMessage();
                if (!this.ratelimitHandlerRunning) this.ratelimitHandler();
            }

            if (this.queue.length == 0 || this.messagesSend >= this.maxMessages) {
                this.queueRunning = false;
                clearInterval(interval);
            }
        }, 500);
    }

    ratelimitHandler() {
        this.ratelimitHandlerRunning = true;

        setTimeout(() => {
            this.messagesSend = 0;
            this.ratelimitHandlerRunning = false;

            if (this.queue.length > 0 && !this.queueRunning) this.sendLoop();
        }, 1000 * 30 + 1);
    }

    dequeueAndSendMessage() {
        const { channel, message } = this.queue.shift();
        this.sendMessage(channel, message);
    }

    sendMessage(channel, message) {
        this.ws.send(`PRIVMSG #${channel.toLowerCase()} :${message}`);
        this.messagesSend++;
    }

    setMaxMessages(botType) {
        switch (botType.toLowerCase()) {
            case "known":
                this.maxMessages = 50;
                break;
            case "verified":
                this.maxMessages = 7500;
                break;
            default:
                this.maxMessages = 20;
                break;
        }
    }

    // leave(channel) {
    //     // leave channel
    // }
}

export default Chat;
