import OBSWebSocket from "obs-websocket-js";
import fetch from "node-fetch";
import xml2js from "xml2js";
import config from "../../config";
import EventEmitter from "events";

const parseString = xml2js.parseString;

class ObsSwitcher extends EventEmitter {
  constructor(address, password, low, normal, offline, lowBitrateTrigger) {
    super();

    this.obs = new OBSWebSocket();
    this.isLive = false;
    this.address = address;
    this.password = password;
    this.lowBitrateScene = low;
    this.normalScene = normal;
    this.offlineScene = offline;
    this.lowBitrateTrigger = lowBitrateTrigger;
    this.bitrate = null;
    this.currentScene = null;

    this.obs
      .connect({ address: this.address, password: this.password })
      .catch(e => {
        // handle this somewhere else
      });

    this.obs.on("ConnectionClosed", this.onDisconnect.bind(this));
    this.obs.on("AuthenticationSuccess", this.onAuth.bind(this));
    this.obs.on("AuthenticationFailure", this.onAuthFail.bind(this));
    this.obs.on("error", this.error.bind(this));
  }

  onAuth() {
    console.log(`Success! We're connected & authenticated.`);

    this.interval = setInterval(async () => {
      const currentScene = await this.obs.GetCurrentScene();
      const bitrate = await this.getBitrate();
      const canSwitch =
        currentScene.name == this.lowBitrateScene ||
        currentScene.name == this.normalScene ||
        currentScene.name == this.offlineScene;

      this.currentScene = currentScene.name;

      if (bitrate !== null) {
        this.isLive = true;

        this.isLive &&
          canSwitch &&
          (bitrate === 0 &&
            currentScene.name !== this.lowBitrateScene &&
            (this.obs.setCurrentScene({
              "scene-name": this.lowBitrateScene
            }),
            config.twitchChat.enableAutoSwitchNotification &&
              this.emit("live")),
          bitrate <= this.lowBitrateTrigger &&
            currentScene.name !== this.lowBitrateScene &&
            bitrate !== 0 &&
            (this.obs.setCurrentScene({
              "scene-name": this.lowBitrateScene
            }),
            config.twitchChat.enableAutoSwitchNotification &&
              this.emit("lowBitrateScene"),
            console.log(
              `Low bitrate detected switching to scene ${this.lowBitrateScene}.`
            )),
          bitrate > this.lowBitrateTrigger &&
            currentScene.name !== this.normalScene &&
            (this.obs.setCurrentScene({ "scene-name": this.normalScene }),
            config.twitchChat.enableAutoSwitchNotification &&
              this.emit("normalScene"),
            console.log(`Switching back to scene ${this.normalScene}.`)));
      } else {
        this.isLive = false;

        canSwitch &&
          currentScene.name !== this.offlineScene &&
          (this.obs.setCurrentScene({ "scene-name": this.offlineScene }),
          config.twitchChat.enableAutoSwitchNotification &&
            this.emit("offlineScene"),
          console.log(
            `Error receiving current bitrate or steam is offline. Switching to scene ${
              this.offlineScene
            }.`
          ));
      }
    }, config.obs.requestMs);
  }

  async getBitrate(username = "live") {
    try {
      const response = await fetch(`http://${config.nginx.ip}/stat`);
      const data = await response.text();

      parseString(data, (err, result) => {
        const publish = result.rtmp.server[0].application[0].live[0].stream;
        if (publish == null) {
          this.bitrate = 0;
        } else {
          const stream = publish.find(stream => {
            return stream.name[0] === username;
          });

          this.bitrate = Math.round(stream.bw_video[0] / 1024);
        }
      });
    } catch (e) {
      console.log("Error fetching stats");
    }

    return this.bitrate;
  }

  error(e) {
    console.log(e);
  }

  onDisconnect() {
    console.error("Can't connect to OBS or lost connnection.");
    clearInterval(this.interval);

    this.reconnect();
  }

  onAuthFail() {
    console.log("Failed to authenticate to OBS");
  }

  reconnect() {
    console.log("Trying to reconnect in 5 seconds");
    setTimeout(() => {
      this.obs
        .connect({ address: this.address, password: this.password })
        .catch(e => {
          // handle this somewhere else
        });
    }, 5000);
  }
}

export default ObsSwitcher;
