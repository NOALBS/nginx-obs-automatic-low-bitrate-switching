import ObsSwitcher from "./components/ObsSwitcher";
import Chat from "./components/Chat";
import config from "../config";

const obs = new ObsSwitcher(
  config.ipObs,
  config.passwordObs,
  config.lowBitrateScene,
  config.normalScene,
  config.offlineScene,
  config.lowBitrateTrigger
);

const chat = new Chat(
  config.twitchUsername,
  config.twitchOauth,
  `#${config.twitchUsername}`,
  obs
);
