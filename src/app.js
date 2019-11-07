import StartMessage from "./components/startMessage";
import ChatController from "./components/chat/chatController";

import config from "../config/config";

const chatController = new ChatController(config.chatServices);
