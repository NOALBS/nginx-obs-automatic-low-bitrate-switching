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
    key: "onAuth",
    value: function onAuth() {
      var _this2 = this;

      log.success("Successfully connected");
      this.obs.SetHeartbeat({
        enable: true
      });
      this.getSceneList();
      this.interval = setInterval(
      /*#__PURE__*/
      _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee() {
        var bitrate, _ref2, currentScene, canSwitch;

        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                if (_this2.obsStreaming) {
                  _context.next = 2;
                  break;
                }

                return _context.abrupt("return");

              case 2:
                _context.next = 4;
                return _this2.getBitrate();

              case 4:
                bitrate = _context.sent;
                _context.next = 7;
                return _this2.canSwitch();

              case 7:
                _ref2 = _context.sent;
                currentScene = _ref2.currentScene;
                canSwitch = _ref2.canSwitch;

                if (bitrate !== null) {
                  _this2.isLive = true;
                  _this2.isLive && canSwitch && (bitrate === 0 && currentScene.name !== _this2.previousScene && (_this2.obs.setCurrentScene({
                    "scene-name": _this2.previousScene
                  }), _this2.switchSceneEmit("live", _this2.previousScene), log.info("Stream went online switching to scene: \"".concat(_this2.previousScene, "\""))), bitrate <= _this2.lowBitrateTrigger && currentScene.name !== _this2.lowBitrateScene && bitrate !== 0 && (_this2.obs.setCurrentScene({
                    "scene-name": _this2.lowBitrateScene
                  }), _this2.previousScene = _this2.lowBitrateScene, _this2.switchSceneEmit("lowBitrateScene"), log.info("Low bitrate detected switching to scene: \"".concat(_this2.lowBitrateScene, "\""))), bitrate > _this2.lowBitrateTrigger && currentScene.name !== _this2.normalScene && (_this2.obs.setCurrentScene({
                    "scene-name": _this2.normalScene
                  }), _this2.previousScene = _this2.normalScene, _this2.switchSceneEmit("normalScene"), log.info("Switching to normal scene: \"".concat(_this2.normalScene, "\""))));
                } else {
                  _this2.isLive = false;
                  canSwitch && currentScene.name !== _this2.offlineScene && (_this2.obs.setCurrentScene({
                    "scene-name": _this2.offlineScene
                  }), _this2.switchSceneEmit("offlineScene"), _this2.streamStatus = null, log.warn("Error receiving current bitrate or stream is offline. Switching to offline scene: \"".concat(_this2.offlineScene, "\"")));
                }

              case 11:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      })), _config.default.obs.requestMs);
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
        var _this3 = this;

        var rtmp, response, data;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                if (!this.nginxSettings) {
                  rtmp = /rtmp:\/\/(.*)\/(\w+)\/(\w+)/g.exec(_config.default.nginx.rtmp);
                  this.nginxSettings = {
                    ip: rtmp[1],
                    application: rtmp[2],
                    key: rtmp[3]
                  };
                }

                _context2.prev = 1;
                _context2.next = 4;
                return (0, _nodeFetch.default)("http://".concat(this.nginxSettings.ip, "/stat"));

              case 4:
                response = _context2.sent;
                _context2.next = 7;
                return response.text();

              case 7:
                data = _context2.sent;
                parseString(data, function (err, result) {
                  var publish = result.rtmp.server[0].application.find(function (stream) {
                    return stream.name[0] === _this3.nginxSettings.application;
                  }).live[0].stream;

                  if (publish == null) {
                    _this3.nginxVideoMeta = null;
                    _this3.bitrate = null;
                  } else {
                    var stream = publish.find(function (stream) {
                      return stream.name[0] === _this3.nginxSettings.key;
                    });
                    _this3.nginxVideoMeta = stream.meta[0].video[0];
                    _this3.bitrate = Math.round(stream.bw_video[0] / 1024);
                  }
                });
                _context2.next = 14;
                break;

              case 11:
                _context2.prev = 11;
                _context2.t0 = _context2["catch"](1);
                log.error("[NGINX] Error fetching stats");

              case 14:
                return _context2.abrupt("return", this.bitrate);

              case 15:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[1, 11]]);
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
      var _this4 = this;

      log.info("Trying to reconnect in 5 seconds");
      setTimeout(function () {
        _this4.obs.connect({
          address: _this4.address,
          password: _this4.password
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
        var _ref3, canSwitch;

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
                _ref3 = _context3.sent;
                canSwitch = _ref3.canSwitch;

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