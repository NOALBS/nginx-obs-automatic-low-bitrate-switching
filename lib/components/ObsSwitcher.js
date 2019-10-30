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
        var bitrate, _ref, currentScene, canSwitch;

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
                bitrate = _context.sent;
                _context.next = 7;
                return this.canSwitch();

              case 7:
                _ref = _context.sent;
                currentScene = _ref.currentScene;
                canSwitch = _ref.canSwitch;

                if (bitrate !== null) {
                  this.isLive = true;
                  this.isLive && canSwitch && (bitrate === 0 && currentScene.name !== this.previousScene && (this.obs.setCurrentScene({
                    "scene-name": this.previousScene
                  }), this.switchSceneEmit("live", this.previousScene), log.info("Stream went online switching to scene: \"".concat(this.previousScene, "\""))), bitrate <= this.lowBitrateTrigger && currentScene.name !== this.lowBitrateScene && bitrate !== 0 && (this.obs.setCurrentScene({
                    "scene-name": this.lowBitrateScene
                  }), this.previousScene = this.lowBitrateScene, this.switchSceneEmit("lowBitrateScene"), log.info("Low bitrate detected switching to scene: \"".concat(this.lowBitrateScene, "\""))), bitrate > this.lowBitrateTrigger && currentScene.name !== this.normalScene && (this.obs.setCurrentScene({
                    "scene-name": this.normalScene
                  }), this.previousScene = this.normalScene, this.switchSceneEmit("normalScene"), log.info("Switching to normal scene: \"".concat(this.normalScene, "\""))));
                } else {
                  this.isLive = false;
                  canSwitch && currentScene.name !== this.offlineScene && (this.obs.setCurrentScene({
                    "scene-name": this.offlineScene
                  }), this.switchSceneEmit("offlineScene"), this.streamStatus = null, log.warn("Error receiving current bitrate or stream is offline. Switching to offline scene: \"".concat(this.offlineScene, "\"")));
                }

              case 11:
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

        var _config$rtmp, server, stats, application, key, response, data;

        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                _config$rtmp = _config.default.rtmp, server = _config$rtmp.server, stats = _config$rtmp.stats, application = _config$rtmp.application, key = _config$rtmp.key;
                _context2.t0 = server;
                _context2.next = _context2.t0 === "nginx" ? 4 : 18;
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
                return _context2.abrupt("break", 20);

              case 18:
                log.error("[STATS] Something went wrong at getting the RTMP server, did you enter the correct name in the config?");
                return _context2.abrupt("break", 20);

              case 20:
                return _context2.abrupt("return", this.bitrate);

              case 21:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[4, 14]]);
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
        var _ref2, canSwitch;

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
                _ref2 = _context3.sent;
                canSwitch = _ref2.canSwitch;

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