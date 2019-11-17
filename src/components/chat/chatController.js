import Twitch from "./twitch/twitch";

class ChatController {
    constructor(chatServices) {
        this.chatServices = chatServices;
        this.users = users;
        this.connections = {};

        this.startChatServices();
        this.joinChannels();
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
    }

    send(service, channel, message) {
        if (this.connections[service] != null) this.connections[service].enqueue(channel, message);
    }
}

export default ChatController;
