import Twitch from "./twitch/twitch";
import events from "../globalEvents";

class ChatController {
    constructor(chatServices) {
        this.chatServices = chatServices;
        this.connections = {};

        this.startChatServices();
        this.joinChannels();

        events.on("chat:message", this.messageHandler.bind(this));
    }

    startChatServices() {
        this.chatServices.twitch.enable && (this.connections.twitch = new Twitch(this.chatServices.twitch));
    }

    joinChannels() {
        events.once("hellopleasegivemethechannels", channels => {
            channels.map(chn => {
                const { provider, channel } = chn;
                events.emit(`join:${provider}`, channel);
            });
        });

        events.emit("db:request", "hellopleasegivemethechannels", "getChannels");
    }

    send(provider, channel, message) {
        events.emit(`send:${provider}`, channel, message);
    }

    messageHandler(provider, channel, username, message, isMod) {
        console.log(provider, channel, username, message, isMod);
    }

    }
}

export default ChatController;
