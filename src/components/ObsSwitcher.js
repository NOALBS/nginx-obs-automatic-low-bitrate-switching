import OBSWebSocket from "obs-websocket-js";
import request from "request";
import xml2js from "xml2js";
import config from "../../config";

import Chat from "./Chat";

class ObsSwitcher {
  constructor() {
    this.obs = new OBSWebSocket();
    this.parseString = xml2js.parseString;
    this.isLive = false;
  }

  connect() {
    this.obs
      .connect({ address: config.ipObs, password: config.passwordObs })
      .then(() => console.log(`Success! We're connected & authenticated.`))
      .catch(() =>
        console.error("Can't connect to OBS, did you enter the correct ip?")
      );

    this.obs.on("AuthenticationSuccess", this.onAuth.bind(this));
    this.obs.on("error", this.error.bind(this));
  }

  onAuth() {
    new Chat(
      config.twitchUsername,
      config.twitchOauth,
      `#${config.twitchUsername}`,
      this.obs
    );

    setInterval(() => {
      request(`http://${config.ipNginx}/stat`, (error, response, body) => {
        if (error) console.log("Can't access nginx");

        this.parseString(body, async (err, result) => {
          const currentScene = await this.obs.GetCurrentScene();
          const canSwitch =
            currentScene.name == config.lowBitrateScene ||
            currentScene.name == config.normalScene ||
            currentScene.name == config.offlineScene;

          try {
            const bitrate =
              result.rtmp.server[0].application[0].live[0].stream[0]
                .bw_video[0] / 1024;

            this.isLive = true;

            this.isLive &&
              canSwitch &&
              (bitrate === 0 &&
                this.obs.setCurrentScene({ "scene-name": config.normalScene }),
              bitrate <= config.lowBitrateTrigger &&
                currentScene.name !== config.lowBitrateScene &&
                bitrate !== 0 &&
                (this.obs.setCurrentScene({
                  "scene-name": config.lowBitrateScene
                }),
                console.log(
                  `Low bitrate detected switching to scene ${
                    config.lowBitrateScene
                  }.`
                )),
              bitrate > config.lowBitrateTrigger &&
                currentScene.name !== config.normalScene &&
                (this.obs.setCurrentScene({ "scene-name": config.normalScene }),
                console.log(`Switching back to scene ${config.normalScene}.`)));
          } catch (e) {
            this.isLive = false;

            canSwitch &&
              currentScene.name !== config.offlineScene &&
              (this.obs.setCurrentScene({ "scene-name": config.offlineScene }),
              console.log(
                `Error receiving current bitrate or steam is offline. Switching to scene ${
                  config.offlineScene
                }.`
              ));
          }
        });
      });
    }, config.requestMs);
  }

  error(e) {
    console.log(e);
  }
}

export default ObsSwitcher;
