import format from "string-template";

import Twitch from "./twitch/twitch";
import events from "../globalEvents";
import * as chatCommands from "./chatCommands";

import en from "../../../locales/en";
import zh_tw from "../../../locales/zh_tw";

class ChatController {
    constructor(chatServices) {
        this.chatServices = chatServices;
        this.events = events;
        this.connections = {};
        this.commands = ["switch"];

        // maybe make a language controller
        this.languages = {
            en: en,
            zh_tw: zh_tw
        };

        this.addCommands();
        this.startChatServices();
        this.joinChannels();

        events.on("chat:message", this.messageHandler.bind(this));
    }

    startChatServices() {
        this.chatServices.twitch.enable && (this.connections.twitch = new Twitch(this.chatServices.twitch));
    }

    async joinChannels() {
        try {
            const res = await events.do("db:request", "getChannels");

            res.map(chn => {
                const { provider, channel } = chn;
                events.emit(`join:${provider}`, channel);
            });
        } catch (e) {
            console.error(e);
        }
    }

    send(provider, channel, message) {
        events.emit(`send:${provider}`, channel, message);
    }

    async messageHandler(provider, channel, username, message, isMod) {
        console.log(provider, channel, username, message, isMod);

        try {
            const chn = await events.do("db:request", "getChannel", channel);

            if (!message.startsWith(chn.prefix)) return;

            let [commandName, ...params] = message.slice(1).split(" ");

            if (commandName in chn.alias) {
                const alias = chn.alias[commandName].split(" ");
                alias.length == 1 ? (commandName = alias[0]) : ([commandName, ...params] = alias);
            }

            switch (true) {
                case commandName == "noalbs":
                case chn.adminUsers.includes(message.username):
                case chn.enableModCommands && isMod && chn.modCommands.includes(commandName):
                case chn.enablePublicCommands && chn.publicComands.includes(commandName):
                case username === chn.channel:
                    if (!this.commands.includes(commandName)) return;
                    const returned = await this[commandName](chn, ...params);

                    if (returned != null) {
                        if (returned.type == "send") {
                            this.send(provider, channel, returned.data);
                        } else {
                            this.send(provider, channel, format(this.languages[chn.language][commandName][returned.type], returned.data));
                        }
                    }
                    console.log(username, "executed", commandName);
                    break;

                default:
                    break;
            }
        } catch (e) {
            console.error(e);
        }
    }

    addCommands() {
        for (const [name, func] of Object.entries(chatCommands)) {
            ChatController.prototype[name] = func;
            this.commands.push(name);
        }
    }

    // can't use switch as a function :( so just redirect it :D
    async switch(channel, sceneName) {
        return await this.Switch(channel, sceneName);
    }
}

export default ChatController;
