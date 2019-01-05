"use strict";

var _ObsSwitcher = _interopRequireDefault(require("./components/ObsSwitcher"));

var _Chat = _interopRequireDefault(require("./components/Chat"));

var _config = _interopRequireDefault(require("../config"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

var obs = new _ObsSwitcher.default(_config.default.ipObs, _config.default.passwordObs, _config.default.lowBitrateScene, _config.default.normalScene, _config.default.offlineScene, _config.default.lowBitrateTrigger);

if (_config.default.enableTwitchChat) {
  var chat = new _Chat.default(_config.default.twitchUsername, _config.default.twitchOauth, "#".concat(_config.default.twitchUsername), obs);
}