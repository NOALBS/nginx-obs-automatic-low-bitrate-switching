```
    ███╗   ██╗ ██████╗  █████╗ ██╗     ██████╗ ███████╗
    ████╗  ██║██╔═══██╗██╔══██╗██║     ██╔══██╗██╔════╝
    ██╔██╗ ██║██║   ██║███████║██║     ██████╔╝███████╗
    ██║╚██╗██║██║   ██║██╔══██║██║     ██╔══██╗╚════██║
    ██║ ╚████║╚██████╔╝██║  ██║███████╗██████╔╝███████║
    ╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝ ╚══════╝
```

# nginx-obs-automatic-low-bitrate-switching
Simple app to automatically switch scenes in OBS Studio/OBS.Live based on the current bitrate fetched from the server stats.

---
# !!! PLEASE BE ADVISED !!!


NOALBS is used as a DIY tool to have your OBS Studio/OBS.Live auto switch scenes when you are either in a LOW bitrate situation or if your source disconnects completely.

## Upsides to using NOALBS:
- You did it yourself so be proud of what you have accomplished.
- You're using a computer, internet and power you are already paying for.
- There is literally NO COST to you when you setup and use NOALBS using the equipment you already own and pay for.

## Downsides to using NOALBS:
- It's DIY, meaning you have to do it all yourself, of course you can always contact us on discord @ b3ck#3517 or 715209#0600 and we'll do our best to help out.
- If you lose power or internet where you run NOALBS there is no redundancy.
- It takes a bit to setup, it is not for the faint of heart when it comes to installing and configuring advanced programs to get it running.

(Watch video below for help)

If you're okay with all of that then all I can say is if you can't figure it out, reach out to us or the community if you need help, we're here for you.

---
## How to video (WINDOWS):
[![YouTube: How to Download, Install and Run NOALBS for IRL Livestreaming (WINDOWS)](https://i.imgur.com/98sptuM.png)](https://www.youtube.com/watch?v=rglDAIm73cM)

---
## Similar Solutions / Paid Services:
Don't feel like setting this all up by yourself? Check out these links for similar solutions/paid services:

-   [IRLToolkit](https://irltoolkit.com/)
-   [psynapticmedia.com](http://www.psynapticmedia.com/super-stream-system-by-psynaps/)
-   [norip.io](https://www.norip.io)
-   Do you offer a similar solution or paid service? Want your link here? Message me on Discord [b3ck#3517](https://discordapp.com/channels/@me/96991451006660608)

---
## Table of Contents:
- [Build Prerequisities](#build-prerequisities)
- [Installation from Source](#installation-from-source-and-nginx-setup)
- [Configuring the CONFIG and Setting up Roles](#configuring-the-config-and-setting-up-roles)
- [How to run from source](#how-to-run-from-source)
- [Chat Commands](#chat-commands)
- [How to pull RTMP stream into OBS](#how-to-pull-rtmp-stream-into-obs)
- [Running with other servers (not NGINX)](#running-with-other-servers-not-nginx)
  - [Using the built-in Node-Media-Server](#using-the-built-in-node-media-server)
  - [Using an external Node-Media-Server](#using-an-external-node-media-server)
  - [Using Nimble Streamer Server (with SRT protocol)](#using-nimble-streamer-server-with-srt-protocol)
  - [Using SLS (SRT-LIVE-SERVER)](#using-sls-srt-live-server)
- [Help it won't change scenes](#help-it-wont-change-scenes)

---
## Build Prerequisities

-   [Git](http://git-scm.com/)
-   [Node.js](http://nodejs.org/) (with NPM)

> This script uses OBS plugin "obs-websocket" in conjunction with "OBS-Studio". For monitoring "NGINX-RTMP" (see links below).

-   [OBS-Studio](https://github.com/obsproject/obs-studio/)
-   [OBS-WEBSOCKET](https://github.com/Palakis/obs-websocket/)

> NOALBS supports monitoring streams on either NGINX-RTMP server, Node-Media-Server, Nimble Server, and even SLS (SRT-Live-Server).
Node-Media-Server is also built into NOALBS for an easy all-in-one streaming solution.

-   [NGINX-RTMP](https://github.com/arut/nginx-rtmp-module/)
-   [Node-Media-Server](https://github.com/illuspas/Node-Media-Server/)
-   [Nimble Server](https://wmspanel.com/nimble/install)
-   [SLS (SRT-Live-Server)](https://gitlab.com/mattwb65/srt-live-server)

<sub>_[(table of contents)](#table-of-contents)_</sub>

---
## Installation from Source and NGINX Setup

-   `git clone <repository-url>` or download from [RELEASES](https://github.com/715209/nginx-obs-automatic-low-bitrate-switching/releases)
-   Change into the new directory.
-   `npm install --production`

If you already have NGINX-RTMP server
-   Replace your `nginx.conf` with the one given here.
-   Put `stat.xsl` in your nginx `html` folder.

Otherwise here is a Windows version of NGINX+RTMP Server
- [Hosted on Github](https://github.com/715209/nginx-obs-automatic-low-bitrate-switching/raw/master/nginx/nginx_1.7.11.3_Gryphon_With_NOALBS.config_files_03162020.zip)
- Everything is ready to go inside this zip, just extract and click on the `nginx_start.bat` file to start NGINX, you can use `nginx_stop.bat` to stop NGINX. HTTP server runs on Port `80`, RTMP server runs on `1935`, if you need to edit the config file it's in the `/conf` folder, named `nginx.conf`.

<sub>_[(table of contents)](#table-of-contents)_</sub>

---
## Configuring the CONFIG and Setting up Roles

Edit `config.json` to your own settings.

Here is an example config with comments (click to enlarge):

**_PLEASE NOTE CONFIG ARCHITECTURE MAY NOT REPRESENT CURRENT CONFIG ARCHITECTURE_**

![alt text](https://i.imgur.com/cVbz1bN.png "Configuration Comments (Click to Enlarge)")

-   Use https://twitchapps.com/tmi to get your oauth from Twitch for use with chat commands.
    > We recommend using your main Twitch BOT account for this, but if you do not have a Twitch Bot account just use your Main Twitch Account.

Also if you are going to be using your Twitch BOT Account please make sure it is an 'Editor' of your channel, see example;

Go to this URL: [Twitch User Roles](https://www.twitch.tv/dashboard/roles/), Find your bot on the list, add checkmark to 'Editor', done.

![alt text](https://i.imgur.com/yRlBe5U.png "Setting your bot as Editor")

<sub>_[(table of contents)](#table-of-contents)_</sub>

---
## How to run from source

Run the node app by running: `npm start`. Then stream to `rtmp://IPHERE/publish/live`

<sub>_[(table of contents)](#table-of-contents)_</sub>

---
## Chat Commands

This script gives you the option to enable some simple chat commands to help you manage your stream from your own Twitch chat, here is how to use them:

> Please note: Admins are all the users in the "adminUsers" array in the config, MODs (if enabled in the config) are all of your MODs, and Public (if enabled in the config) is anyone in your chat.

> | Default Role | Command                               | Description                                                                                             | Example              |
> | :----------: | ------------------------------------- | :------------------------------------------------------------------------------------------------------ | :------------------- |
> |    Admins    | !host (channelname)                   | hosts said channel, and stops streaming in OBS if enabled in config.                                    | !host 715209         |
> |    Admins    | !unhost                               | unhosts whoever you are currently hosting.                                                              | !unhost              |
> |    Admins    | !raid (channelname)                   | raids said channel and stops streaming in OBS if enabled in config.                                     | !raid 715209         |
> |    Admins    | !start                                | on-demand command to start streaming in OBS.                                                            | !start               |
> |    Admins    | !stop                                 | on-demand command to stop streaming in OBS.                                                             | !stop                |
> |    Admins    | !rec (on/off)                         | on-demand command to start/stop recording in OBS.                                                       | !rec on              |
> |    Admins    | !switch (scene)                       | switches to the provided SCENE ([fuzzy match](https://wikipedia.org/wiki/Approximate_string_matching)). | !switch INTRO        |
> |    Admins    | !alias (add/remove) (alias) (command) | add an alias for a command                                                                              | !alias add ss switch |
> |     MODs     | !refresh                              | changes to the REFRESH scene for the set interval.                                                      | !refresh             |
> |     MODs     | !fix                                  | tries to fix the stream. (ONLY works with NGINX-RTMP server type.)                                      | !fix                 |
> |     MODs     | !trigger (value)                      | changes the low bitrate threshold to the defined value.                                                 | !trigger 1000        |
> |     MODs     | !sourceinfo                           | gives you details about the SOURCE in chat.                                                             | !sourceinfo          |
> |     MODs     | !obsinfo                              | gives you details about OBS in chat.                                                                    | !obsinfo             |
> |    Public    | !bitrate                              | returns current BITRATE                                                                                 | !bitrate             |

You can also enable/disable certain features from chat, see below:

> | Default Role | Command            | Description                                                | Example      |
> | :----------: | ------------------ | :--------------------------------------------------------- | :----------- |
> |    Admins    | !public (on/off)   | enables/disables the use of Public commands.               | !public off  |
> |    Admins    | !mod (on/off)      | enables/disables the use of MOD commands.                  | !mod on      |
> |    Admins    | !notify (on/off)   | enables/disables the notifications in chat.                | !notify off  |
> |    Admins    | !autostop (on/off) | enables/disables the auto stop feature when you host/raid. | !autostop on |

<sub>_[(table of contents)](#table-of-contents)_</sub>

---
## How to pull RTMP stream into OBS

Update your OBS to v26+ and follow the steps below:

1.) In OBS create the following scenes:
- `LIVE`, `LOW`, `BRB`, `REFRESH`
 - I highly recommend creating a `STARTUP` & `PRIVACY` scene, the `STARTUP` scene can contain whatever you want to start your stream on and then switch to `LIVE` when you're ready, the `PRIVACY` scene can be whatever you want to put the stream on when you need privacy, the main thing is that it's out of NOALBS scope and won't automatically switch scenes.
- The normal flow is to have your OBS on `STARTUP` when you start stream and when you're ready either you or an instructed MOD can !switch LIVE, when you need privacy use !switch PRIVACY.

2.) In your `LIVE` scene, add a 'Media Source', match the image below:
![image](https://user-images.githubusercontent.com/1740542/108275687-54d2a700-713c-11eb-8ee0-22142cfc02a6.png)
  - Right click on the 'Media Source' > Transform > Stretch to screen (this will stretch the video source no matter the resolution, ex; 480p, 720p, 1080p etc.)

3.) Copy and Paste(Reference) the 'Media Source' from the `LIVE` scene into your `LOW` scene.
  - Do the same transformation procedure from step (2).

4.) Go over all of your scenes and make them your own.

<sub>_[(table of contents)](#table-of-contents)_</sub>

---
## Running with other servers (not NGINX):
### Using the built-in Node-Media-Server
Defining a `nodeMediaServer` block in config.json will enable a fully functional node-media-server RTMP server to accept incoming streams:

```JSON
    "rtmp": {
        "application": "publish",
        "key": "live"
    },
    "nodeMediaServer": {
        "rtmp": {
            "port": 1935,
            "chunk_size": 60000,
            "gop_cache": true,
            "ping": 30,
            "ping_timeout": 60
        },
        "http": {
            "port": 8000
        }
    },
```

> The `nodeMediaServer` object is passed directly as the node-media-server configuration, [more details here](https://github.com/illuspas/Node-Media-Server#npm-version-recommended). It will also automatically fill out the rtmp server type and stats fields.

> Note: This is probably best for local connections and testing only unless you [enable authentication](https://github.com/illuspas/Node-Media-Server#authentication)

<sub>_[(table of contents)](#table-of-contents)_</sub>

---
### Using an external Node-Media-Server
Modify the RTMP section in config.json like this to connect to a node-media-server running externally:

```JSON
    "rtmp": {
        "server": "node-media-server",
        "stats": "http://localhost:8000/api/streams",
        "application": "publish",
        "key": "live"
    },
```

<sub>_[(table of contents)](#table-of-contents)_</sub>

---
### Using Nimble Streamer Server (with SRT protocol)

Nimble must have [API access enabled](https://wmspanel.com/nimble/api) and be configured as a SRT receiver - see ["Set up receiving of SRT"](https://blog.wmspanel.com/2017/07/setup-srt-secure-reliable-transport-nimble-streamer.html) and have an outgoing stream ("Add outgoing stream" on same page)

Modify the RTMP section in config.json to this:

```JSON
    "rtmp": {
        "server": "nimble",
        "stats": "http://nimble:8082",
        "id": "0.0.0.0:1234",
        "application": "live",
        "key": "srt"
    },
```

- `stats`: URL to nimble API
- `id`: UDP listener ID (Usually IP:Port)
- `application`: Outgoing stream "Application Name"
- `key`: Outgoing stream "Stream Name"

>Switches on low bitrate or high RTT (high RTT seems to be a more accurate way of determining if the stream is bad with this)

You can change the high RTT trigger value inside config.json:

```JSON
    "obs": {
        ...
        "highRttTrigger": 2500,
    },
```

<sub>_[(table of contents)](#table-of-contents)_</sub>

---
### Using SLS (SRT-LIVE-SERVER)
> Big Thanks to [oozebrood](https://www.twitch.tv/oozebrood), [matthewwb2](https://www.twitch.tv/matthewwb2), and [kyle___d](https://www.twitch.tv/kyle___d) for all of the hard work they've put into getting SRT to the masses!

If you're using [Matt's modified version](https://gitlab.com/mattwb65/srt-live-server) of SLS then follow this section;

You MUST Modify the ENTIRE RTMP section in NOALBS `config.json` file to match this:

(And yes we know calling this configuration block the 'RTMP section' is rather dumb at this point, but it is what it is until it is changed.)

```JSON
    "rtmp": {
        "server": "srt-live-server",
        "stats": "http://127.0.0.1:8181/stats",
        "publisher": "publish/live/feed1"
    },
```

- `server`: Type of streaming server. (ex; nginx, nms, nimble or srt-live-server )
- `stats`: URL to SLS stats page (ex; http://127.0.0.1:8181/stats )
- `publisher`: StreamID of the where you are publishing the feed. (ex; publish/live/feed1 )

    - Publisher, what is a publisher? it's a combination of `domain_publisher`/`app_publisher`/`<whatever-you-want>`.
    - So if your `domain_publisher` was "uplive.sls.com", and your `app_publisher` was "live", it would be `uplive.sls.com/live/<whatever-you-want>`.
    - You could literally call you domain_publisher 'billy', app_publisher 'bob', and then set your streamid (publisher) to 'billy/bob/thorton' if you wanted to.
    - Publisher is also what you entered in the config under `default_sid`.  Unless you are streaming to a different 'StreamID' of course, ex; `publish/live/tinkerbell`.


See Example Below from the `sls.conf` file in the SLS main directory:
    
![image](https://user-images.githubusercontent.com/1740542/94368739-7955b600-00ab-11eb-9946-20b66f9f4fb2.png)

So in actuality your 'publisher' is your default `StreamID`, like in the example above it's `billy/bob/thorton`.

>Switches on low bitrate or high RTT (high RTT seems to be a more accurate way of determining if the stream is bad with this)

You can change the high RTT trigger value inside config.json:

```JSON
    "obs": {
        ...
        "highRttTrigger": 2500,
    },
```

====
How do I publish to the SLS Server? see [HERE](https://gitlab.com/mattwb65/srt-live-server#1test-with-ffmpeg)

How do I pull the SRT feed into OBS?

- Add Media Source
- Uncheck Local File
- In the "Input" field enter in: `srt://<SERVER-IP>:<PORT>/?streamid=<PUBLISHER>`
- In the "Input Format" field enter in: `mpegts`
- Check `Seekable` then click `OK`

<sub>_[(table of contents)](#table-of-contents)_</sub>

---
## Help it won't change scenes

It will only change scenes when OBS is set on a scene that's in the config.  
(This is so that it wont change when you are on for example your 'intro' or 'locked-brb' scene)

<sub>_[(table of contents)](#table-of-contents)_</sub>
