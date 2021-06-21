import OBSWebSocket from "obs-websocket-js";
import fetch from "node-fetch";
import xml2js from "xml2js";
import config from "../../config";
import EventEmitter from "events";
import signale from "signale";

signale.config({
    displayTimestamp: true,
    displayDate: true,
});

const log = signale.scope("OBS");
const parseString = xml2js.parseString;

class ObsSwitcher extends EventEmitter {
    constructor(
        address,
        password,
        low,
        normal,
        offline,
        lowBitrateTrigger,
        highRttTrigger = 2500
    ) {
        super();

        this.obs = new OBSWebSocket();
        this.isLive = false;
        this.address = address;
        this.password = password;
        this.lowBitrateScene = low;
        this.normalScene = normal;
        this.offlineScene = offline;
        this.lowBitrateTrigger = lowBitrateTrigger;
        this.highRttTrigger = highRttTrigger;
        this.bitrate = null;
        this.nginxVideoMeta = null;
        this.streamStatus = null;
        this.heartbeat = null;
        this.obsStreaming = false;
        this.currentScene = null;
        this.nginxSettings;
        this.previousScene = this.lowBitrateScene;
        this.scenes = null;

        this.obs
            .connect({ address: this.address, password: this.password })
            .catch((e) => {
                // handle this somewhere else
            });

        this.obs.on("ConnectionClosed", this.onDisconnect.bind(this));
        this.obs.on("AuthenticationSuccess", this.onAuth.bind(this));
        this.obs.on("AuthenticationFailure", this.onAuthFail.bind(this));
        this.obs.on("error", this.error.bind(this));
        this.obs.on("StreamStatus", this.setStreamStatus.bind(this));
        this.obs.on("StreamStopped", this.streamStopped.bind(this));
        this.obs.on("StreamStarted", this.streamStarted.bind(this));
        this.obs.on("Heartbeat", this.handleHeartbeat.bind(this));
        this.obs.on("ScenesChanged", this.scenesChanged.bind(this));

        log.info("Connecting & authenticating");
    }

    async switchSceneIfNecessary() {
        if (
            !this.obsStreaming &&
            (config.obs.onlySwitchWhenStreaming == null ||
                config.obs.onlySwitchWhenStreaming)
        )
            return;

        const [bitrate, rtt] = await this.getBitrate();
        const { currentScene, canSwitch } = await this.canSwitch();

        if (bitrate !== null) {
            this.isLive = true;

            if (["nimble", "srt-live-server"].includes(config.rtmp.server)) {
                this.isLive &&
                    canSwitch &&
                    (bitrate === 0 &&
                        currentScene.name !== this.previousScene &&
                        (this.obs.send("SetCurrentScene", {
                            "scene-name": this.previousScene,
                        }),
                        this.switchSceneEmit("live", this.previousScene),
                        log.info(
                            `Stream went online switching to scene: "${this.previousScene}"`
                        )),
                    (rtt < this.highRttTrigger || rtt >= this.highRttTrigger) &&
                        bitrate <= this.lowBitrateTrigger &&
                        currentScene.name !== this.lowBitrateScene &&
                        bitrate !== 0 &&
                        (this.obs.send("SetCurrentScene", {
                            "scene-name": this.lowBitrateScene,
                        }),
                        (this.previousScene = this.lowBitrateScene),
                        this.switchSceneEmit("lowBitrateScene"),
                        log.info(
                            `Low bitrate detected switching to scene: "${this.lowBitrateScene}"`
                        )),
                    rtt >= this.highRttTrigger &&
                        bitrate > this.lowBitrateTrigger &&
                        currentScene.name !== this.lowBitrateScene &&
                        bitrate !== 0 &&
                        (this.obs.send("SetCurrentScene", {
                            "scene-name": this.lowBitrateScene,
                        }),
                        (this.previousScene = this.lowBitrateScene),
                        this.switchSceneEmit("lowBitrateScene"),
                        log.info(
                            `High RTT detected switching to scene: "${this.lowBitrateScene}"`
                        )),
                    rtt < this.highRttTrigger &&
                        bitrate > this.lowBitrateTrigger &&
                        currentScene.name !== this.normalScene &&
                        (this.obs.send("SetCurrentScene", {
                            "scene-name": this.normalScene,
                        }),
                        (this.previousScene = this.normalScene),
                        this.switchSceneEmit("normalScene"),
                        log.info(
                            `Switching to normal scene: "${this.normalScene}"`
                        )));
            } else {
                this.isLive &&
                    canSwitch &&
                    (bitrate === 0 &&
                        currentScene.name !== this.previousScene &&
                        (this.obs.send("SetCurrentScene", {
                            "scene-name": this.previousScene,
                        }),
                        this.switchSceneEmit("live", this.previousScene),
                        log.info(
                            `Stream went online switching to scene: "${this.previousScene}"`
                        )),
                    bitrate <= this.lowBitrateTrigger &&
                        currentScene.name !== this.lowBitrateScene &&
                        bitrate !== 0 &&
                        (this.obs.send("SetCurrentScene", {
                            "scene-name": this.lowBitrateScene,
                        }),
                        (this.previousScene = this.lowBitrateScene),
                        this.switchSceneEmit("lowBitrateScene"),
                        log.info(
                            `Low bitrate detected switching to scene: "${this.lowBitrateScene}"`
                        )),
                    bitrate > this.lowBitrateTrigger &&
                        currentScene.name !== this.normalScene &&
                        (this.obs.send("SetCurrentScene", {
                            "scene-name": this.normalScene,
                        }),
                        (this.previousScene = this.normalScene),
                        this.switchSceneEmit("normalScene"),
                        log.info(
                            `Switching to normal scene: "${this.normalScene}"`
                        )));
            }
        } else {
            this.isLive = false;

            canSwitch &&
                currentScene.name !== this.offlineScene &&
                (this.obs.send("SetCurrentScene", {
                    "scene-name": this.offlineScene
                }),
                this.switchSceneEmit("offlineScene"),
                (this.streamStatus = null),
                log.warn(
                    `Error receiving current bitrate or stream is offline. Switching to offline scene: "${this.offlineScene}"`
                ));
        }
    }

    onAuth() {
        log.success(`Successfully connected`);
        this.obs.send("SetHeartbeat", { enable: true });
        this.getSceneList();

        this.interval = setInterval(
            this.switchSceneIfNecessary.bind(this),
            config.obs.requestMs
        );
    }

    switchSceneEmit(sceneName, args) {
        if (
            config.twitchChat.enableAutoSwitchNotification &&
            this.obsStreaming
        ) {
            this.emit(sceneName, args);
        }
    }

    async getBitrate() {
        const { server, stats, application, key, id, publisher } = config.rtmp;

        switch (server) {
            case "nginx":
                try {
                    const response = await fetch(stats);
                    const data = await response.text();

                    parseString(data, (err, result) => {
                        const publish = result.rtmp.server[0].application.find(
                            (stream) => {
                                return stream.name[0] === application;
                            }
                        ).live[0].stream;

						const stream = publish?.find((stream) => {
							return stream.name[0] === key;
						});

						if (stream != null) {
                            this.nginxVideoMeta = stream.meta[0].video[0];
                            this.bitrate = Math.round(
								stream.bw_video[0] / 1024
							);
						} else {
							this.nginxVideoMeta = null;
							this.bitrate = null;
						}
                    });
                } catch (e) {
                    log.error("[NGINX] Error fetching stats");
                }
                break;

            case "node-media-server":
                try {
                    const response = await fetch(
                        `${stats}/${application}/${key}`
                    );
                    const data = await response.json();

                    this.bitrate = data.bitrate || null;
                } catch (e) {
                    log.error(
                        "[NMS] Error fetching stats, is the API http server running?"
                    );
                }
                break;

            case "nimble":
                try {
                    // SRT stats to see RTT and if streaming is active
                    const srtresponse = await fetch(
                        stats + "/manage/srt_receiver_stats"
                    );
                    const srtdata = await srtresponse.json();
                    const srtreceiver = srtdata.SrtReceivers.filter(
                        (receiver) => receiver.id.includes(id)
                    );
                    const publish = srtreceiver[0].state;

                    if (publish == "disconnected") {
                        this.bitrate = null;
                        this.rtt = null;
                    } else {
                        // RTMP status for bitrate. srt_receiver_stats seems to give an averaged number that isn't as useful.
                        // Probably requires nimble to be configured to make the video from SRT available on RTMP even though it's not used anywhere
                        const rtmpresponse = await fetch(
                            stats + "/manage/rtmp_status"
                        );
                        const rtmpdata = await rtmpresponse.json();
                        const rtmpstream = rtmpdata
                            .filter((rtmp) => rtmp.app == application)[0]
                            .streams.filter((stream) => stream.strm == key);
                        this.bitrate = Math.round(
                            rtmpstream[0].bandwidth / 1024
                        );
                        this.rtt = srtreceiver[0].stats.link.rtt;
                    }
                } catch (e) {
                    log.error("[NIMBLE] Error fetching stats: " + e);
                }
                break;

            case "srt-live-server":
                try {
                    const srtresponse = await fetch(stats);
                    const srtdata = await srtresponse.json();
                    const stream = srtdata.publishers[publisher];

                    this.bitrate = stream?.bitrate || null;
                    this.rtt = stream?.rtt ?? null;
                } catch (e) {
                    log.error("[SLS] Error fetching stats: " + e);
                }
                break;

            default:
                log.error(
                    "[STATS] Something went wrong at getting the RTMP server, did you enter the correct name in the config?"
                );
                break;
        }

        return [this.bitrate, this.rtt];
    }

    setStreamStatus(res) {
        this.streamStatus = res;
    }

    error(e) {
        log.error(e);
    }

    onDisconnect() {
        log.error("Can't connect or lost connnection");
        clearInterval(this.interval);

        this.reconnect();
    }

    onAuthFail() {
        log.error("Failed to authenticate");
    }

    reconnect() {
        log.info("Trying to reconnect in 5 seconds");
        setTimeout(() => {
            this.obs
                .connect({ address: this.address, password: this.password })
                .catch((e) => {
                    // handle this somewhere else
                });
        }, 5000);
    }

    async streamStopped() {
        this.obsStreaming = false;
        this.nginxVideoMeta = null;
        this.bitrate = null;

        const { canSwitch } = await this.canSwitch();

        if (canSwitch) {
            this.obs.send("SetCurrentScene", {
                "scene-name": this.offlineScene,
            });
        }
    }

    streamStarted() {
        this.obsStreaming = true;
    }

    async getSceneList() {
        const list = await this.obs.send("GetSceneList");
        this.scenes = list.scenes;
    }

    scenesChanged() {
        this.getSceneList();
    }

    handleHeartbeat(heartbeat) {
        this.heartbeat = heartbeat;
        this.obsStreaming = heartbeat.streaming;
    }

    async canSwitch() {
        const currentScene = await this.obs.send("GetCurrentScene");
        const canSwitch =
            currentScene.name == this.lowBitrateScene ||
            currentScene.name == this.normalScene ||
            currentScene.name == this.offlineScene;

        this.currentScene = currentScene.name;

        return { currentScene, canSwitch };
    }
}

export default ObsSwitcher;
