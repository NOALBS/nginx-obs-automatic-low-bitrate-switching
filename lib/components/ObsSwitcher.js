"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _obsWebsocketJs = _interopRequireDefault(require("obs-websocket-js"));

var _nodeFetch = _interopRequireDefault(require("node-fetch"));

var _xml2js = _interopRequireDefault(require("xml2js"));

var _config = _interopRequireDefault(require("../../config"));

var _events = _interopRequireDefault(require("events"));

var _signale = _interopRequireDefault(require("signale"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

_signale.default.config({
  displayTimestamp: true,
  displayDate: true
});

var log = _signale.default.scope("OBS");

var parseString = _xml2js.default.parseString;

class ObsSwitcher extends _events.default {
  constructor(address, password, low, normal, offline, lowBitrateTrigger) {
    var highRttTrigger = arguments.length > 6 && arguments[6] !== undefined ? arguments[6] : 2500;
    super();
    this.obs = new _obsWebsocketJs.default();
    this.isLive = false;
    this.address = address;
    this.password = password;
    this.lowBitrateScene = low;
    this.normalScene = normal;
    this.offlineScene = offline;
    this.lowBitrateTrigger = lowBitrateTrigger;
    this.highRttTrigger = highRttTrigger;
    this.bitrate = null;
    this.nginxVideoMeta = null;
    this.streamStatus = null;
    this.heartbeat = null;
    this.obsStreaming = false;
    this.currentScene = null;
    this.nginxSettings;
    this.previousScene = this.lowBitrateScene;
    this.scenes = null;
    this.obs.connect({
      address: this.address,
      password: this.password
    }).catch(e => {// handle this somewhere else
    });
    this.obs.on("ConnectionClosed", this.onDisconnect.bind(this));
    this.obs.on("AuthenticationSuccess", this.onAuth.bind(this));
    this.obs.on("AuthenticationFailure", this.onAuthFail.bind(this));
    this.obs.on("error", this.error.bind(this));
    this.obs.on("StreamStatus", this.setStreamStatus.bind(this));
    this.obs.on("StreamStopped", this.streamStopped.bind(this));
    this.obs.on("StreamStarted", this.streamStarted.bind(this));
    this.obs.on("Heartbeat", this.handleHeartbeat.bind(this));
    this.obs.on("ScenesChanged", this.scenesChanged.bind(this));
    log.info("Connecting & authenticating");
  }

  switchSceneIfNecessary() {
    var _this = this;

    return _asyncToGenerator(function* () {
      if (!_this.obsStreaming && (_config.default.obs.onlySwitchWhenStreaming == null || _config.default.obs.onlySwitchWhenStreaming)) return;
      var [bitrate, rtt] = yield _this.getBitrate();
      var {
        currentScene,
        canSwitch
      } = yield _this.canSwitch();

      if (bitrate !== null) {
        _this.isLive = true;

        if (["nimble", "srt-live-server"].includes(_config.default.rtmp.server)) {
          _this.isLive && canSwitch && (bitrate === 0 && currentScene.name !== _this.previousScene && (_this.obs.send("SetCurrentScene", {
            "scene-name": _this.previousScene
          }), _this.switchSceneEmit("live", _this.previousScene), log.info("Stream went online switching to scene: \"".concat(_this.previousScene, "\""))), (rtt < _this.highRttTrigger || rtt >= _this.highRttTrigger) && bitrate <= _this.lowBitrateTrigger && currentScene.name !== _this.lowBitrateScene && bitrate !== 0 && (_this.obs.send("SetCurrentScene", {
            "scene-name": _this.lowBitrateScene
          }), _this.previousScene = _this.lowBitrateScene, _this.switchSceneEmit("lowBitrateScene"), log.info("Low bitrate detected switching to scene: \"".concat(_this.lowBitrateScene, "\""))), rtt >= _this.highRttTrigger && bitrate > _this.lowBitrateTrigger && currentScene.name !== _this.lowBitrateScene && bitrate !== 0 && (_this.obs.send("SetCurrentScene", {
            "scene-name": _this.lowBitrateScene
          }), _this.previousScene = _this.lowBitrateScene, _this.switchSceneEmit("lowBitrateScene"), log.info("High RTT detected switching to scene: \"".concat(_this.lowBitrateScene, "\""))), rtt < _this.highRttTrigger && bitrate > _this.lowBitrateTrigger && currentScene.name !== _this.normalScene && (_this.obs.send("SetCurrentScene", {
            "scene-name": _this.normalScene
          }), _this.previousScene = _this.normalScene, _this.switchSceneEmit("normalScene"), log.info("Switching to normal scene: \"".concat(_this.normalScene, "\""))));
        } else {
          _this.isLive && canSwitch && (bitrate === 0 && currentScene.name !== _this.previousScene && (_this.obs.send("SetCurrentScene", {
            "scene-name": _this.previousScene
          }), _this.switchSceneEmit("live", _this.previousScene), log.info("Stream went online switching to scene: \"".concat(_this.previousScene, "\""))), bitrate <= _this.lowBitrateTrigger && currentScene.name !== _this.lowBitrateScene && bitrate !== 0 && (_this.obs.send("SetCurrentScene", {
            "scene-name": _this.lowBitrateScene
          }), _this.previousScene = _this.lowBitrateScene, _this.switchSceneEmit("lowBitrateScene"), log.info("Low bitrate detected switching to scene: \"".concat(_this.lowBitrateScene, "\""))), bitrate > _this.lowBitrateTrigger && currentScene.name !== _this.normalScene && (_this.obs.send("SetCurrentScene", {
            "scene-name": _this.normalScene
          }), _this.previousScene = _this.normalScene, _this.switchSceneEmit("normalScene"), log.info("Switching to normal scene: \"".concat(_this.normalScene, "\""))));
        }
      } else {
        _this.isLive = false;
        canSwitch && currentScene.name !== _this.offlineScene && (_this.obs.send("SetCurrentScene", {
          "scene-name": _this.offlineScene
        }), _this.switchSceneEmit("offlineScene"), _this.streamStatus = null, log.warn("Error receiving current bitrate or stream is offline. Switching to offline scene: \"".concat(_this.offlineScene, "\"")));
      }
    })();
  }

  onAuth() {
    log.success("Successfully connected");
    this.obs.send("SetHeartbeat", {
      enable: true
    });
    this.getSceneList();
    this.interval = setInterval(this.switchSceneIfNecessary.bind(this), _config.default.obs.requestMs);
  }

  switchSceneEmit(sceneName, args) {
    if (_config.default.twitchChat.enableAutoSwitchNotification && this.obsStreaming) {
      this.emit(sceneName, args);
    }
  }

  getBitrate() {
    var _this2 = this;

    return _asyncToGenerator(function* () {
      var {
        server,
        stats,
        application,
        key,
        id,
        publisher
      } = _config.default.rtmp;

      switch (server) {
        case "nginx":
          try {
            var response = yield (0, _nodeFetch.default)(stats);
            var data = yield response.text();
            parseString(data, (err, result) => {
              var publish = result.rtmp.server[0].application.find(stream => {
                return stream.name[0] === application;
              }).live[0].stream;
              var stream = publish === null || publish === void 0 ? void 0 : publish.find(stream => {
                return stream.name[0] === key;
              });

              if (stream != null) {
                _this2.nginxVideoMeta = stream.meta[0].video[0];
                _this2.bitrate = Math.round(stream.bw_video[0] / 1024);
              } else {
                _this2.nginxVideoMeta = null;
                _this2.bitrate = null;
              }
            });
          } catch (e) {
            log.error("[NGINX] Error fetching stats");
          }

          break;

        case "node-media-server":
          try {
            var _response = yield (0, _nodeFetch.default)("".concat(stats, "/").concat(application, "/").concat(key));

            var _data = yield _response.json();

            _this2.bitrate = _data.bitrate || null;
          } catch (e) {
            log.error("[NMS] Error fetching stats, is the API http server running?");
          }

          break;

        case "nimble":
          try {
            // SRT stats to see RTT and if streaming is active
            var srtresponse = yield (0, _nodeFetch.default)(stats + "/manage/srt_receiver_stats");
            var srtdata = yield srtresponse.json();
            var srtreceiver = srtdata.SrtReceivers.filter(receiver => receiver.id.includes(id));
            var publish = srtreceiver[0].state;

            if (publish == "disconnected") {
              _this2.bitrate = null;
              _this2.rtt = null;
            } else {
              // RTMP status for bitrate. srt_receiver_stats seems to give an averaged number that isn't as useful.
              // Probably requires nimble to be configured to make the video from SRT available on RTMP even though it's not used anywhere
              var rtmpresponse = yield (0, _nodeFetch.default)(stats + "/manage/rtmp_status");
              var rtmpdata = yield rtmpresponse.json();
              var rtmpstream = rtmpdata.filter(rtmp => rtmp.app == application)[0].streams.filter(stream => stream.strm == key);
              _this2.bitrate = Math.round(rtmpstream[0].bandwidth / 1024);
              _this2.rtt = srtreceiver[0].stats.link.rtt;
            }
          } catch (e) {
            log.error("[NIMBLE] Error fetching stats: " + e);
          }

          break;

        case "srt-live-server":
          try {
            var _stream$rtt;

            var _srtresponse = yield (0, _nodeFetch.default)(stats);

            var _srtdata = yield _srtresponse.json();

            var stream = _srtdata.publishers[publisher];
            _this2.bitrate = (stream === null || stream === void 0 ? void 0 : stream.bitrate) || null;
            _this2.rtt = (_stream$rtt = stream === null || stream === void 0 ? void 0 : stream.rtt) !== null && _stream$rtt !== void 0 ? _stream$rtt : null;
          } catch (e) {
            log.error("[SLS] Error fetching stats: " + e);
          }

          break;

        default:
          log.error("[STATS] Something went wrong at getting the RTMP server, did you enter the correct name in the config?");
          break;
      }

      return [_this2.bitrate, _this2.rtt];
    })();
  }

  setStreamStatus(res) {
    this.streamStatus = res;
  }

  error(e) {
    log.error(e);
  }

  onDisconnect() {
    log.error("Can't connect or lost connnection");
    clearInterval(this.interval);
    this.reconnect();
  }

  onAuthFail() {
    log.error("Failed to authenticate");
  }

  reconnect() {
    log.info("Trying to reconnect in 5 seconds");
    setTimeout(() => {
      this.obs.connect({
        address: this.address,
        password: this.password
      }).catch(e => {// handle this somewhere else
      });
    }, 5000);
  }

  streamStopped() {
    var _this3 = this;

    return _asyncToGenerator(function* () {
      _this3.obsStreaming = false;
      _this3.nginxVideoMeta = null;
      _this3.bitrate = null;
      var {
        canSwitch
      } = yield _this3.canSwitch();

      if (canSwitch) {
        _this3.obs.send("SetCurrentScene", {
          "scene-name": _this3.offlineScene
        });
      }
    })();
  }

  streamStarted() {
    this.obsStreaming = true;
  }

  getSceneList() {
    var _this4 = this;

    return _asyncToGenerator(function* () {
      var list = yield _this4.obs.send("GetSceneList");
      _this4.scenes = list.scenes;
    })();
  }

  scenesChanged() {
    this.getSceneList();
  }

  handleHeartbeat(heartbeat) {
    this.heartbeat = heartbeat;
    this.obsStreaming = heartbeat.streaming;
  }

  canSwitch() {
    var _this5 = this;

    return _asyncToGenerator(function* () {
      var currentScene = yield _this5.obs.send("GetCurrentScene");
      var canSwitch = currentScene.name == _this5.lowBitrateScene || currentScene.name == _this5.normalScene || currentScene.name == _this5.offlineScene;
      _this5.currentScene = currentScene.name;
      return {
        currentScene,
        canSwitch
      };
    })();
  }

}

var _default = ObsSwitcher;
exports.default = _default;