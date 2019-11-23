// get all the users
export function getUsers() {
    return this.db.get("users").value();
}

// get the specified user
export function getUser(channel) {
    return this.db
        .get("users")
        .find({ chat: { channel } })
        .value();
}

// chat prefix for user
export function getPrefix(channel) {
    return this.db
        .get("users")
        .find({ chat: { channel } })
        .value().chat.prefix;
}

// list of aliasses for specified username
export function getAliasses(username) {
    // return this.data;
    return "not implemented";
}

// Channels needs to atleast return [provider, channel]
export function getChannels() {
    return this.db
        .get("users")
        .map("chat")
        .value();
}

export function getChannel(channel) {
    return this.db
        .get("users")
        .find({ chat: { channel } })
        .value().chat;
}
