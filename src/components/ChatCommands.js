class ChatCommands {
  constructor(_, prefix) {
    this.ws = _.ws;
    this.channel = _.username;
    this.obs = _.obs;
    this.prefix = prefix;
    this.options = ["host", "unhost", "start", "stop", "switch"];
  }

  host(username) {
    if (username != null) {
      this.ws.send(`PRIVMSG #${this.channel} :/host ${username}`);

      setTimeout(() => {
        this.stop();
      }, 5000);
    } else {
      console.log("Error executing host command no username");
    }
  }

  unhost() {
    this.ws.send(`PRIVMSG #${this.channel} :/unhost`);
  }

  async start() {
    // start streaming
    try {
      await this.obs.startStreaming();
    } catch (e) {
      console.log(e);
    }
  }

  async stop() {
    // stop streaming
    try {
      await this.obs.stopStreaming();
    } catch (e) {
      console.log(e);
    }
  }

  async switch(sceneName) {
    // switch scene
    try {
      await this.obs.setCurrentScene({ "scene-name": sceneName });
    } catch (e) {
      console.log(e);
    }
  }
}

export default ChatCommands;
