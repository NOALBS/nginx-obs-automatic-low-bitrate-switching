import ObsSwitcher from "./components/ObsSwitcher";
import Chat from "./components/Chat";
import NodeMediaServer from "./components/NodeMediaServer";
import config from "../config";

console.log(`
    ███╗   ██╗ ██████╗  █████╗ ██╗     ██████╗ ███████╗
    ████╗  ██║██╔═══██╗██╔══██╗██║     ██╔══██╗██╔════╝
    ██╔██╗ ██║██║   ██║███████║██║     ██████╔╝███████╗
    ██║╚██╗██║██║   ██║██╔══██║██║     ██╔══██╗╚════██║
    ██║ ╚████║╚██████╔╝██║  ██║███████╗██████╔╝███████║
    ╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝ ╚══════╝ v${process.env.npm_package_version}
`);

const obs = new ObsSwitcher(
    config.obs.ip,
    config.obs.password,
    config.obs.lowBitrateScene,
    config.obs.normalScene,
    config.obs.offlineScene,
    config.obs.lowBitrateTrigger
);

if (config.twitchChat.enable) {
    const chat = new Chat(config.twitchChat.botUsername, config.twitchChat.oauth, config.twitchChat.channel, obs);
}

const nodeMediaServer = new NodeMediaServer(config.nodeMediaServer, obs);