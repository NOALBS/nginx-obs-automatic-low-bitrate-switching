import Twitch from "./twitch/twitch";
import events from "../globalEvents";

class ChatController {
    constructor(chatServices) {
        this.chatServices = chatServices;
        this.users = users;
        this.connections = {};

        this.startChatServices();
        this.joinChannels();

        events.on("chat:message", this.messageHandler.bind(this));
    }

    startChatServices() {
        this.chatServices.twitch.enable && (this.connections.twitch = new Twitch(this.chatServices.twitch));
    }

    joinChannels() {
        for (const user in this.users) {
            if (this.users.hasOwnProperty(user)) {
                const settings = this.users[user].chat;

                if (this.connections[settings.provider] != null) this.connections[settings.provider].enqueueJoin(settings.channel);
            }
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
