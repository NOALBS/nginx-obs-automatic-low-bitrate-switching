"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _ws = _interopRequireDefault(require("ws"));

var _config = _interopRequireDefault(require("../../config"));

var _fs = _interopRequireDefault(require("fs"));

var _signale = _interopRequireDefault(require("signale"));

var _fastFuzzy = require("fast-fuzzy");

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

_signale.default.config({
  displayTimestamp: true,
  displayDate: true
});

var log = _signale.default.scope("CHT");

var Chat =
/*#__PURE__*/
function () {
  function Chat(username, password, channel, obs) {
    _classCallCheck(this, Chat);

    this.username = username; // username

    this.password = password; // oauth

    this.channel = "#".concat(channel); // #channel

    this.obsProps = obs;
    this.obs = obs.obs;
    this.prefix = _config.default.twitchChat.prefix;
    this.commands = ["host", "unhost", "start", "stop", "switch", "raid", "bitrate", "refresh", "trigger", "sourceinfo", "obsinfo", "public", "mod", "notify", "autostop", "rec", "noalbs"];
    this.allowAllCommands = _config.default.twitchChat.publicCommands;
    this.allowModsCommands = _config.default.twitchChat.modCommands;
    this.wait = false;
    this.rate = 0;
    this.rateInterval = false;
    this.isRefreshing = false;
    this.open();
    this.obsProps.on("live", this.live.bind(this));
    this.obsProps.on("normalScene", this.onNormalScene.bind(this));
    this.obsProps.on("lowBitrateScene", this.onLowBitrateScene.bind(this));
    this.obsProps.on("offlineScene", this.onOfflineScene.bind(this));
    log.info("Connecting to twitch");
  }

  _createClass(Chat, [{
    key: "open",
    value: function open() {
      this.ws = new _ws.default("wss://irc-ws.chat.twitch.tv:443");
      this.ws.onopen = this.onOpen.bind(this);
      this.ws.onmessage = this.onMessage.bind(this);
      this.ws.onerror = this.onError.bind(this);
      this.ws.onclose = this.onClose.bind(this);
    }
  }, {
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
        log.success("Successfully Connected");
        log.success("Authenticating and joining channel ".concat(this.channel));
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
      log.error("Disconnected from twitch server");
      clearInterval(this.interval);
      this.ws.removeAllListeners();
      this.reconnect();
    }
  }, {
    key: "close",
    value: function close() {
      if (this.ws) {
        this.ws.close();
      }
    }
  }, {
    key: "reconnect",
    value: function reconnect() {
      var _this2 = this;

      log.info("Trying to reconnect in 5 seconds");
      setTimeout(function () {
        log.info("Reconnecting...");

        _this2.open();
      }, 5000);
    }
  }, {
    key: "onError",
    value: function onError(e) {
      log.error(new Error(e));
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
                log.success("".concat(parsed.username, " Executed ").concat(commandName, " command"));
                this.setWait();
              } else {
                log.error("".concat(parsed.username, " Executed unknown command ").concat(commandName));
              }
            }
          } else if (parsed.command === "PING") {
            this.ws.send("PONG :".concat(parsed.message));
          } else if (parsed.command === "HOSTTARGET") {
            if (parsed.message != null && _config.default.twitchChat.enableAutoStopStreamOnHostOrRaid && this.obsProps.bitrate != null) {
              log.info("Channel started hosting, stopping stream");
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
      var _this3 = this;

      this.rate++;

      if (!this.rateInterval) {
        this.rateInterval = true;
        setTimeout(function () {
          _this3.rate = 0;
          _this3.rateInterval = false;
        }, 30000);
      }

      if (!this.wait) {
        this.wait = true;
        setTimeout(function () {
          _this3.wait = false;
        }, 2000);
      }
    }
  }, {
    key: "host",
    value: function host(username) {
      if (username != null) {
        this.say("/host ".concat(username));
      } else {
        this.say("Error no username"); // console.log("Error executing host command no username");
      }
    }
  }, {
    key: "unhost",
    value: function unhost() {
      this.say("/unhost");
    }
  }, {
    key: "raid",
    value: function raid(username) {
      if (username != null) {
        this.say("/raid ".concat(username));
      } else {
        this.say("Error no username"); // console.log("Error executing host command no username");
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
                this.say("Successfully started stream");
                _context.next = 10;
                break;

              case 6:
                _context.prev = 6;
                _context.t0 = _context["catch"](0);
                log.error(_context.t0);
                this.say("Error ".concat(_context.t0.error));

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
                this.say("Successfully stopped stream");
                _context2.next = 10;
                break;

              case 6:
                _context2.prev = 6;
                _context2.t0 = _context2["catch"](0);
                log.error(_context2.t0.error);
                this.say("".concat(_context2.t0.error));

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
    key: "rec",
    value: function rec(bool) {
      if (!bool) {
        this.say("[REC] ".concat(this.obsProps.heartbeat.recording ? "started" : "stopped"));
        return;
      }

      switch (bool) {
        case "on":
          this.startStopRec(true);
          return;

        case "off":
          this.startStopRec(false);
          return;

        default:
          this.say("[REC] Invalid option");
          return;
      }
    }
  }, {
    key: "startStopRec",
    value: function () {
      var _startStopRec = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee3(bool) {
        var res, _res;

        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                if (!bool) {
                  _context3.next = 14;
                  break;
                }

                _context3.prev = 1;
                _context3.next = 4;
                return this.obs.StartRecording();

              case 4:
                res = _context3.sent;
                if (res.status === "ok") this.say("[REC] Started");
                log.success("Started recording");
                _context3.next = 12;
                break;

              case 9:
                _context3.prev = 9;
                _context3.t0 = _context3["catch"](1);
                this.say("[REC] already started");

              case 12:
                _context3.next = 25;
                break;

              case 14:
                _context3.prev = 14;
                _context3.next = 17;
                return this.obs.StopRecording();

              case 17:
                _res = _context3.sent;
                if (_res.status === "ok") this.say("[REC] Stopped");
                log.success("Stopped recording");
                _context3.next = 25;
                break;

              case 22:
                _context3.prev = 22;
                _context3.t1 = _context3["catch"](14);
                this.say("[REC] already stopped");

              case 25:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[1, 9], [14, 22]]);
      }));

      function startStopRec(_x) {
        return _startStopRec.apply(this, arguments);
      }

      return startStopRec;
    }()
  }, {
    key: "switch",
    value: function () {
      var _switch2 = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee4(sceneName) {
        var res, scene;
        return regeneratorRuntime.wrap(function _callee4$(_context4) {
          while (1) {
            switch (_context4.prev = _context4.next) {
              case 0:
                res = (0, _fastFuzzy.search)(sceneName, this.obsProps.scenes, {
                  keySelector: function keySelector(obj) {
                    return obj.name;
                  }
                });
                scene = res.length > 0 ? res[0].name : sceneName;
                _context4.prev = 2;
                _context4.next = 5;
                return this.obs.setCurrentScene({
                  "scene-name": scene
                });

              case 5:
                this.say("Scene successfully switched to \"".concat(scene, "\""));
                _context4.next = 12;
                break;

              case 8:
                _context4.prev = 8;
                _context4.t0 = _context4["catch"](2);
                log.error(_context4.t0);
                this.say(_context4.t0.error);

              case 12:
              case "end":
                return _context4.stop();
            }
          }
        }, _callee4, this, [[2, 8]]);
      }));

      function _switch(_x2) {
        return _switch2.apply(this, arguments);
      }

      return _switch;
    }()
  }, {
    key: "bitrate",
    value: function bitrate() {
      if (this.obsProps.bitrate != null) {
        this.say("Current bitrate: ".concat(this.obsProps.bitrate, " Kbps"));
      } else {
        this.say("Current bitrate: offline");
      }
    }
  }, {
    key: "sourceinfo",
    value: function sourceinfo() {
      if (this.obsProps.nginxVideoMeta != null) {
        var _this$obsProps$nginxV = this.obsProps.nginxVideoMeta,
            height = _this$obsProps$nginxV.height,
            frame_rate = _this$obsProps$nginxV.frame_rate;
        this.say("[SRC] R: ".concat(height[0], " | F: ").concat(frame_rate[0], " | B: ").concat(this.obsProps.bitrate));
      } else {
        this.say("[SRC] offline");
      }
    }
  }, {
    key: "obsinfo",
    value: function obsinfo() {
      if (this.obsProps.streamStatus != null) {
        var _this$obsProps$stream = this.obsProps.streamStatus,
            fps = _this$obsProps$stream.fps,
            kbitsPerSec = _this$obsProps$stream.kbitsPerSec;
        this.say("[OBS] S: ".concat(this.obsProps.currentScene, " | F: ").concat(Math.round(fps), " | B: ").concat(kbitsPerSec));
      } else {
        this.say("[OBS] offline");
      }
    }
  }, {
    key: "refresh",
    value: function () {
      var _refresh = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee5() {
        var _this4 = this;

        var lastScene;
        return regeneratorRuntime.wrap(function _callee5$(_context5) {
          while (1) {
            switch (_context5.prev = _context5.next) {
              case 0:
                if (this.isRefreshing) {
                  _context5.next = 13;
                  break;
                }

                _context5.prev = 1;
                lastScene = this.obsProps.currentScene;
                _context5.next = 5;
                return this.obs.setCurrentScene({
                  "scene-name": _config.default.obs.refreshScene
                });

              case 5:
                this.say("Refreshing stream");
                this.isRefreshing = true;
                setTimeout(function () {
                  _this4.obs.setCurrentScene({
                    "scene-name": lastScene
                  });

                  _this4.say("Refreshing stream completed");

                  _this4.isRefreshing = false;
                }, _config.default.obs.refreshSceneInterval);
                _context5.next = 13;
                break;

              case 10:
                _context5.prev = 10;
                _context5.t0 = _context5["catch"](1);
                log.error(_context5.t0);

              case 13:
              case "end":
                return _context5.stop();
            }
          }
        }, _callee5, this, [[1, 10]]);
      }));

      function refresh() {
        return _refresh.apply(this, arguments);
      }

      return refresh;
    }()
  }, {
    key: "live",
    value: function live(previous) {
      // this.ws.send(`PRIVMSG ${this.channel} :Scene switching to live`);
      this.say("Scene switched to \"".concat(previous, "\""));
    }
  }, {
    key: "onNormalScene",
    value: function onNormalScene() {
      this.say("Scene switched to \"".concat(_config.default.obs.normalScene, "\""));
      this.bitrate();
    }
  }, {
    key: "onLowBitrateScene",
    value: function onLowBitrateScene() {
      this.say("Scene switched to \"".concat(_config.default.obs.lowBitrateScene, "\""));
      this.bitrate();
    }
  }, {
    key: "onOfflineScene",
    value: function onOfflineScene() {
      // this.ws.send(`PRIVMSG ${this.channel} :Stream went offline`);
      this.say("Scene switched to \"".concat(_config.default.obs.offlineScene, "\""));
    }
  }, {
    key: "trigger",
    value: function trigger(number) {
      if (number) {
        if (!isNaN(number)) {
          this.obsProps.lowBitrateTrigger = +number;
          _config.default.obs.lowBitrateTrigger = +number;
          this.handleWriteToConfig();
          this.say("Trigger successfully set to ".concat(this.obsProps.lowBitrateTrigger, " Kbps"));
        } else {
          this.say("Error editing trigger ".concat(number, " is not a valid value"));
        }

        return;
      }

      this.say("Current trigger set at ".concat(this.obsProps.lowBitrateTrigger, " Kbps"));
    }
  }, {
    key: "public",
    value: function _public(bool) {
      this.handleEnable("enablePublicCommands", bool, "Public comands");
    }
  }, {
    key: "mod",
    value: function mod(bool) {
      this.handleEnable("enableModCommands", bool, "Mod commands");
    }
  }, {
    key: "notify",
    value: function notify(bool) {
      this.handleEnable("enableAutoSwitchNotification", bool, "Auto switch notification");
    }
  }, {
    key: "autostop",
    value: function autostop(bool) {
      this.handleEnable("enableAutoStopStreamOnHostOrRaid", bool, "Auto stop stream");
    }
  }, {
    key: "handleEnable",
    value: function handleEnable(name, bool, response) {
      if (!bool) {
        this.say("".concat(response, " is ").concat(_config.default.twitchChat[name] ? "enabled" : "disabled"));
        return;
      }

      if (bool === "on" && _config.default.twitchChat[name] != true) {
        _config.default.twitchChat[name] = true;
        this.handleWriteToConfig();
        this.say("".concat(response, " enabled"));
      } else if (bool === "off" && _config.default.twitchChat[name] != false) {
        _config.default.twitchChat[name] = false;
        this.handleWriteToConfig();
        this.say("".concat(response, " disabled"));
      } else {
        this.say("".concat(response, " already ").concat(_config.default.twitchChat[name] ? "enabled" : "disabled"));
      }
    }
  }, {
    key: "handleWriteToConfig",
    value: function handleWriteToConfig() {
      _fs.default.writeFile('"../../config.json', JSON.stringify(_config.default, null, 4), function (err) {
        if (err) log.error(err);
      });
    }
  }, {
    key: "say",
    value: function say(message) {
      this.ws.send("PRIVMSG ".concat(this.channel, " :").concat(message));
    }
  }, {
    key: "noalbs",
    value: function noalbs(a) {
      if (a === "version") this.say("Running NOALBS v".concat(process.env.npm_package_version));
    }
  }]);

  return Chat;
}();

var _default = Chat;
exports.default = _default;