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

var _nodeFetch = _interopRequireDefault(require("node-fetch"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function _toConsumableArray(arr) { return _arrayWithoutHoles(arr) || _iterableToArray(arr) || _nonIterableSpread(); }

function _nonIterableSpread() { throw new TypeError("Invalid attempt to spread non-iterable instance"); }

function _arrayWithoutHoles(arr) { if (Array.isArray(arr)) { for (var i = 0, arr2 = new Array(arr.length); i < arr.length; i++) { arr2[i] = arr[i]; } return arr2; } }

function _toArray(arr) { return _arrayWithHoles(arr) || _iterableToArray(arr) || _nonIterableRest(); }

function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance"); }

function _iterableToArray(iter) { if (Symbol.iterator in Object(iter) || Object.prototype.toString.call(iter) === "[object Arguments]") return Array.from(iter); }

function _arrayWithHoles(arr) { if (Array.isArray(arr)) return arr; }

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
    this.commands = ["host", "unhost", "start", "stop", "switch", "raid", "bitrate", "refresh", "trigger", "sourceinfo", "obsinfo", "public", "mod", "notify", "autostop", "rec", "noalbs", "fix", "alias"];
    this.aliases = {
      o: "obsinfo",
      s: "sourceinfo",
      b: "bitrate",
      r: "refresh",
      ss: "switch"
    };
    this.allowAllCommands = _config.default.twitchChat.publicCommands;
    this.allowModsCommands = _config.default.twitchChat.modCommands;
    this.wait = false;
    this.rate = 0;
    this.rateInterval = false;
    this.isRefreshing = false;
    this.open();
    this.registerAliases();
    this.obsProps.on("live", this.live.bind(this));
    this.obsProps.on("normalScene", this.onNormalScene.bind(this));
    this.obsProps.on("lowBitrateScene", this.onLowBitrateScene.bind(this));
    this.obsProps.on("offlineScene", this.onOfflineScene.bind(this));
  }

  _createClass(Chat, [{
    key: "open",
    value: function open() {
      log.info("Connecting to twitch");
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
        if (_this.sendPing) return;

        _this.ws.send("PING :tmi.twitch.tv\r\n");

        _this.sendPing = new Date().getTime();
        _this.pingTimeout = setTimeout(function () {
          log.error("Didn't receive PONG in time.. reconnecting to twitch.");

          _this.close();

          _this.sendPing = null;
        }, 1000 * 10);
      }, 1000 * 60 * 2);
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

        switch (parsed.command) {
          case "PRIVMSG":
            this.handleMessage(parsed);
            break;

          case "HOSTTARGET":
            if (_config.default.twitchChat.enableAutoStopStreamOnHostOrRaid && this.obsProps.bitrate != null) {
              log.info("Channel started hosting, stopping stream");
              this.stop();
            }

            break;

          case "PING":
            this.ws.send("PONG ".concat(parsed.channel));
            break;

          case "PONG":
            var ms = new Date().getTime() - this.sendPing; // console.log(`Pong received after ${ms} ms`);

            clearTimeout(this.pingTimeout);
            this.sendPing = null;
            break;

          default:
            break;
        }
      }
    }
  }, {
    key: "parse",
    value: function parse(message) {
      var parsedMessage = {
        tags: {},
        channel: null,
        command: null,
        username: null,
        message: null,
        raw: message
      }; // tags

      if (message.startsWith("@")) {
        var space = message.indexOf(" ");
        var tagsRaw = message.slice(1, space);
        var tagsSplit = tagsRaw.split(";");
        tagsSplit.map(function (d) {
          var tagSplit = d.split("=");
          if (tagSplit[1] == "") tagSplit[1] = null;
          parsedMessage.tags[tagSplit[0]] = tagSplit[1];
        });
      }

      message = message.slice(space + 1).trim().split(" ");
      var pos = 0;

      if (message[0].startsWith(":")) {
        parsedMessage.username = message[0].substring(1, message[0].indexOf("!"));
        pos += 1;
      }

      parsedMessage.command = message[pos];
      parsedMessage.channel = message[pos + 1];
      if (!message[pos + 2] == "") parsedMessage.message = message.slice(3).join(" ").slice(1);
      return parsedMessage;
    }
  }, {
    key: "handleMessage",
    value: function handleMessage(msg) {
      if (!msg.message.startsWith(this.prefix)) return;

      var _msg$message$slice$sp = msg.message.slice(1).split(" "),
          _msg$message$slice$sp2 = _toArray(_msg$message$slice$sp),
          commandName = _msg$message$slice$sp2[0],
          params = _msg$message$slice$sp2.slice(1);

      if (commandName in this.aliases) {
        commandName = this.aliases[commandName];
      }

      switch (true) {
        case commandName == "noalbs":
        case _config.default.twitchChat.adminUsers.includes(msg.username):
        case _config.default.twitchChat.enableModCommands && msg.tags.mod === "1" && this.allowModsCommands.includes(commandName):
        case _config.default.twitchChat.enablePublicCommands && !this.wait && this.allowAllCommands.includes(commandName):
        case msg.username === this.channel.substring(1):
          if (this.rate == 20) return;
          if (!this.commands.includes(commandName)) return;
          this[commandName].apply(this, _toConsumableArray(params));
          log.success("".concat(msg.username, " Executed ").concat(commandName, " command"));
          this.setWait();
          break;

        default:
          break;
      }
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
                if (!(sceneName == null)) {
                  _context4.next = 2;
                  break;
                }

                return _context4.abrupt("return", this.say("No scene specified"));

              case 2:
                res = (0, _fastFuzzy.search)(sceneName, this.obsProps.scenes, {
                  keySelector: function keySelector(obj) {
                    return obj.name;
                  }
                });
                scene = res.length > 0 ? res[0].name : sceneName;
                _context4.prev = 4;
                _context4.next = 7;
                return this.obs.setCurrentScene({
                  "scene-name": scene
                });

              case 7:
                this.say("Scene successfully switched to \"".concat(scene, "\""));
                _context4.next = 14;
                break;

              case 10:
                _context4.prev = 10;
                _context4.t0 = _context4["catch"](4);
                log.error(_context4.t0);
                this.say(_context4.t0.error);

              case 14:
              case "end":
                return _context4.stop();
            }
          }
        }, _callee4, this, [[4, 10]]);
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
                  _context5.next = 15;
                  break;
                }

                _context5.prev = 1;
                lastScene = this.obsProps.currentScene;

                if (!(lastScene == null)) {
                  _context5.next = 5;
                  break;
                }

                return _context5.abrupt("return", this.say("Error refreshing stream"));

              case 5:
                _context5.next = 7;
                return this.obs.setCurrentScene({
                  "scene-name": _config.default.obs.refreshScene
                });

              case 7:
                this.say("Refreshing stream");
                this.isRefreshing = true;
                setTimeout(function () {
                  _this4.obs.setCurrentScene({
                    "scene-name": lastScene
                  });

                  _this4.say("Refreshing stream completed");

                  _this4.isRefreshing = false;
                }, _config.default.obs.refreshSceneInterval);
                _context5.next = 15;
                break;

              case 12:
                _context5.prev = 12;
                _context5.t0 = _context5["catch"](1);
                log.error(_context5.t0);

              case 15:
              case "end":
                return _context5.stop();
            }
          }
        }, _callee5, this, [[1, 12]]);
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
  }, {
    key: "alias",
    value: function alias(method, _alias, commandName) {
      var _this5 = this;

      var exists = false;

      switch (method) {
        case "add":
          if (!this.commands.includes(commandName)) return this.say("Error ".concat(commandName, " does not exist")); // Check if already exists to replace it

          _config.default.twitchChat.alias.map(function (arr) {
            if (arr[0] == _alias) {
              arr[1] = commandName;
              exists = true;
            }
          });

          this.aliases[_alias] = commandName;
          if (exists) return this.writeAliasToConfig(_alias);

          _config.default.twitchChat.alias.push([_alias, commandName]);

          this.writeAliasToConfig(_alias);
          break;

        case "remove":
          _config.default.twitchChat.alias.map(function (arr, index) {
            if (arr[0] == _alias) {
              _config.default.twitchChat.alias.splice(index);

              delete _this5.aliases[_alias];

              _this5.handleWriteToConfig();

              _this5.say("Removed alias \"".concat(_alias, "\""));

              exists = true;
            }
          });

          if (exists) return;
          this.say("Alias \"".concat(_alias, "\" does not exist"));
          break;

        default:
          break;
      }
    }
  }, {
    key: "writeAliasToConfig",
    value: function writeAliasToConfig(alias) {
      this.handleWriteToConfig();
      this.say("Added alias \"".concat(alias, "\""));
    }
  }, {
    key: "fix",
    value: function () {
      var _fix = _asyncToGenerator(
      /*#__PURE__*/
      regeneratorRuntime.mark(function _callee6() {
        var _this$obsProps$nginxS, ip, application, key, response;

        return regeneratorRuntime.wrap(function _callee6$(_context6) {
          while (1) {
            switch (_context6.prev = _context6.next) {
              case 0:
                this.say("Trying to fix the stream");
                _context6.prev = 1;
                _this$obsProps$nginxS = this.obsProps.nginxSettings, ip = _this$obsProps$nginxS.ip, application = _this$obsProps$nginxS.application, key = _this$obsProps$nginxS.key;
                _context6.next = 5;
                return (0, _nodeFetch.default)("http://".concat(ip, "/control/drop/subscriber?app=").concat(application, "&name=").concat(key));

              case 5:
                response = _context6.sent;

                if (response.ok) {
                  this.say("Successfully fixed the stream");
                }

                _context6.next = 13;
                break;

              case 9:
                _context6.prev = 9;
                _context6.t0 = _context6["catch"](1);
                console.log(_context6.t0);
                this.say("Error fixing the stream");

              case 13:
              case "end":
                return _context6.stop();
            }
          }
        }, _callee6, this, [[1, 9]]);
      }));

      function fix() {
        return _fix.apply(this, arguments);
      }

      return fix;
    }()
  }, {
    key: "registerAliases",
    value: function registerAliases() {
      if (_config.default.twitchChat.alias == null) return;
      var _iteratorNormalCompletion = true;
      var _didIteratorError = false;
      var _iteratorError = undefined;

      try {
        for (var _iterator = _config.default.twitchChat.alias[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true) {
          var alias = _step.value;
          this.aliases[alias[0]] = alias[1];
        }
      } catch (err) {
        _didIteratorError = true;
        _iteratorError = err;
      } finally {
        try {
          if (!_iteratorNormalCompletion && _iterator.return != null) {
            _iterator.return();
          }
        } finally {
          if (_didIteratorError) {
            throw _iteratorError;
          }
        }
      }
    }
  }]);

  return Chat;
}();

var _default = Chat;
exports.default = _default;