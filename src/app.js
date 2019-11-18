import StartMessage from "./components/startMessage";
import ChatController from "./components/chat/chatController";
import Db from "./components/database";
import events from "./components/globalEvents";

import config from "../config/config";

events.on("db:connected", () => {
    const chatController = new ChatController(config.chatServices);
});

const db = new Db(config.database);
