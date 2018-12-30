"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _config = _interopRequireDefault(require("../../config"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

var ChatCommands =
/*#__PURE__*/
function () {
  function ChatCommands(_, prefix) {
    _classCallCheck(this, ChatCommands);

    this.ws = _.ws;
    this.channel = _.username;
    this.obs = _.obs;
    this.prefix = prefix;
    this.options = ["host", "unhost", "start", "stop", "switch", "raid"];
  }

  _createClass(ChatCommands, [{
    key: "host",
    value: function host(username) {
      var _this = this;

      if (username != null) {
        this.ws.send("PRIVMSG #".concat(this.channel, " :/host ").concat(username));
        setTimeout(function () {
          _this.stop();
        }, _config.default.stopStreamOnHostInterval);
      } else {
        this.ws.send("PRIVMSG #".concat(this.channel, " :Error no username"));
        console.log("Error executing host command no username");
      }
    }
  }, {
    key: "unhost",
    value: function unhost() {
      this.ws.send("PRIVMSG #".concat(this.channel, " :/unhost"));
    }
  }, {
    key: "raid",
    value: function raid(username) {
      var _this2 = this;

      if (username != null) {
        this.ws.send("PRIVMSG #".concat(this.channel, " :/raid ").concat(username));
        setTimeout(function () {
          _this2.stop();
        }, _config.default.stopStreamOnRaidInterval);
      } else {
        console.log("Error executing host command no username");
        this.ws.send("PRIVMSG #".concat(this.channel, " :Error no username"));
      }
    }
  }, {
    key: "start",
    value: function () {
      var _start = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.prev = 0;
                _context.next = 3;
                return this.obs.startStreaming();

              case 3:
                this.ws.send("PRIVMSG #".concat(this.channel, " :Successfully started stream"));
                _context.next = 10;
                break;

              case 6:
                _context.prev = 6;
                _context.t0 = _context["catch"](0);
                console.log(_context.t0);
                this.ws.send("PRIVMSG #".concat(this.channel, " :Error ").concat(_context.t0.error));

              case 10:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this, [[0, 6]]);
      }));

      function start() {
        return _start.apply(this, arguments);
      }

      return start;
    }()
  }, {
    key: "stop",
    value: function () {
      var _stop = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee2() {
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                _context2.prev = 0;
                _context2.next = 3;
                return this.obs.stopStreaming();

              case 3:
                this.ws.send("PRIVMSG #".concat(this.channel, " :Successfully stopped stream"));
                _context2.next = 10;
                break;

              case 6:
                _context2.prev = 6;
                _context2.t0 = _context2["catch"](0);
                console.log(_context2.t0.error);
                this.ws.send("PRIVMSG #".concat(this.channel, " :").concat(_context2.t0.error));

              case 10:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[0, 6]]);
      }));

      function stop() {
        return _stop.apply(this, arguments);
      }

      return stop;
    }()
  }, {
    key: "switch",
    value: function () {
      var _switch2 = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee3(sceneName) {
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                _context3.prev = 0;
                _context3.next = 3;
                return this.obs.setCurrentScene({
                  "scene-name": sceneName
                });

              case 3:
                this.ws.send("PRIVMSG #".concat(this.channel, " :Scene successfully switched to \"").concat(sceneName, "\""));
                _context3.next = 9;
                break;

              case 6:
                _context3.prev = 6;
                _context3.t0 = _context3["catch"](0);
                console.log(_context3.t0);

              case 9:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[0, 6]]);
      }));

      function _switch(_x) {
        return _switch2.apply(this, arguments);
      }

      return _switch;
    }()
  }]);

  return ChatCommands;
}();

var _default = ChatCommands;
exports.default = _default;