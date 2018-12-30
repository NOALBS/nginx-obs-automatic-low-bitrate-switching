"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

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
    this.options = ["host", "unhost", "start", "stop", "switch"];
  }

  _createClass(ChatCommands, [{
    key: "host",
    value: function host(username) {
      var _this = this;

      if (username != null) {
        this.ws.send("PRIVMSG #".concat(this.channel, " :/host ").concat(username));
        setTimeout(function () {
          _this.stop();
        }, 5000);
      } else {
        console.log("Error executing host command no username");
      }
    }
  }, {
    key: "unhost",
    value: function unhost() {
      this.ws.send("PRIVMSG #".concat(this.channel, " :/unhost"));
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
                _context.next = 8;
                break;

              case 5:
                _context.prev = 5;
                _context.t0 = _context["catch"](0);
                console.log(_context.t0);

              case 8:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this, [[0, 5]]);
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
                _context2.next = 8;
                break;

              case 5:
                _context2.prev = 5;
                _context2.t0 = _context2["catch"](0);
                console.log(_context2.t0);

              case 8:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[0, 5]]);
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
                _context3.next = 8;
                break;

              case 5:
                _context3.prev = 5;
                _context3.t0 = _context3["catch"](0);
                console.log(_context3.t0);

              case 8:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[0, 5]]);
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