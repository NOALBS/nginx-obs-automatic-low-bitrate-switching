var OBSWebSocket = require("obs-websocket-js");
var express = require("express");
var request = require("request");
var xml2js = require("xml2js");

var config = require("./config");

const obs = new OBSWebSocket();
const app = express();
const parseString = xml2js.parseString;
let isLive = false;

obs
  .connect({ address: config.ipObs, password: config.passwordObs })
  .then(() => console.log(`Success! We're connected & authenticated.`))
  .catch(() =>
    console.error("Can't connect to OBS, did you enter the correct ip?")
  );

app.get("/online", (req, res) => {
  isLive = true;
  res.sendStatus(200);
});

app.get("/offline", (req, res) => {
  isLive = false;
  res.sendStatus(200);
});

setInterval(() => {
  request(`http://${config.ipNginx}/stat`, (error, response, body) => {
    if (error) console.log("Can't access nginx");

    parseString(body, async (err, result) => {
      const currentScene = await obs.GetCurrentScene();
      const canSwitch =
        currentScene.name == config.lowBitrateScene ||
        currentScene.name == config.normalScene ||
        currentScene.name == config.offlineScene;

      try {
        const bitrate =
          result.rtmp.server[0].application[0].live[0].stream[0].bw_video[0] /
          1024;

        isLive = true;

        isLive &&
          canSwitch &&
          (bitrate <= config.lowBitrateTrigger &&
            currentScene.name !== config.lowBitrateScene &&
            bitrate !== 0 &&
            (obs.setCurrentScene({ "scene-name": config.lowBitrateScene }),
            console.log(
              `Low bitrate detected switching to scene ${
                config.lowBitrateScene
              }.`
            )),
          bitrate > config.lowBitrateTrigger &&
            currentScene.name !== config.normalScene &&
            (obs.setCurrentScene({ "scene-name": config.normalScene }),
            console.log(`Switching back to scene ${config.normalScene}.`)));
      } catch (e) {
        isLive = false;
        canSwitch &&
          currentScene.name !== config.offlineScene &&
          (obs.setCurrentScene({ "scene-name": config.offlineScene }),
          console.log(
            `Error receiving current bitrate or steam is offline. Switching to scene ${
              config.offlineScene
            }.`
          ));
      }
    });
  });
}, config.requestMs);

app.listen(config.PORT, () => {
  console.log(`App listening on port ${config.PORT}!`);
});
