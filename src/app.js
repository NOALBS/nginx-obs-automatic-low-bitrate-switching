import StartMessage from "./components/startMessage";
import ChatController from "./components/chat/chatController";

import config from "../config/config";
import users from "../config/users";

const chatController = new ChatController(config.chatServices, users);
