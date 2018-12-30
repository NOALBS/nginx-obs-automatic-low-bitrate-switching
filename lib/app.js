"use strict";

var _ObsSwitcher = _interopRequireDefault(require("./components/ObsSwitcher"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

var OBS = new _ObsSwitcher.default();
OBS.connect();