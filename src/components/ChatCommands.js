import config from "../../config";

class ChatCommands {
  constructor(_, prefix) {
    this.ws = _.ws;
    this.channel = _.username;
    this.obs = _.obs;
    this.prefix = prefix;
    this.options = ["host", "unhost", "start", "stop", "switch", "raid"];
  }

  host(username) {
    if (username != null) {
      this.ws.send(`PRIVMSG #${this.channel} :/host ${username}`);

      setTimeout(() => {
        this.stop();
      }, config.stopStreamOnHostInterval);
    } else {
      this.ws.send(`PRIVMSG #${this.channel} :Error no username`);
      console.log("Error executing host command no username");
    }
  }

  unhost() {
    this.ws.send(`PRIVMSG #${this.channel} :/unhost`);
  }

  raid(username) {
    if (username != null) {
      this.ws.send(`PRIVMSG #${this.channel} :/raid ${username}`);

      setTimeout(() => {
        this.stop();
      }, config.stopStreamOnRaidInterval);
    } else {
      console.log("Error executing host command no username");
      this.ws.send(`PRIVMSG #${this.channel} :Error no username`);
    }
  }

  async start() {
    // start streaming
    try {
      await this.obs.startStreaming();
      this.ws.send(`PRIVMSG #${this.channel} :Successfully started stream`);
    } catch (e) {
      console.log(e);
      this.ws.send(`PRIVMSG #${this.channel} :Error ${e.error}`);
    }
  }

  async stop() {
    // stop streaming
    try {
      await this.obs.stopStreaming();

      this.ws.send(`PRIVMSG #${this.channel} :Successfully stopped stream`);
    } catch (e) {
      console.log(e.error);
      this.ws.send(`PRIVMSG #${this.channel} :${e.error}`);
    }
  }

  async switch(sceneName) {
    // switch scene
    try {
      await this.obs.setCurrentScene({ "scene-name": sceneName });

      this.ws.send(
        `PRIVMSG #${
          this.channel
        } :Scene successfully switched to "${sceneName}"`
      );
    } catch (e) {
      console.log(e);
    }
  }
}

export default ChatCommands;
