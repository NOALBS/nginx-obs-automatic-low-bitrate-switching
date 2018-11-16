var OBSWebSocket = require("obs-websocket-js");
var express = require("express");
var request = require("request");
var xml2js = require("xml2js");

var config = require("./config");

const obs = new OBSWebSocket();
const app = express();
const parseString = xml2js.parseString;
let isLive = false;

obs.connect({ address: config.ipObs, password: config.password })
    .then(function () { console.log(`Success! We're connected & authenticated.`) })
    .catch(function (err) { console.error("Can't connect to OBS, did you enter the correct ip?") });

app.get('/online', function (req, res) {
    isLive = true;

    obs.setCurrentScene({ 'scene-name': config.normalScene });
    res.sendStatus(200);
});

app.get('/offline', function (req, res) {
    isLive = false;

    obs.setCurrentScene({ 'scene-name': config.offlineScene });
    res.sendStatus(200);
});

setInterval(function () {
    if (isLive) {
        request('http://' + config.ipNginx + '/stat', function (error, response, body) {
            if (error) throw error;

            parseString(body, async function (err, result) {
                let bitrate = result.rtmp.server[0].application[0].live[0].stream[0].bw_video[0] / 1024;
                const currentScene = await obs.GetCurrentScene();

                bitrate <= config.lowBitrateTrigger && currentScene.name !== config.lowBitrateScene && bitrate !== 0 && (obs.setCurrentScene({ 'scene-name': config.lowBitrateScene }), console.log(`Low bitrate detected switching to scene ${config.lowBitrateScene}.`));
                bitrate > config.lowBitrateTrigger && currentScene.name !== config.normalScene && (obs.setCurrentScene({ 'scene-name': config.normalScene }), console.log(`Switching back to scene ${config.normalScene}.`));
            });
        });
    }
}, 2000);

app.listen(3000, function () { console.log('App listening on port 3000!') });