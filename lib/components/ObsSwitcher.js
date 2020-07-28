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

function _typeof(obj) { if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { _typeof = function _typeof(obj) { return typeof obj; }; } else { _typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return _typeof(obj); }

function _slicedToArray(arr, i) { return _arrayWithHoles(arr) || _iterableToArrayLimit(arr, i) || _nonIterableRest(); }

function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance"); }

function _iterableToArrayLimit(arr, i) { var _arr = []; var _n = true; var _d = false; var _e = undefined; try { for (var _i = arr[Symbol.iterator](), _s; !(_n = (_s = _i.next()).done); _n = true) { _arr.push(_s.value); if (i && _arr.length === i) break; } } catch (err) { _d = true; _e = err; } finally { try { if (!_n && _i["return"] != null) _i["return"](); } finally { if (_d) throw _e; } } return _arr; }

function _arrayWithHoles(arr) { if (Array.isArray(arr)) return arr; }

function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

function _possibleConstructorReturn(self, call) { if (call && (_typeof(call) === "object" || typeof call === "function")) { return call; } return _assertThisInitialized(self); }

function _getPrototypeOf(o) { _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return _getPrototypeOf(o); }

function _inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) _setPrototypeOf(subClass, superClass); }

function _setPrototypeOf(o, p) { _setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return _setPrototypeOf(o, p); }

function _assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

_signale.default.config({
  displayTimestamp: true,
  displayDate: true
});

var log = _signale.default.scope("OBS");

var parseString = _xml2js.default.parseString;

var ObsSwitcher =
/*#__PURE__*/
function (_EventEmitter) {
  _inherits(ObsSwitcher, _EventEmitter);

  function ObsSwitcher(address, password, low, normal, offline, lowBitrateTrigger) {
    var _this;

    var highRttTrigger = arguments.length > 6 && arguments[6] !== undefined ? arguments[6] : 2500;

    _classCallCheck(this, ObsSwitcher);

    _this = _possibleConstructorReturn(this, _getPrototypeOf(ObsSwitcher).call(this));
    _this.obs = new _obsWebsocketJs.default();
    _this.isLive = false;
    _this.address = address;
    _this.password = password;
    _this.lowBitrateScene = low;
    _this.normalScene = normal;
    _this.offlineScene = offline;
    _this.lowBitrateTrigger = lowBitrateTrigger;
    _this.highRttTrigger = highRttTrigger;
    _this.bitrate = null;
    _this.nginxVideoMeta = null;
    _this.streamStatus = null;
    _this.heartbeat = null;
    _this.obsStreaming = false;
    _this.currentScene = null;
    _this.nginxSettings;
    _this.previousScene = _this.lowBitrateScene;
    _this.scenes = null;

    _this.obs.connect({
      address: _this.address,
      password: _this.password
    }).catch(function (e) {// handle this somewhere else
    });

    _this.obs.on("ConnectionClosed", _this.onDisconnect.bind(_assertThisInitialized(_assertThisInitialized(_this))));

    _this.obs.on("AuthenticationSuccess", _this.onAuth.bind(_assertThisInitialized(_assertThisInitialized(_this))));

    _this.obs.on("AuthenticationFailure", _this.onAuthFail.bind(_assertThisInitialized(_assertThisInitialized(_this))));

    _this.obs.on("error", _this.error.bind(_assertThisInitialized(_assertThisInitialized(_this))));

    _this.obs.on("StreamStatus", _this.setStreamStatus.bind(_assertThisInitialized(_assertThisInitialized(_this))));

    _this.obs.on("StreamStopped", _this.streamStopped.bind(_assertThisInitialized(_assertThisInitialized(_this))));

    _this.obs.on("StreamStarted", _this.streamStarted.bind(_assertThisInitialized(_assertThisInitialized(_this))));

    _this.obs.on("Heartbeat", _this.handleHeartbeat.bind(_assertThisInitialized(_assertThisInitialized(_this))));

    _this.obs.on("ScenesChanged", _this.scenesChanged.bind(_assertThisInitialized(_assertThisInitialized(_this))));

    log.info("Connecting & authenticating");
    return _this;
  }

  _createClass(ObsSwitcher, [{
    key: "switchSceneIfNecessary",
    value: function () {
      var _switchSceneIfNecessary = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee() {
        var _ref, _ref2, bitrate, rtt, _ref3, currentScene, canSwitch;

        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                if (!(!this.obsStreaming && (_config.default.obs.onlySwitchWhenStreaming == null || _config.default.obs.onlySwitchWhenStreaming))) {
                  _context.next = 2;
                  break;
                }

                return _context.abrupt("return");

              case 2:
                _context.next = 4;
                return this.getBitrate();

              case 4:
                _ref = _context.sent;
                _ref2 = _slicedToArray(_ref, 2);
                bitrate = _ref2[0];
                rtt = _ref2[1];
                _context.next = 10;
                return this.canSwitch();

              case 10:
                _ref3 = _context.sent;
                currentScene = _ref3.currentScene;
                canSwitch = _ref3.canSwitch;

                if (bitrate !== null) {
                  this.isLive = true;

                  if (_config.default.rtmp.server === "nimble") {
                    this.isLive && canSwitch && (bitrate === 0 && currentScene.name !== this.previousScene && (this.obs.setCurrentScene({
                      "scene-name": this.previousScene
                    }), this.switchSceneEmit("live", this.previousScene), log.info("Stream went online switching to scene: \"".concat(this.previousScene, "\""))), (rtt < this.highRttTrigger || rtt >= this.highRttTrigger) && bitrate <= this.lowBitrateTrigger && currentScene.name !== this.lowBitrateScene && bitrate !== 0 && (this.obs.setCurrentScene({
                      "scene-name": this.lowBitrateScene
                    }), this.previousScene = this.lowBitrateScene, this.switchSceneEmit("lowBitrateScene"), log.info("Low bitrate detected switching to scene: \"".concat(this.lowBitrateScene, "\""))), rtt >= this.highRttTrigger && bitrate > this.lowBitrateTrigger && currentScene.name !== this.lowBitrateScene && bitrate !== 0 && (this.obs.setCurrentScene({
                      "scene-name": this.lowBitrateScene
                    }), this.previousScene = this.lowBitrateScene, this.switchSceneEmit("lowBitrateScene"), log.info("High RTT detected switching to scene: \"".concat(this.lowBitrateScene, "\""))), rtt < this.highRttTrigger && bitrate > this.lowBitrateTrigger && currentScene.name !== this.normalScene && (this.obs.setCurrentScene({
                      "scene-name": this.normalScene
                    }), this.previousScene = this.normalScene, this.switchSceneEmit("normalScene"), log.info("Switching to normal scene: \"".concat(this.normalScene, "\""))));
                  } else {
                    this.isLive && canSwitch && (bitrate === 0 && currentScene.name !== this.previousScene && (this.obs.setCurrentScene({
                      "scene-name": this.previousScene
                    }), this.switchSceneEmit("live", this.previousScene), log.info("Stream went online switching to scene: \"".concat(this.previousScene, "\""))), bitrate <= this.lowBitrateTrigger && currentScene.name !== this.lowBitrateScene && bitrate !== 0 && (this.obs.setCurrentScene({
                      "scene-name": this.lowBitrateScene
                    }), this.previousScene = this.lowBitrateScene, this.switchSceneEmit("lowBitrateScene"), log.info("Low bitrate detected switching to scene: \"".concat(this.lowBitrateScene, "\""))), bitrate > this.lowBitrateTrigger && currentScene.name !== this.normalScene && (this.obs.setCurrentScene({
                      "scene-name": this.normalScene
                    }), this.previousScene = this.normalScene, this.switchSceneEmit("normalScene"), log.info("Switching to normal scene: \"".concat(this.normalScene, "\""))));
                  }
                } else {
                  this.isLive = false;
                  canSwitch && currentScene.name !== this.offlineScene && (this.obs.setCurrentScene({
                    "scene-name": this.offlineScene
                  }), this.switchSceneEmit("offlineScene"), this.streamStatus = null, log.warn("Error receiving current bitrate or stream is offline. Switching to offline scene: \"".concat(this.offlineScene, "\"")));
                }

              case 14:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function switchSceneIfNecessary() {
        return _switchSceneIfNecessary.apply(this, arguments);
      }

      return switchSceneIfNecessary;
    }()
  }, {
    key: "onAuth",
    value: function onAuth() {
      log.success("Successfully connected");
      this.obs.SetHeartbeat({
        enable: true
      });
      this.getSceneList();
      this.interval = setInterval(this.switchSceneIfNecessary.bind(this), _config.default.obs.requestMs);
    }
  }, {
    key: "switchSceneEmit",
    value: function switchSceneEmit(sceneName, args) {
      if (_config.default.twitchChat.enableAutoSwitchNotification && this.obsStreaming) {
        this.emit(sceneName, args);
      }
    }
  }, {
    key: "getBitrate",
    value: function () {
      var _getBitrate = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee2() {
        var _this2 = this;

        var _config$rtmp, server, stats, application, key, id, response, data, _response, _data, srtresponse, srtdata, srtreceiver, publish, rtmpresponse, rtmpdata, rtmpstream;

        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                _config$rtmp = _config.default.rtmp, server = _config$rtmp.server, stats = _config$rtmp.stats, application = _config$rtmp.application, key = _config$rtmp.key, id = _config$rtmp.id;
                _context2.t0 = server;
                _context2.next = _context2.t0 === "nginx" ? 4 : _context2.t0 === "node-media-server" ? 18 : _context2.t0 === "nimble" ? 32 : 61;
                break;

              case 4:
                _context2.prev = 4;
                _context2.next = 7;
                return (0, _nodeFetch.default)(stats);

              case 7:
                response = _context2.sent;
                _context2.next = 10;
                return response.text();

              case 10:
                data = _context2.sent;
                parseString(data, function (err, result) {
                  var publish = result.rtmp.server[0].application.find(function (stream) {
                    return stream.name[0] === application;
                  }).live[0].stream;

                  if (publish == null) {
                    _this2.nginxVideoMeta = null;
                    _this2.bitrate = null;
                  } else {
                    var stream = publish.find(function (stream) {
                      return stream.name[0] === key;
                    });
                    _this2.nginxVideoMeta = stream.meta[0].video[0];
                    _this2.bitrate = Math.round(stream.bw_video[0] / 1024);
                  }
                });
                _context2.next = 17;
                break;

              case 14:
                _context2.prev = 14;
                _context2.t1 = _context2["catch"](4);
                log.error("[NGINX] Error fetching stats");

              case 17:
                return _context2.abrupt("break", 63);

              case 18:
                _context2.prev = 18;
                _context2.next = 21;
                return (0, _nodeFetch.default)("".concat(stats, "/").concat(application, "/").concat(key));

              case 21:
                _response = _context2.sent;
                _context2.next = 24;
                return _response.json();

              case 24:
                _data = _context2.sent;
                this.bitrate = _data.bitrate || null;
                _context2.next = 31;
                break;

              case 28:
                _context2.prev = 28;
                _context2.t2 = _context2["catch"](18);
                log.error("[NMS] Error fetching stats, is the API http server running?");

              case 31:
                return _context2.abrupt("break", 63);

              case 32:
                _context2.prev = 32;
                _context2.next = 35;
                return (0, _nodeFetch.default)(stats + "/manage/srt_receiver_stats");

              case 35:
                srtresponse = _context2.sent;
                _context2.next = 38;
                return srtresponse.json();

              case 38:
                srtdata = _context2.sent;
                srtreceiver = srtdata.SrtReceivers.filter(function (receiver) {
                  return receiver.id == id;
                });
                publish = srtreceiver[0].state;

                if (!(publish == "disconnected")) {
                  _context2.next = 46;
                  break;
                }

                this.bitrate = null;
                this.rtt = null;
                _context2.next = 55;
                break;

              case 46:
                _context2.next = 48;
                return (0, _nodeFetch.default)(stats + "/manage/rtmp_status");

              case 48:
                rtmpresponse = _context2.sent;
                _context2.next = 51;
                return rtmpresponse.json();

              case 51:
                rtmpdata = _context2.sent;
                rtmpstream = rtmpdata.filter(function (rtmp) {
                  return rtmp.app == application;
                })[0].streams.filter(function (stream) {
                  return stream.strm == key;
                });
                this.bitrate = Math.round(rtmpstream[0].bandwidth / 1024);
                this.rtt = srtreceiver[0].stats.link.rtt;

              case 55:
                _context2.next = 60;
                break;

              case 57:
                _context2.prev = 57;
                _context2.t3 = _context2["catch"](32);
                log.error("[NIMBLE] Error fetching stats: " + _context2.t3);

              case 60:
                return _context2.abrupt("break", 63);

              case 61:
                log.error("[STATS] Something went wrong at getting the RTMP server, did you enter the correct name in the config?");
                return _context2.abrupt("break", 63);

              case 63:
                return _context2.abrupt("return", [this.bitrate, this.rtt]);

              case 64:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[4, 14], [18, 28], [32, 57]]);
      }));

      function getBitrate() {
        return _getBitrate.apply(this, arguments);
      }

      return getBitrate;
    }()
  }, {
    key: "setStreamStatus",
    value: function setStreamStatus(res) {
      this.streamStatus = res;
    }
  }, {
    key: "error",
    value: function error(e) {
      log.error(e);
    }
  }, {
    key: "onDisconnect",
    value: function onDisconnect() {
      log.error("Can't connect or lost connnection");
      clearInterval(this.interval);
      this.reconnect();
    }
  }, {
    key: "onAuthFail",
    value: function onAuthFail() {
      log.error("Failed to authenticate");
    }
  }, {
    key: "reconnect",
    value: function reconnect() {
      var _this3 = this;

      log.info("Trying to reconnect in 5 seconds");
      setTimeout(function () {
        _this3.obs.connect({
          address: _this3.address,
          password: _this3.password
        }).catch(function (e) {// handle this somewhere else
        });
      }, 5000);
    }
  }, {
    key: "streamStopped",
    value: function () {
      var _streamStopped = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee3() {
        var _ref4, canSwitch;

        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.obsStreaming = false;
                this.nginxVideoMeta = null;
                this.bitrate = null;
                _context3.next = 5;
                return this.canSwitch();

              case 5:
                _ref4 = _context3.sent;
                canSwitch = _ref4.canSwitch;

                if (canSwitch) {
                  this.obs.setCurrentScene({
                    "scene-name": this.offlineScene
                  });
                }

              case 8:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this);
      }));

      function streamStopped() {
        return _streamStopped.apply(this, arguments);
      }

      return streamStopped;
    }()
  }, {
    key: "streamStarted",
    value: function streamStarted() {
      this.obsStreaming = true;
    }
  }, {
    key: "getSceneList",
    value: function () {
      var _getSceneList = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee4() {
        var list;
        return regeneratorRuntime.wrap(function _callee4$(_context4) {
          while (1) {
            switch (_context4.prev = _context4.next) {
              case 0:
                _context4.next = 2;
                return this.obs.GetSceneList();

              case 2:
                list = _context4.sent;
                this.scenes = list.scenes;

              case 4:
              case "end":
                return _context4.stop();
            }
          }
        }, _callee4, this);
      }));

      function getSceneList() {
        return _getSceneList.apply(this, arguments);
      }

      return getSceneList;
    }()
  }, {
    key: "scenesChanged",
    value: function scenesChanged() {
      this.getSceneList();
    }
  }, {
    key: "handleHeartbeat",
    value: function handleHeartbeat(heartbeat) {
      this.heartbeat = heartbeat;
      this.obsStreaming = heartbeat.streaming;
    }
  }, {
    key: "canSwitch",
    value: function () {
      var _canSwitch = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee5() {
        var currentScene, canSwitch;
        return regeneratorRuntime.wrap(function _callee5$(_context5) {
          while (1) {
            switch (_context5.prev = _context5.next) {
              case 0:
                _context5.next = 2;
                return this.obs.GetCurrentScene();

              case 2:
                currentScene = _context5.sent;
                canSwitch = currentScene.name == this.lowBitrateScene || currentScene.name == this.normalScene || currentScene.name == this.offlineScene;
                this.currentScene = currentScene.name;
                return _context5.abrupt("return", {
                  currentScene: currentScene,
                  canSwitch: canSwitch
                });

              case 6:
              case "end":
                return _context5.stop();
            }
          }
        }, _callee5, this);
      }));

      function canSwitch() {
        return _canSwitch.apply(this, arguments);
      }

      return canSwitch;
    }()
  }]);

  return ObsSwitcher;
}(_events.default);

var _default = ObsSwitcher;
exports.default = _default;