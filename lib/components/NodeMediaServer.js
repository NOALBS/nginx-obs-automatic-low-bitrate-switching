"use strict";

var _nodeMediaServer = _interopRequireDefault(require("node-media-server"));

var _signale = _interopRequireDefault(require("signale"));

var _config = _interopRequireDefault(require("../../config"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

var NodeMediaServerManager = function NodeMediaServerManager(nmsConfig, obsSwitcher) {
  _classCallCheck(this, NodeMediaServerManager);

  if (!nmsConfig) {
    return;
  }

  try {
    console = _signale.default.scope("NMS");
  } catch (e) {
    console.log = _signale.default.scope("NMS").log;

    _signale.default.scope("NMS").warn("Couldn't set console for node-media-server. Consider upgrading to node 10 or higher to resovle this problem (error: ".concat(e, ")"));
  }

  this.obsSwitcher = obsSwitcher;
  this.nodeMediaServer = new _nodeMediaServer.default(nmsConfig);
  this.nodeMediaServer.run();

  if (_config.default.rtmp && !_config.default.rtmp.stats) {
    _config.default.rtmp.server = "node-media-server";
    var auth = "";

    if (nmsConfig.auth && nmsConfig.auth.api) {
      auth = "".concat(nmsConfig.auth.api_user, ":").concat(nmsConfig.auth.api_pass, "@");
    }

    if (nmsConfig.http) {
      _config.default.rtmp.stats = "http://".concat(auth, "localhost:").concat(nmsConfig.http.port, "/api/streams");
    }

    if (nmsConfig.https) {
      _config.default.rtmp.stats = "https://".concat(auth, "localhost:").concat(nmsConfig.https.port, "/api/streams");
    }
  }

  this.nodeMediaServer.on('postPublish', function (id, StreamPath, args) {
    obsSwitcher.switchSceneIfNecessary();
  });
  this.nodeMediaServer.on('donePublish', function (id, StreamPath, args) {
    obsSwitcher.switchSceneIfNecessary();
  });
};

module.exports = NodeMediaServerManager;