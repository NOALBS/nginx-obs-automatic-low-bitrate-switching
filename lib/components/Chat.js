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

var _stringTemplate = _interopRequireDefault(require("string-template"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

_signale.default.config({
  displayTimestamp: true,
  displayDate: true
});

var log = _signale.default.scope("CHT");

class Chat {
  constructor(username, password, channel, obs) {
    this.username = username.toLowerCase(); // username

    this.password = password; // oauth

    this.channel = "#".concat(channel.toLowerCase()); // #channel

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
    this.language = "en";
    this.open();
    this.registerAliases();
    this.getLanguage();
    this.obsProps.on("live", this.live.bind(this));
    this.obsProps.on("normalScene", this.onNormalScene.bind(this));
    this.obsProps.on("lowBitrateScene", this.onLowBitrateScene.bind(this));
    this.obsProps.on("offlineScene", this.onOfflineScene.bind(this));
  }

  open() {
    log.info("Connecting to twitch");
    this.ws = new _ws.default("wss://irc-ws.chat.twitch.tv:443");
    this.ws.onopen = this.onOpen.bind(this);
    this.ws.onmessage = this.onMessage.bind(this);
    this.ws.onerror = this.onError.bind(this);
    this.ws.onclose = this.onClose.bind(this);
  }

  keepAlive() {
    this.interval = setInterval(() => {
      if (this.sendPing) return;
      this.ws.send("PING :tmi.twitch.tv\r\n");
      this.sendPing = new Date().getTime();
      this.pingTimeout = setTimeout(() => {
        log.error("Didn't receive PONG in time.. reconnecting to twitch.");
        this.close();
        this.sendPing = null;
      }, 1000 * 10);
    }, 1000 * 60 * 2);
  }

  onOpen() {
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

  onClose() {
    log.error("Disconnected from twitch server");
    clearInterval(this.interval);
    this.ws.removeAllListeners();
    this.reconnect();
  }

  close() {
    if (this.ws) {
      this.ws.close();
    }
  }

  reconnect() {
    log.info("Trying to reconnect in 5 seconds");
    setTimeout(() => {
      log.info("Reconnecting...");
      this.open();
    }, 5000);
  }

  onError(e) {
    log.error(new Error(e));
  }

  onMessage(message) {
    if (message !== null) {
      var parsed = this.parse(message.data);

      switch (parsed.command) {
        case "PRIVMSG":
          this.handleMessage(parsed);
          break;

        case "HOSTTARGET":
          if (_config.default.twitchChat.enableAutoStopStreamOnHostOrRaid && !parsed.message.startsWith("-") && this.obsProps.bitrate != null) {
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

  parse(message) {
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
      tagsSplit.map(d => {
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

  handleMessage(msg) {
    if (!msg.message.startsWith(this.prefix)) return;
    var [commandName, ...params] = msg.message.slice(1).split(" ");

    if (commandName in this.aliases) {
      var alias = this.aliases[commandName].split(" ");
      alias.length == 1 ? commandName = alias[0] : [commandName, ...params] = alias;
    }

    switch (true) {
      case commandName == "noalbs":
      case _config.default.twitchChat.adminUsers.includes(msg.username):
      case _config.default.twitchChat.enableModCommands && msg.tags.mod === "1" && this.allowModsCommands.includes(commandName):
      case _config.default.twitchChat.enablePublicCommands && !this.wait && this.allowAllCommands.includes(commandName):
      case msg.username === this.channel.substring(1):
        if (this.rate == 20) return;
        if (!this.commands.includes(commandName)) return;
        this[commandName](...params);
        log.success("".concat(msg.username, " Executed ").concat(commandName, " command"));
        this.setWait();
        break;

      default:
        break;
    }
  }

  setWait() {
    this.rate++;

    if (!this.rateInterval) {
      this.rateInterval = true;
      setTimeout(() => {
        this.rate = 0;
        this.rateInterval = false;
      }, 30000);
    }

    if (!this.wait) {
      this.wait = true;
      setTimeout(() => {
        this.wait = false;
      }, 2000);
    }
  }

  host(username) {
    if (username != null) {
      this.say("/host ".concat(username));
    } else {
      this.say("Error no username"); // console.log("Error executing host command no username");
    }
  }

  unhost() {
    this.say("/unhost");
  }

  raid(username) {
    if (username != null) {
      this.say("/raid ".concat(username));
    } else {
      this.say("Error no username"); // console.log("Error executing host command no username");
    }
  }

  start() {
    var _this = this;

    return _asyncToGenerator(function* () {
      // start streaming
      try {
        yield _this.obs.send("StartStreaming");

        _this.say(_this.locale.start.success);
      } catch (e) {
        log.error(e);

        _this.say((0, _stringTemplate.default)(_this.locale.start.error, {
          error: e.error
        }));
      }
    })();
  }

  stop() {
    var _this2 = this;

    return _asyncToGenerator(function* () {
      // stop streaming
      try {
        yield _this2.obs.send("StopStreaming");

        _this2.say(_this2.locale.stop.success);
      } catch (e) {
        log.error(e.error);

        _this2.say((0, _stringTemplate.default)(_this2.locale.stop.error, {
          error: e.error
        }));
      }
    })();
  }

  rec(bool) {
    if (!bool) {
      this.say("[REC] ".concat(this.obsProps.heartbeat.recording ? this.locale.rec.started : this.locale.rec.stopped));
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
        this.say("[REC] ".concat(this.locale.rec.invalid));
        return;
    }
  }

  startStopRec(bool) {
    var _this3 = this;

    return _asyncToGenerator(function* () {
      if (bool) {
        try {
          var res = yield _this3.obs.send("StartRecording");
          if (res.status === "ok") _this3.say("[REC] ".concat(_this3.locale.rec.started));
          log.success("Started recording");
        } catch (error) {
          _this3.say((0, _stringTemplate.default)("[REC] ".concat(_this3.locale.rec.error), {
            option: _this3.locale.rec.started
          }));
        }
      } else {
        try {
          var _res = yield _this3.obs.send("StopRecording");

          if (_res.status === "ok") _this3.say("[REC] ".concat(_this3.locale.rec.stopped));
          log.success("Stopped recording");
        } catch (error) {
          _this3.say((0, _stringTemplate.default)(" [REC] ".concat(_this3.locale.rec.error), {
            option: _this3.locale.rec.stopped
          }));
        }
      }
    })();
  }

  switch(sceneName) {
    var _this4 = this;

    return _asyncToGenerator(function* () {
      if (sceneName == null) return _this4.say(_this4.locale.switch.error);
      var res = (0, _fastFuzzy.search)(sceneName, _this4.obsProps.scenes, {
        keySelector: obj => obj.name
      });
      var scene = res.length > 0 ? res[0].name : sceneName;

      try {
        yield _this4.obs.send("SetCurrentScene", {
          "scene-name": scene
        });

        _this4.say((0, _stringTemplate.default)(_this4.locale.switch.success, {
          scene
        }));
      } catch (e) {
        log.error(e);

        _this4.say(e.error);
      }
    })();
  }

  bitrate() {
    if (this.obsProps.bitrate != null) {
      if (this.obsProps.rtt != null && this.locale.bitrate.success_rtt != null) {
        this.say((0, _stringTemplate.default)(this.locale.bitrate.success_rtt, {
          bitrate: this.obsProps.bitrate,
          rtt: this.obsProps.rtt
        }));
      } else {
        this.say((0, _stringTemplate.default)(this.locale.bitrate.success, {
          bitrate: this.obsProps.bitrate
        }));
      }
    } else {
      this.say(this.locale.bitrate.error);
    }
  }

  sourceinfo() {
    if (this.obsProps.nginxVideoMeta != null) {
      var {
        height,
        frame_rate
      } = this.obsProps.nginxVideoMeta;
      this.say((0, _stringTemplate.default)(this.locale.sourceinfo.success, {
        height: height[0],
        fps: frame_rate[0],
        bitrate: this.obsProps.bitrate
      }));
    } else {
      this.say(this.locale.sourceinfo.error);
    }
  }

  obsinfo() {
    if (this.obsProps.streamStatus != null) {
      var {
        fps,
        kbitsPerSec
      } = this.obsProps.streamStatus;
      this.say((0, _stringTemplate.default)(this.locale.obsinfo.success, {
        currentScene: this.obsProps.currentScene,
        fps: Math.round(fps),
        bitrate: kbitsPerSec
      }));
    } else {
      this.say(this.locale.obsinfo.error);
    }
  }

  refresh() {
    var _this5 = this;

    return _asyncToGenerator(function* () {
      if (!_this5.isRefreshing) {
        try {
          var lastScene = _this5.obsProps.currentScene;
          if (lastScene == null) return _this5.say(_this5.locale.refresh.error);
          yield _this5.obs.send("SetCurrentScene", {
            "scene-name": _config.default.obs.refreshScene
          });

          _this5.say(_this5.locale.refresh.success);

          _this5.isRefreshing = true;
          setTimeout(() => {
            _this5.obs.send("SetCurrentScene", {
              "scene-name": lastScene
            });

            _this5.say(_this5.locale.refresh.done);

            _this5.isRefreshing = false;
          }, _config.default.obs.refreshSceneInterval);
        } catch (e) {
          log.error(e);
        }
      }
    })();
  }

  live(previous) {
    // this.ws.send(`PRIVMSG ${this.channel} :Scene switching to live`);
    this.say((0, _stringTemplate.default)(this.locale.sceneSwitch.switch, {
      scene: previous
    }));
  }

  onNormalScene() {
    this.say((0, _stringTemplate.default)(this.locale.sceneSwitch.switch, {
      scene: _config.default.obs.normalScene
    }));
    this.bitrate();
  }

  onLowBitrateScene() {
    this.say((0, _stringTemplate.default)(this.locale.sceneSwitch.switch, {
      scene: _config.default.obs.lowBitrateScene
    }));
    this.bitrate();
  }

  onOfflineScene() {
    // this.ws.send(`PRIVMSG ${this.channel} :Stream went offline`);
    this.say((0, _stringTemplate.default)(this.locale.sceneSwitch.switch, {
      scene: _config.default.obs.offlineScene
    }));
  }

  trigger(number) {
    if (number) {
      if (!isNaN(number)) {
        this.obsProps.lowBitrateTrigger = +number;
        _config.default.obs.lowBitrateTrigger = +number;
        this.handleWriteToConfig();
        this.say((0, _stringTemplate.default)(this.locale.trigger.success, {
          number: this.obsProps.lowBitrateTrigger
        }));
      } else {
        this.say((0, _stringTemplate.default)(this.locale.trigger.error, {
          number: number
        }));
      }

      return;
    }

    this.say((0, _stringTemplate.default)(this.locale.trigger.current, {
      number: this.obsProps.lowBitrateTrigger
    }));
  }

  public(bool) {
    this.handleEnable("enablePublicCommands", bool, this.locale.handleCommands.public);
  }

  mod(bool) {
    this.handleEnable("enableModCommands", bool, this.locale.handleCommands.mod);
  }

  notify(bool) {
    this.handleEnable("enableAutoSwitchNotification", bool, this.locale.handleCommands.notify);
  }

  autostop(bool) {
    this.handleEnable("enableAutoStopStreamOnHostOrRaid", bool, this.locale.handleCommands.autostop);
  }

  handleEnable(name, bool, response) {
    if (!bool) {
      this.say("".concat(response, " ").concat(_config.default.twitchChat[name] ? this.locale.handleCommands.enabled : this.locale.handleCommands.disabled));
      return;
    }

    if (bool === "on" && _config.default.twitchChat[name] != true) {
      _config.default.twitchChat[name] = true;
      this.handleWriteToConfig();
      this.say("".concat(response, " ").concat(this.locale.handleCommands.enabled));
    } else if (bool === "off" && _config.default.twitchChat[name] != false) {
      _config.default.twitchChat[name] = false;
      this.handleWriteToConfig();
      this.say("".concat(response, " ").concat(this.locale.handleCommands.disabled));
    } else {
      this.say("".concat(response, " ").concat(this.locale.handleCommands.already, " ").concat(_config.default.twitchChat[name] ? this.locale.handleCommands.enabled : this.locale.handleCommands.disabled));
    }
  }

  handleWriteToConfig() {
    _fs.default.writeFile("".concat(__dirname, "/../../config.json"), JSON.stringify(_config.default, null, 4), err => {
      if (err) log.error(err);
    });
  }

  say(message) {
    this.ws.send("PRIVMSG ".concat(this.channel, " :").concat(message));
  }

  noalbs(a) {
    if (a === "version") this.say("Running NOALBS v".concat(process.env.npm_package_version));
  }

  alias(method, alias, commandName) {
    var exists = false;

    switch (method) {
      case "add":
        if (!this.commands.includes(commandName)) return this.say((0, _stringTemplate.default)(this.locale.alias.error, {
          command: commandName
        })); // Check if already exists to replace it

        _config.default.twitchChat.alias.map(arr => {
          if (arr[0] == alias) {
            arr[1] = commandName;
            exists = true;
          }
        });

        this.aliases[alias] = commandName;
        if (exists) return this.writeAliasToConfig(alias);

        _config.default.twitchChat.alias.push([alias, commandName]);

        this.writeAliasToConfig(alias);
        break;

      case "remove":
        _config.default.twitchChat.alias.map((arr, index) => {
          if (arr[0] == alias) {
            _config.default.twitchChat.alias.splice(index);

            delete this.aliases[alias];
            this.handleWriteToConfig();
            this.say((0, _stringTemplate.default)(this.locale.alias.removed, {
              alias: alias
            }));
            exists = true;
          }
        });

        if (exists) return;
        this.say((0, _stringTemplate.default)(this.locale.alias.error, {
          command: alias
        }));
        break;

      default:
        break;
    }
  }

  writeAliasToConfig(alias) {
    this.handleWriteToConfig();
    this.say((0, _stringTemplate.default)(this.locale.alias.success, {
      alias: alias
    }));
  }

  fix() {
    var _this6 = this;

    return _asyncToGenerator(function* () {
      _this6.say(_this6.locale.fix.try);

      var {
        availableRequests
      } = yield _this6.obs.send("GetVersion");

      if (availableRequests.includes("RestartMedia")) {
        var s = yield _this6.obs.send("GetMediaSourcesList");
        s.mediaSources.filter(e => e.mediaState == "playing").forEach( /*#__PURE__*/function () {
          var _ref = _asyncToGenerator(function* (e) {
            var _sourceSettings$input;

            var {
              sourceSettings
            } = yield _this6.obs.send("GetSourceSettings", {
              sourceName: e.sourceName
            });
            var input = (_sourceSettings$input = sourceSettings.input) === null || _sourceSettings$input === void 0 ? void 0 : _sourceSettings$input.toLowerCase();

            if (input !== null && input !== void 0 && input.startsWith("rtmp") || input !== null && input !== void 0 && input.startWith("srt")) {
              yield _this6.obs.send("RestartMedia", {
                sourceName: e.sourceName
              });
            }
          });

          return function (_x) {
            return _ref.apply(this, arguments);
          };
        }());
      } else {
        var {
          server,
          stats,
          application,
          key
        } = _config.default.rtmp;
        var site = /(\w+:\/\/[^\/]+)/g.exec(stats)[1];

        switch (server) {
          case "nginx":
            try {
              var response = yield (0, _nodeFetch.default)("".concat(site, "/control/drop/subscriber?app=").concat(application, "&name=").concat(key));

              if (response.ok) {
                _this6.say(_this6.locale.fix.success);
              }
            } catch (e) {
              _this6.say(_this6.locale.fix.error);
            }

            break;

          default:
            _this6.say(_this6.locale.fix.error);

            break;
        }
      }
    })();
  }

  registerAliases() {
    if (_config.default.twitchChat.alias == null) return;

    for (var alias of _config.default.twitchChat.alias) {
      this.aliases[alias[0]] = alias[1];
    }
  }

  getLanguage() {
    if (_config.default.language != null) this.language = _config.default.language;

    _fs.default.readFile("".concat(__dirname, "/../../locales/").concat(this.language, ".json"), "utf8", (err, data) => {
      if (err) {
        log.error("Error loading language \"".concat(this.language, "\""));
        process.exit();
      }

      this.locale = JSON.parse(data);
    });
  }

}

var _default = Chat;
exports.default = _default;