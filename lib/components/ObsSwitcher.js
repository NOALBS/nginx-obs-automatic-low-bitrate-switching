"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _obsWebsocketJs = _interopRequireDefault(require("obs-websocket-js"));

var _request = _interopRequireDefault(require("request"));

var _xml2js = _interopRequireDefault(require("xml2js"));

var _config = _interopRequireDefault(require("../../config"));

var _Chat = _interopRequireDefault(require("./Chat"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

var ObsSwitcher =
/*#__PURE__*/
function () {
  function ObsSwitcher() {
    _classCallCheck(this, ObsSwitcher);

    this.obs = new _obsWebsocketJs.default();
    this.parseString = _xml2js.default.parseString;
    this.isLive = false;
  }

  _createClass(ObsSwitcher, [{
    key: "connect",
    value: function connect() {
      this.obs.connect({
        address: _config.default.ipObs,
        password: _config.default.passwordObs
      }).then(function () {
        return console.log("Success! We're connected & authenticated.");
      }).catch(function () {
        return console.error("Can't connect to OBS, did you enter the correct ip?");
      });
      this.obs.on("AuthenticationSuccess", this.onAuth.bind(this));
      this.obs.on("error", this.error.bind(this));
    }
  }, {
    key: "onAuth",
    value: function onAuth() {
      var _this = this;

      new _Chat.default(_config.default.twitchUsername, _config.default.twitchOauth, "#".concat(_config.default.twitchUsername), this.obs);
      setInterval(function () {
        (0, _request.default)("http://".concat(_config.default.ipNginx, "/stat"), function (error, response, body) {
          if (error) console.log("Can't access nginx");

          _this.parseString(body,
          /*#__PURE__*/
          function () {
            var _ref = _asyncToGenerator(
            /*#__PURE__*/
            regeneratorRuntime.mark(function _callee(err, result) {
              var currentScene, canSwitch, bitrate;
              return regeneratorRuntime.wrap(function _callee$(_context) {
                while (1) {
                  switch (_context.prev = _context.next) {
                    case 0:
                      _context.next = 2;
                      return _this.obs.GetCurrentScene();

                    case 2:
                      currentScene = _context.sent;
                      canSwitch = currentScene.name == _config.default.lowBitrateScene || currentScene.name == _config.default.normalScene || currentScene.name == _config.default.offlineScene;

                      try {
                        bitrate = result.rtmp.server[0].application[0].live[0].stream[0].bw_video[0] / 1024;
                        _this.isLive = true;
                        _this.isLive && canSwitch && (bitrate === 0 && _this.obs.setCurrentScene({
                          "scene-name": _config.default.lowBitrateScene
                        }), bitrate <= _config.default.lowBitrateTrigger && currentScene.name !== _config.default.lowBitrateScene && bitrate !== 0 && (_this.obs.setCurrentScene({
                          "scene-name": _config.default.lowBitrateScene
                        }), console.log("Low bitrate detected switching to scene ".concat(_config.default.lowBitrateScene, "."))), bitrate > _config.default.lowBitrateTrigger && currentScene.name !== _config.default.normalScene && (_this.obs.setCurrentScene({
                          "scene-name": _config.default.normalScene
                        }), console.log("Switching back to scene ".concat(_config.default.normalScene, "."))));
                      } catch (e) {
                        _this.isLive = false;
                        canSwitch && currentScene.name !== _config.default.offlineScene && (_this.obs.setCurrentScene({
                          "scene-name": _config.default.offlineScene
                        }), console.log("Error receiving current bitrate or steam is offline. Switching to scene ".concat(_config.default.offlineScene, ".")));
                      }

                    case 5:
                    case "end":
                      return _context.stop();
                  }
                }
              }, _callee, this);
            }));

            return function (_x, _x2) {
              return _ref.apply(this, arguments);
            };
          }());
        });
      }, _config.default.requestMs);
    }
  }, {
    key: "error",
    value: function error(e) {
      console.log(e);
    }
  }]);

  return ObsSwitcher;
}();

var _default = ObsSwitcher;
exports.default = _default;