"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _ws = _interopRequireDefault(require("ws"));

var _ChatCommands = _interopRequireDefault(require("./ChatCommands"));

var _config = _interopRequireDefault(require("../../config"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

var Chat =
/*#__PURE__*/
function () {
  function Chat(username, password, channel, OBS) {
    _classCallCheck(this, Chat);

    this.username = username; // username

    this.password = password; // oauth

    this.channel = channel; // #channel

    this.ws = new _ws.default("wss://irc-ws.chat.twitch.tv:443");
    this.obs = OBS;
    this.ws.onopen = this.onOpen.bind(this);
    this.ws.onmessage = this.onMessage.bind(this);
    this.ws.onerror = this.onError.bind(this);
    this.ws.onclose = this.onClose.bind(this);
  }

  _createClass(Chat, [{
    key: "keepAlive",
    value: function keepAlive() {
      var _this = this;

      setInterval(function () {
        _this.ws.send("PING :tmi.twitch.tv\r\n");
      }, 2000);
    }
  }, {
    key: "onOpen",
    value: function onOpen() {
      if (this.ws !== null && this.ws.readyState === 1) {
        console.log("Successfully Connected to websocket");
        console.log("Authenticating and joining channel ".concat(this.channel));
        this.ws.send("CAP REQ :twitch.tv/tags");
        this.ws.send("PASS ".concat(this.password));
        this.ws.send("NICK ".concat(this.username));
        this.ws.send("JOIN ".concat(this.channel));
        this.keepAlive();
        this.commands = new _ChatCommands.default(this, "!");
      }
    }
  }, {
    key: "onClose",
    value: function onClose() {
      console.log("Disconnected from the chat server.");
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
          if (parsed.command === "PRIVMSG" && _config.default.allowedUsers.includes(parsed.username)) {
            // not a command
            if (parsed.message.substr(0, 1) !== this.commands.prefix) return; // Split the message into individual words:

            var parse = parsed.message.slice(1).split(" ");
            var commandName = parse[0];

            if (this.commands.options.includes(commandName)) {
              this.commands[commandName](parse[1]);
              console.log("! Executed ".concat(commandName, " command"));
            } else {
              console.log("! Unknown command ".concat(commandName));
            }
          } else if (parsed.command === "PING") {
            this.ws.send("PONG :".concat(parsed.message));
          }
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
      }

      return parsedMessage;
    }
  }]);

  return Chat;
}();

var _default = Chat;
exports.default = _default;