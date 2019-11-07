import Twitch from "./twitch/twitch";

class ChatController {
    constructor(chatServices) {
        this.chatServices = chatServices;
        this.connections = {};

        this.startChatServices();
    }

    startChatServices() {
        this.chatServices.twitch.enable && (this.connections.twitch = new Twitch(this.chatServices.twitch));
    }

    send(service, channel, message) {
        if (this.connections[service] != null) this.connections[service].enqueue(channel, message);
    }
}

export default ChatController;
