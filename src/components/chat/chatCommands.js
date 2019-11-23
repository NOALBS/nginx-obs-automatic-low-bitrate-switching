// export function host(username) {
//     if (username != null) {
//         this.say(`/host ${username}`);
//     } else {
//         this.say(`Error no username`);
//         // console.log("Error executing host command no username");
//     }
// }

// export function unhost() {
//     this.say(`/unhost`);
// }

// export function raid(username) {
//     if (username != null) {
//         this.say(`/raid ${username}`);
//     } else {
//         this.say(`Error no username`);
//         // console.log("Error executing host command no username");
//     }
// }

// export async function start() {
//     // start streaming
//     try {
//         await this.obs.startStreaming();
//         this.say(this.locale.start.success);
//     } catch (e) {
//         log.error(e);
//         this.say(
//             format(this.locale.start.error, {
//                 error: e.error
//             })
//         );
//     }
// }

// export async function stop() {
//     // stop streaming
//     try {
//         await this.obs.stopStreaming();
//         this.say(this.locale.stop.success);
//     } catch (e) {
//         log.error(e.error);
//         this.say(
//             format(this.locale.stop.error, {
//                 error: e.error
//             })
//         );
//     }
// }
// export function rec(bool) {
//     if (!bool) {
//         this.say(`[REC] ${this.obsProps.heartbeat.recording ? this.locale.rec.started : this.locale.rec.stopped}`);
//         return;
//     }

//     switch (bool) {
//         case "on":
//             this.startStopRec(true);
//             return;
//         case "off":
//             this.startStopRec(false);
//             return;
//         default:
//             this.say(`[REC] ${this.locale.rec.invalid}`);
//             return;
//     }
// }

// export async function startStopRec(bool) {
//     if (bool) {
//         try {
//             const res = await this.obs.StartRecording();
//             if (res.status === "ok") this.say(`[REC] ${this.locale.rec.started}`);
//             log.success(`Started recording`);
//         } catch (error) {
//             this.say(
//                 format(`[REC] ${this.locale.rec.error}`, {
//                     option: this.locale.rec.started
//                 })
//             );
//         }
//     } else {
//         try {
//             const res = await this.obs.StopRecording();
//             if (res.status === "ok") this.say(`[REC] ${this.locale.rec.stopped}`);
//             log.success(`Stopped recording`);
//         } catch (error) {
//             this.say(
//                 format(` [REC] ${this.locale.rec.error}`, {
//                     option: this.locale.rec.stopped
//                 })
//             );
//         }
//     }
// }

export function Switch(channel, sceneName) {
    // console.log(channel, sceneName);
    // switch will send request to ("obs:request", "switch:${channel}", "switch", channel, sceneName); (channel and sceneName)
    // OBS will return the data needed to make the default language string, (so like sceneName and which type like if the request succeeded)
    // this will then be returned to messageHandler.

    return {
        type: "success",
        data: {
            scene: sceneName
        }
    };
}

// export function bitrate() {
//     if (this.obsProps.bitrate != null) {
//         this.say(
//             format(this.locale.bitrate.success, {
//                 bitrate: this.obsProps.bitrate
//             })
//         );
//     } else {
//         this.say(this.locale.bitrate.error);
//     }
// }
// export function sourceinfo() {
//     if (this.obsProps.nginxVideoMeta != null) {
//         const { height, frame_rate } = this.obsProps.nginxVideoMeta;

//         this.say(
//             format(this.locale.sourceinfo.success, {
//                 height: height[0],
//                 fps: frame_rate[0],
//                 bitrate: this.obsProps.bitrate
//             })
//         );
//     } else {
//         this.say(this.locale.sourceinfo.error);
//     }
// }

// export function obsinfo() {
//     if (this.obsProps.streamStatus != null) {
//         const { fps, kbitsPerSec } = this.obsProps.streamStatus;

//         this.say(
//             format(this.locale.obsinfo.success, {
//                 currentScene: this.obsProps.currentScene,
//                 fps: Math.round(fps),
//                 bitrate: kbitsPerSec
//             })
//         );
//     } else {
//         this.say(this.locale.obsinfo.error);
//     }
// }

// export async function refresh() {
//     if (!this.isRefreshing) {
//         try {
//             const lastScene = this.obsProps.currentScene;

//             if (lastScene == null) return this.say(this.locale.refresh.error);

//             await this.obs.setCurrentScene({
//                 "scene-name": config.obs.refreshScene
//             });
//             this.say(this.locale.refresh.success);
//             this.isRefreshing = true;

//             setTimeout(() => {
//                 this.obs.setCurrentScene({
//                     "scene-name": lastScene
//                 });
//                 this.say(this.locale.refresh.done);
//                 this.isRefreshing = false;
//             }, config.obs.refreshSceneInterval);
//         } catch (e) {
//             log.error(e);
//         }
//     }
// }

// export function live(previous) {
//     // this.ws.send(`PRIVMSG ${this.channel} :Scene switching to live`);
//     this.say(
//         format(this.locale.sceneSwitch.switch, {
//             scene: previous
//         })
//     );
// }

// export function onNormalScene() {
//     this.say(
//         format(this.locale.sceneSwitch.switch, {
//             scene: config.obs.normalScene
//         })
//     );
//     this.bitrate();
// }

// export function onLowBitrateScene() {
//     this.say(
//         format(this.locale.sceneSwitch.switch, {
//             scene: config.obs.lowBitrateScene
//         })
//     );
//     this.bitrate();
// }

// export function onOfflineScene() {
//     // this.ws.send(`PRIVMSG ${this.channel} :Stream went offline`);
//     this.say(
//         format(this.locale.sceneSwitch.switch, {
//             scene: config.obs.offlineScene
//         })
//     );
// }

// export function trigger(number) {
//     if (number) {
//         if (!isNaN(number)) {
//             this.obsProps.lowBitrateTrigger = +number;
//             config.obs.lowBitrateTrigger = +number;

//             this.handleWriteToConfig();
//             this.say(
//                 format(this.locale.trigger.success, {
//                     number: this.obsProps.lowBitrateTrigger
//                 })
//             );
//         } else {
//             this.say(
//                 format(this.locale.trigger.error, {
//                     number: number
//                 })
//             );
//         }

//         return;
//     }

//     this.say(
//         format(this.locale.trigger.current, {
//             number: this.obsProps.lowBitrateTrigger
//         })
//     );
// }

// export function publicCommands(bool) {
//     this.handleEnable("enablePublicCommands", bool, this.locale.handleCommands.public);
// }

// export function modCommands(bool) {
//     this.handleEnable("enableModCommands", bool, this.locale.handleCommands.mod);
// }

// export function notify(bool) {
//     this.handleEnable("enableAutoSwitchNotification", bool, this.locale.handleCommands.notify);
// }

// export function autostop(bool) {
//     this.handleEnable("enableAutoStopStreamOnHostOrRaid", bool, this.locale.handleCommands.autostop);
// }

// export function handleEnable(name, bool, response) {
//     if (!bool) {
//         this.say(`${response} is ${config.twitchChat[name] ? this.locale.handleCommands.enabled : this.locale.handleCommands.disabled}`);
//         return;
//     }

//     if (bool === "on" && config.twitchChat[name] != true) {
//         config.twitchChat[name] = true;
//         this.handleWriteToConfig();
//         this.say(`${response} ${this.locale.handleCommands.enabled}`);
//     } else if (bool === "off" && config.twitchChat[name] != false) {
//         config.twitchChat[name] = false;
//         this.handleWriteToConfig();
//         this.say(`${response} ${this.locale.handleCommands.disabled}`);
//     } else {
//         this.say(
//             `${response} ${this.locale.handleCommands.already} ${
//                 config.twitchChat[name] ? this.locale.handleCommands.enabled : this.locale.handleCommands.disabled
//             }`
//         );
//     }
// }

// export function handleWriteToConfig() {
//     fs.writeFile('"../../config.json', JSON.stringify(config, null, 4), err => {
//         if (err) log.error(err);
//     });
// }

export function noalbs(_, a) {
    if (a === "version")
        return {
            type: "send",
            data: `Running NOALBS v${process.env.npm_package_version}`
        };
}

// export function alias(method, alias, commandName) {
//     let exists = false;

//     switch (method) {
//         case "add":
//             if (!this.commands.includes(commandName))
//                 return this.say(
//                     format(this.locale.alias.error, {
//                         command: commandName
//                     })
//                 );

//             // Check if already exists to replace it
//             config.twitchChat.alias.map(arr => {
//                 if (arr[0] == alias) {
//                     arr[1] = commandName;
//                     exists = true;
//                 }
//             });

//             this.aliases[alias] = commandName;
//             if (exists) return this.writeAliasToConfig(alias);

//             config.twitchChat.alias.push([alias, commandName]);
//             this.writeAliasToConfig(alias);
//             break;
//         case "remove":
//             config.twitchChat.alias.map((arr, index) => {
//                 if (arr[0] == alias) {
//                     config.twitchChat.alias.splice(index);
//                     delete this.aliases[alias];
//                     this.handleWriteToConfig();
//                     this.say(
//                         format(this.locale.alias.removed, {
//                             alias: alias
//                         })
//                     );
//                     exists = true;
//                 }
//             });

//             if (exists) return;

//             this.say(
//                 format(this.locale.alias.error, {
//                     command: alias
//                 })
//             );
//             break;
//         default:
//             break;
//     }
// }

// export function writeAliasToConfig(alias) {
//     this.handleWriteToConfig();
//     this.say(
//         format(this.locale.alias.success, {
//             alias: alias
//         })
//     );
// }

// export async function fix() {
//     this.say(this.locale.fix.try);
//     const { server, stats, application, key } = config.rtmp;
//     const site = /(\w+:\/\/[^\/]+)/g.exec(stats)[1];

//     switch (server) {
//         case "nginx":
//             try {
//                 const response = await fetch(`${site}/control/drop/subscriber?app=${application}&name=${key}`);

//                 if (response.ok) {
//                     this.say(this.locale.fix.success);
//                 }
//             } catch (e) {
//                 console.log(e);
//                 this.say(this.locale.fix.error);
//             }
//             break;
//         default:
//             this.say(this.locale.fix.error);
//             break;
//     }
// }

// export function registerAliases() {
//     if (config.twitchChat.alias == null) return;

//     for (const alias of config.twitchChat.alias) {
//         this.aliases[alias[0]] = alias[1];
//     }
// }

// export function getLanguage() {
//     if (config.language != null) this.language = config.language;

//     fs.readFile(`${__dirname}/../../locales/${this.language}.json`, "utf8", (err, data) => {
//         if (err) {
//             log.error(`Error loading language "${this.language}"`);
//             process.exit();
//         }

//         this.locale = JSON.parse(data);
//     });
// }
