"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _ws = _interopRequireDefault(require("ws"));

var _config = _interopRequireDefault(require("../../config"));

var _fs = _interopRequireDefault(require("fs"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

// TODO add reconnect on disconnect
var Chat =
/*#__PURE__*/
function () {
  function Chat(username, password, channel, obs) {
    _classCallCheck(this, Chat);

    this.username = username; // username

    this.password = password; // oauth

    this.channel = "#".concat(channel); // #channel

    this.ws = new _ws.default("wss://irc-ws.chat.twitch.tv:443");
    this.obsProps = obs;
    this.obs = obs.obs;
    this.prefix = "!";
    this.commands = ["host", "unhost", "start", "stop", "switch", "raid", "bitrate", "info", "refresh", "trigger"];
    this.allowAllCommands = ["bitrate", "info"];
    this.allowModsCommands = ["refresh", "trigger"];
    this.wait = false;
    this.rate = 0;
    this.rateInterval = false;
    this.isRefreshing = false;
    this.ws.onopen = this.onOpen.bind(this);
    this.ws.onmessage = this.onMessage.bind(this);
    this.ws.onerror = this.onError.bind(this);
    this.ws.onclose = this.onClose.bind(this);
    this.obsProps.on("live", this.live.bind(this));
    this.obsProps.on("normalScene", this.onNormalScene.bind(this));
    this.obsProps.on("lowBitrateScene", this.onLowBitrateScene.bind(this));
    this.obsProps.on("offlineScene", this.onOfflineScene.bind(this));
  }

  _createClass(Chat, [{
    key: "keepAlive",
    value: function keepAlive() {
      var _this = this;

      this.interval = setInterval(function () {
        _this.ws.send("PING :tmi.twitch.tv\r\n");
      }, 2000);
    }
  }, {
    key: "onOpen",
    value: function onOpen() {
      if (this.ws !== null && this.ws.readyState === 1) {
        console.log("Successfully Connected to websocket");
        console.log("Authenticating and joining channel ".concat(this.channel));
        this.ws.send("CAP REQ :twitch.tv/tags twitch.tv/commands");
        this.ws.send("PASS ".concat(this.password));
        this.ws.send("NICK ".concat(this.username));
        this.ws.send("JOIN ".concat(this.channel));
        this.keepAlive();
      }
    }
  }, {
    key: "onClose",
    value: function onClose() {
      console.log("Disconnected from the chat server.");
      clearInterval(this.interval);
    }
  }, {
    key: "close",
    value: function close() {
      if (this.ws) {
        this.ws.close();
      }
    }
  }, {
    key: "onError",
    value: function onError(e) {
      console.log("Error: ".concat(e));
    }
  }, {
    key: "onMessage",
    value: function onMessage(message) {
      if (message !== null) {
        var parsed = this.parse(message.data);

        if (parsed !== null) {
          if (parsed.command === "PRIVMSG") {
            // not a command
            if (parsed.message.substr(0, 1) !== this.prefix) return; // Split the message into individual words:

            var parse = parsed.message.slice(1).split(" ");
            var commandName = parse[0];

            if (_config.default.twitchChat.adminUsers.includes(parsed.username) && this.rate != 20 || _config.default.twitchChat.enablePublicCommands && this.allowAllCommands.includes(commandName) && !this.wait && this.rate != 20 || _config.default.twitchChat.enableModCommands && parsed.tags.mod === "1" && this.allowModsCommands.includes(commandName) && this.rate != 20 || parsed.username === this.channel.substring(1) && this.rate != 20) {
              if (this.commands.includes(commandName)) {
                this[commandName](parse[1]);
                console.log("! Executed ".concat(commandName, " command"));
                this.setWait();
              } else {
                console.log("! Unknown command ".concat(commandName));
              }
            }
          } else if (parsed.command === "PING") {
            this.ws.send("PONG :".concat(parsed.message));
          } else if (parsed.command === "HOSTTARGET") {
            if (parsed.message != null && _config.default.twitchChat.enableAutoStopStreamOnHostOrRaid && this.obsProps.bitrate != null) {
              console.log("Channel started hosting stopping stream.");
              this.stop();
            }
          }
        }
      }
    }
  }, {
    key: "parse",
    value: function parse(message) {
      var regex = RegExp(/([A-Z]\w*)/, "g");
      var array = regex.exec(message);
      var parsedMessage = {
        tags: {},
        channel: null,
        command: null,
        username: null,
        message: null,
        raw: message
      };
      var firstString = message.split(" ", 1)[0];

      if (message[0] === "@") {
        var space = message.indexOf(" ");
        var tagsRaw = message.slice(1, space);
        var tagsSplit = tagsRaw.split(";");
        tagsSplit.map(function (d) {
          var tagSplit = d.split("=");
          parsedMessage.tags[tagSplit[0]] = tagSplit[1];
        });
        var userIndex = message.indexOf("!");
        parsedMessage.username = message.slice(space + 2, userIndex);
        var commandIndex = message.indexOf(" ", userIndex);
        var channelIndex = message.indexOf("#", space);
        parsedMessage.command = message.slice(commandIndex + 1, channelIndex - 1);
        var messageIndex = message.indexOf(":", commandIndex);
        parsedMessage.channel = message.slice(channelIndex, messageIndex - 1);
        parsedMessage.message = message.slice(messageIndex + 1, message.length - 2);
      } else if (firstString === "PING") {
        parsedMessage.command = "PING";
        parsedMessage.message = message.split(":")[1];
      } else if (array[0] == "HOSTTARGET") {
        var res = message.match(/:([\w]+)/g);
        parsedMessage.command = "HOSTTARGET";
        parsedMessage.message = res[1];
      }

      return parsedMessage;
    }
  }, {
    key: "setWait",
    value: function setWait() {
      var _this2 = this;

      this.rate++;

      if (!this.rateInterval) {
        this.rateInterval = true;
        setTimeout(function () {
          _this2.rate = 0;
          _this2.rateInterval = false;
        }, 30000);
      }

      if (!this.wait) {
        this.wait = true;
        setTimeout(function () {
          _this2.wait = false;
        }, 2000);
      }
    }
  }, {
    key: "host",
    value: function host(username) {
      if (username != null) {
        this.ws.send("PRIVMSG ".concat(this.channel, " :/host ").concat(username));
      } else {
        this.ws.send("PRIVMSG ".concat(this.channel, " :Error no username"));
        console.log("Error executing host command no username");
      }
    }
  }, {
    key: "unhost",
    value: function unhost() {
      this.ws.send("PRIVMSG ".concat(this.channel, " :/unhost"));
    }
  }, {
    key: "raid",
    value: function raid(username) {
      if (username != null) {
        this.ws.send("PRIVMSG ".concat(this.channel, " :/raid ").concat(username));
      } else {
        console.log("Error executing host command no username");
        this.ws.send("PRIVMSG ".concat(this.channel, " :Error no username"));
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
                this.ws.send("PRIVMSG ".concat(this.channel, " :Successfully started stream"));
                _context.next = 10;
                break;

              case 6:
                _context.prev = 6;
                _context.t0 = _context["catch"](0);
                console.log(_context.t0);
                this.ws.send("PRIVMSG ".concat(this.channel, " :Error ").concat(_context.t0.error));

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
                this.ws.send("PRIVMSG ".concat(this.channel, " :Successfully stopped stream"));
                _context2.next = 10;
                break;

              case 6:
                _context2.prev = 6;
                _context2.t0 = _context2["catch"](0);
                console.log(_context2.t0.error);
                this.ws.send("PRIVMSG ".concat(this.channel, " :").concat(_context2.t0.error));

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
                this.ws.send("PRIVMSG ".concat(this.channel, " :Scene successfully switched to \"").concat(sceneName, "\""));
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
  }, {
    key: "bitrate",
    value: function bitrate() {
      this.ws.send("PRIVMSG ".concat(this.channel, " :Current bitrate: ").concat(this.obsProps.bitrate, " Kbps"));
    }
  }, {
    key: "info",
    value: function info() {
      this.ws.send("PRIVMSG ".concat(this.channel, " :Current scene: ").concat(this.obsProps.currentScene, " and bitrate: ").concat(this.obsProps.bitrate, " Kbps"));
    }
  }, {
    key: "refresh",
    value: function () {
      var _refresh = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee4() {
        var _this3 = this;

        return regeneratorRuntime.wrap(function _callee4$(_context4) {
          while (1) {
            switch (_context4.prev = _context4.next) {
              case 0:
                if (this.isRefreshing) {
                  _context4.next = 12;
                  break;
                }

                _context4.prev = 1;
                _context4.next = 4;
                return this.obs.setCurrentScene({
                  "scene-name": _config.default.obs.refreshScene
                });

              case 4:
                this.ws.send("PRIVMSG ".concat(this.channel, " :Refreshing stream"));
                this.isRefreshing = true;
                setTimeout(function () {
                  _this3.obs.setCurrentScene({
                    "scene-name": _config.default.obs.normalScene
                  });

                  _this3.ws.send("PRIVMSG ".concat(_this3.channel, " :Refreshing stream completed"));

                  _this3.isRefreshing = false;
                }, _config.default.obs.refreshSceneInterval);
                _context4.next = 12;
                break;

              case 9:
                _context4.prev = 9;
                _context4.t0 = _context4["catch"](1);
                console.log(_context4.t0);

              case 12:
              case "end":
                return _context4.stop();
            }
          }
        }, _callee4, this, [[1, 9]]);
      }));

      function refresh() {
        return _refresh.apply(this, arguments);
      }

      return refresh;
    }()
  }, {
    key: "live",
    value: function live() {
      // this.ws.send(`PRIVMSG ${this.channel} :Scene switching to live`);
      this.ws.send("PRIVMSG ".concat(this.channel, " :Scene switched to \"").concat(_config.default.obs.lowBitrateScene, "\""));
    }
  }, {
    key: "onNormalScene",
    value: function onNormalScene() {
      this.ws.send("PRIVMSG ".concat(this.channel, " :Scene switched to \"").concat(_config.default.obs.normalScene, "\""));
      this.bitrate();
    }
  }, {
    key: "onLowBitrateScene",
    value: function onLowBitrateScene() {
      this.ws.send("PRIVMSG ".concat(this.channel, " :Scene switched to \"").concat(_config.default.obs.lowBitrateScene, "\""));
      this.bitrate();
    }
  }, {
    key: "onOfflineScene",
    value: function onOfflineScene() {
      // this.ws.send(`PRIVMSG ${this.channel} :Stream went offline`);
      this.ws.send("PRIVMSG ".concat(this.channel, " :Scene switched to \"").concat(_config.default.obs.offlineScene, "\""));
    }
  }, {
    key: "trigger",
    value: function trigger(number) {
      if (number) {
        if (!isNaN(number)) {
          this.obsProps.lowBitrateTrigger = +number;
          _config.default.obs.lowBitrateTrigger = +number;

          _fs.default.writeFile('"../../config.json', JSON.stringify(_config.default, null, 2), function (err) {
            if (err) console.log(err);
          });

          this.ws.send("PRIVMSG ".concat(this.channel, " :Trigger successfully set to ").concat(this.obsProps.lowBitrateTrigger, " Kbps"));
        } else {
          this.ws.send("PRIVMSG ".concat(this.channel, " :Error editing trigger ").concat(number, " is not a valid value"));
        }

        return;
      }

      this.ws.send("PRIVMSG ".concat(this.channel, " :Current trigger set at ").concat(this.obsProps.lowBitrateTrigger, " Kbps"));
    }
  }]);

  return Chat;
}();

var _default = Chat;
exports.default = _default;