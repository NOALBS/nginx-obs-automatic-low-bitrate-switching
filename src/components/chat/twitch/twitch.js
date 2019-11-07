import TwitchConnection from "./twitchConnection";

class Chat extends TwitchConnection {
    constructor(config) {
        super(config.username, config.oauth);

        this.queue = [];
        this.queueRunning = false;
        this.ratelimitHandlerRunning = false;
        this.messagesSend = 0;
        this.maxMessages = 0;

        this.setMaxMessages(config.type);
    }

    enqueue(channel, message) {
        if (this.messagesSend < this.maxMessages && this.queue.length == 0) {
            this.sendMessage(channel, message);
            if (!this.ratelimitHandlerRunning) this.ratelimitHandler();
        } else {
            this.queue.push({ channel, message });
        }
    }

    join(channel) {
        this.ws.send(`JOIN #${channel.toLowerCase()}`);
    }

    // leave(channel) {
    //     // leave channel
    // }

    sendLoop() {
        // just send the message don't wait
        this.queueRunning = true;

        const interval = setInterval(() => {
            this.dequeueAndSendMessage();
            if (!this.ratelimitHandlerRunning) this.ratelimitHandler();

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

            if (this.queue.length > 0) this.sendLoop();
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
}

export default Chat;
