import ObsSwitcher from "./components/ObsSwitcher";
import Chat from "./components/Chat";
import config from "../config";

const obs = new ObsSwitcher(
  config.obs.ip,
  config.obs.password,
  config.obs.lowBitrateScene,
  config.obs.normalScene,
  config.obs.offlineScene,
  config.obs.lowBitrateTrigger
);

if (config.twitchChat.enable) {
  const chat = new Chat(
    config.twitchChat.botUsername,
    config.twitchChat.oauth,
    config.twitchChat.channel,
    obs
  );
}
