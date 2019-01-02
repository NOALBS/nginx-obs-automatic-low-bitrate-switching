"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _obsWebsocketJs = _interopRequireDefault(require("obs-websocket-js"));

var _nodeFetch = _interopRequireDefault(require("node-fetch"));

var _xml2js = _interopRequireDefault(require("xml2js"));

var _config = _interopRequireDefault(require("../../config"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

var parseString = _xml2js.default.parseString;

var ObsSwitcher =
/*#__PURE__*/
function () {
  function ObsSwitcher(address, password, low, normal, offline, lowBitrateTrigger) {
    _classCallCheck(this, ObsSwitcher);

    this.obs = new _obsWebsocketJs.default();
    this.isLive = false;
    this.address = address;
    this.password = password;
    this.lowBitrateScene = low;
    this.normalScene = normal;
    this.offlineScene = offline;
    this.lowBitrateTrigger = lowBitrateTrigger;
    this.bitrate = null;
    this.obs.connect({
      address: this.address,
      password: this.password
    }).catch(function (e) {// handle this somewhere else
    });
    this.obs.on("ConnectionClosed", this.onDisconnect.bind(this));
    this.obs.on("AuthenticationSuccess", this.onAuth.bind(this));
    this.obs.on("AuthenticationFailure", this.onAuthFail.bind(this));
    this.obs.on("error", this.error.bind(this));
  }

  _createClass(ObsSwitcher, [{
    key: "onAuth",
    value: function onAuth() {
      var _this = this;

      console.log("Success! We're connected & authenticated.");
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
                return _this.obs.GetCurrentScene();

              case 2:
                currentScene = _context.sent;
                _context.next = 5;
                return _this.getBitrate();

              case 5:
                bitrate = _context.sent;
                canSwitch = currentScene.name == _this.lowBitrateScene || currentScene.name == _this.normalScene || currentScene.name == _this.offlineScene;

                if (bitrate !== null) {
                  _this.isLive = true;
                  _this.isLive && canSwitch && (bitrate === 0 && _this.obs.setCurrentScene({
                    "scene-name": _this.lowBitrateScene
                  }), bitrate <= _this.lowBitrateTrigger && currentScene.name !== _this.lowBitrateScene && bitrate !== 0 && (_this.obs.setCurrentScene({
                    "scene-name": _this.lowBitrateScene
                  }), console.log("Low bitrate detected switching to scene ".concat(_this.lowBitrateScene, "."))), bitrate > _this.lowBitrateTrigger && currentScene.name !== _this.normalScene && (_this.obs.setCurrentScene({
                    "scene-name": _this.normalScene
                  }), console.log("Switching back to scene ".concat(_this.normalScene, "."))));
                } else {
                  _this.isLive = false;
                  canSwitch && currentScene.name !== _this.offlineScene && (_this.obs.setCurrentScene({
                    "scene-name": _this.offlineScene
                  }), console.log("Error receiving current bitrate or steam is offline. Switching to scene ".concat(_this.offlineScene, ".")));
                }

              case 8:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      })), _config.default.requestMs);
    }
  }, {
    key: "getBitrate",
    value: function () {
      var _getBitrate = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee2() {
        var _this2 = this;

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
                return (0, _nodeFetch.default)("http://".concat(_config.default.ipNginx, "/stat"));

              case 4:
                response = _context2.sent;
                _context2.next = 7;
                return response.text();

              case 7:
                data = _context2.sent;
                parseString(data, function (err, result) {
                  var publish = result.rtmp.server[0].application[0].live[0].stream;

                  if (publish == null) {
                    _this2.bitrate = null;
                  } else {
                    var stream = publish.find(function (stream) {
                      return stream.name[0] === username;
                    });
                    _this2.bitrate = stream.bw_video[0] / 1024;
                  }
                });
                _context2.next = 14;
                break;

              case 11:
                _context2.prev = 11;
                _context2.t0 = _context2["catch"](1);
                console.log("Error fetching stats");

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
    key: "error",
    value: function error(e) {
      console.log(e);
    }
  }, {
    key: "onDisconnect",
    value: function onDisconnect() {
      console.error("Can't connect to OBS or lost connnection.");
      clearInterval(this.interval);
      this.reconnect();
    }
  }, {
    key: "onAuthFail",
    value: function onAuthFail() {
      console.log("Failed to authenticate to OBS");
    }
  }, {
    key: "reconnect",
    value: function reconnect() {
      var _this3 = this;

      console.log("Trying to reconnect in 5 seconds");
      setTimeout(function () {
        _this3.obs.connect({
          address: _this3.address,
          password: _this3.password
        }).catch(function (e) {// handle this somewhere else
        });
      }, 5000);
    }
  }]);

  return ObsSwitcher;
}();

var _default = ObsSwitcher;
exports.default = _default;