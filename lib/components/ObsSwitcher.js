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
  function ObsSwitcher(address, password, low, normal, offline) {
    _classCallCheck(this, ObsSwitcher);

    this.obs = new _obsWebsocketJs.default();
    this.isLive = false;
    this.address = address;
    this.password = password;
    this.lowBitrateScene = low;
    this.normalScene = normal;
    this.offlineScene = offline;
    this.obs.connect({
      address: this.ipObs,
      password: this.passwordObs
    }).then(function () {
      return console.log("Success! We're connected & authenticated.");
    }).catch(function () {
      return console.error("Can't connect to OBS, did you enter the correct ip?");
    });
    this.obs.on("AuthenticationSuccess", this.onAuth.bind(this));
    this.obs.on("error", this.error.bind(this));
  }

  _createClass(ObsSwitcher, [{
    key: "onAuth",
    value: function onAuth() {
      var _this = this;

      setInterval(
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
                return ObsSwitcher.getBitrate();

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
    key: "error",
    value: function error(e) {
      console.log(e);
    }
  }], [{
    key: "getBitrate",
    value: function () {
      var _getBitrate = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee2() {
        var username,
            response,
            data,
            bitrate,
            _args2 = arguments;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                username = _args2.length > 0 && _args2[0] !== undefined ? _args2[0] : "live";
                _context2.next = 3;
                return (0, _nodeFetch.default)("http://".concat(_config.default.ipNginx, "/stat"));

              case 3:
                response = _context2.sent;
                _context2.next = 6;
                return response.text();

              case 6:
                data = _context2.sent;
                parseString(data, function (err, result) {
                  var publish = result.rtmp.server[0].application[0].live[0].stream;

                  if (publish == null) {
                    bitrate = null;
                  } else {
                    var stream = publish.find(function (stream) {
                      return stream.name[0] === username;
                    });
                    bitrate = stream.bw_video[0] / 1024;
                  }
                });
                return _context2.abrupt("return", bitrate);

              case 9:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this);
      }));

      function getBitrate() {
        return _getBitrate.apply(this, arguments);
      }

      return getBitrate;
    }()
  }]);

  return ObsSwitcher;
}();

var _default = ObsSwitcher;
exports.default = _default;