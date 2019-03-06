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

    _this.obs.on("Heartbeat", function (heartbeat) {
      return _this.heartbeat = heartbeat;
    });

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
      this.interval = setInterval(
      /*#__PURE__*/
      _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee() {
        var currentScene, bitrate, canSwitch;
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return _this2.obs.GetCurrentScene();

              case 2:
                currentScene = _context.sent;
                _context.next = 5;
                return _this2.getBitrate();

              case 5:
                bitrate = _context.sent;
                canSwitch = currentScene.name == _this2.lowBitrateScene || currentScene.name == _this2.normalScene || currentScene.name == _this2.offlineScene;
                _this2.currentScene = currentScene.name;

                if (bitrate !== null) {
                  _this2.isLive = true;
                  _this2.isLive && canSwitch && (bitrate === 0 && currentScene.name !== _this2.lowBitrateScene && (_this2.obs.setCurrentScene({
                    "scene-name": _this2.lowBitrateScene
                  }), _this2.switchSceneEmit("live"), log.info("Stream went online switching to scene: \"".concat(_this2.lowBitrateScene, "\""))), bitrate <= _this2.lowBitrateTrigger && currentScene.name !== _this2.lowBitrateScene && bitrate !== 0 && (_this2.obs.setCurrentScene({
                    "scene-name": _this2.lowBitrateScene
                  }), _this2.switchSceneEmit("lowBitrateScene"), log.info("Low bitrate detected switching to scene: \"".concat(_this2.lowBitrateScene, "\""))), bitrate > _this2.lowBitrateTrigger && currentScene.name !== _this2.normalScene && (_this2.obs.setCurrentScene({
                    "scene-name": _this2.normalScene
                  }), _this2.switchSceneEmit("normalScene"), log.info("Switching to normal scene: \"".concat(_this2.normalScene, "\""))));
                } else {
                  _this2.isLive = false;
                  canSwitch && currentScene.name !== _this2.offlineScene && (_this2.obs.setCurrentScene({
                    "scene-name": _this2.offlineScene
                  }), _this2.switchSceneEmit("offlineScene"), _this2.streamStatus = null, log.warn("Error receiving current bitrate or stream is offline. Switching to offline scene: \"".concat(_this2.offlineScene, "\"")));
                }

              case 9:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      })), _config.default.obs.requestMs);
    }
  }, {
    key: "switchSceneEmit",
    value: function switchSceneEmit(sceneName) {
      if (_config.default.twitchChat.enableAutoSwitchNotification && this.obsStreaming) {
        this.emit(sceneName);
      }
    }
  }, {
    key: "getBitrate",
    value: function () {
      var _getBitrate = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee2() {
        var _this3 = this;

        var username,
            response,
            data,
            _args2 = arguments;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                username = _args2.length > 0 && _args2[0] !== undefined ? _args2[0] : "live";
                _context2.prev = 1;
                _context2.next = 4;
                return (0, _nodeFetch.default)("http://".concat(_config.default.nginx.ip, "/stat"));

              case 4:
                response = _context2.sent;
                _context2.next = 7;
                return response.text();

              case 7:
                data = _context2.sent;
                parseString(data, function (err, result) {
                  var publish = result.rtmp.server[0].application[0].live[0].stream;

                  if (publish == null) {
                    _this3.nginxVideoMeta = null;
                    _this3.bitrate = null;
                  } else {
                    var stream = publish.find(function (stream) {
                      return stream.name[0] === username;
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
    value: function streamStopped() {
      this.obsStreaming = false;
    }
  }, {
    key: "streamStarted",
    value: function streamStarted() {
      this.obsStreaming = true;
    }
  }]);

  return ObsSwitcher;
}(_events.default);

var _default = ObsSwitcher;
exports.default = _default;