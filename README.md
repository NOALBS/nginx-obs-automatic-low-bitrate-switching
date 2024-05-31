# NOALBS

A simple executable for all Operating Systems (Windows, Mac & Linux) to automatically switch scenes in OBS Studio/OBS.Live; based on the current bitrate fetched from your ingest server stats.  NOALBS is used as a DIY tool to have your OBS Studio/OBS.Live auto switch scenes when you are either in a LOW bitrate situation or if your source disconnects completely from your ingest server (RTMP, SRT Etc..).

---
### Connect with Us:
| <a href="https://discord.gg/efWu5HWM2u"><img src="https://github.com/715209/nginx-obs-automatic-low-bitrate-switching/assets/1740542/8e96e359-373e-441a-b653-4c613d6dc521" alt="NOALBS DISCORD" width="50" height="auto"></a> | <a href="https://twitter.com/NOALBS"><img src="https://github.com/715209/nginx-obs-automatic-low-bitrate-switching/assets/1740542/7b0b20d7-5dd7-4666-a3f0-089faa517fa3" alt="NOALBS TWITTER" width="50" height="auto"></a> |
|---|---|
|[Discord](https://discord.gg/efWu5HWM2u)|[Twitter](https://twitter.com/NOALBS)|

---
# Similar Solutions / Paid Services

Don't feel like setting this all up by yourself?
- Message me [@b3ck](https://discordapp.com/channels/@me/96991451006660608) on Discord and I'll do everything for you for a fee 😄👍

  or..

  
- Check out these links for similar solutions/paid services:

| Service Name | Details/Slogan |
|---|---|
| [BELABOX](https://cloud.belabox.net) | The only SRT/SRTLA service optimized for bonding with a [BELABOX DIY](https://b3ck.com/b3cks-belabox) / [BELABOX BEE](https://shop.belabox.net/product/belabox-bee) |
| [IRLHosting](https://irlhosting.com/?ref=noalbs) | Cloud hosting and endpoints |
| [AntiScuff](https://antiscuff.com/?utm_source=noalbs&utm_campaign=referral&utm_medium=landing) | Cloud OBS Hosting |
| [IRLToolkit](https://irltoolkit.com/) | Step your stream up to the next level! |
| [noRIP.io](https://www.norip.io) | Services to stabilize your stream |
| [StreamAtom](https://streamatom.com/irl?screen=RtmpServer) | Powered by the StreamAtom Team, Supported by you. |
| [IRLtools](https://irltools.com?ref=NOALBS) | IRL Accessories & Guides / Cloud Solutions Coming Soon.. |

Do you offer a similar solution or paid service? Want your link here? Message [@b3ck](https://discordapp.com/channels/@me/96991451006660608) on Discord.
- Also if your service is already listed above and you would like me to add any type of tracking to the URL so you know where your traffic is coming from send me a DM on Discord.

---

# Quick Start

- Download the latest binary from [releases](https://github.com/715209/nginx-obs-automatic-low-bitrate-switching/releases)
- Have [OBS-Studio](https://github.com/obsproject/obs-studio/) and [OBS-WebSocket](https://github.com/obsproject/obs-websocket/releases/latest) 5.0.0 or higher installed
- [Configure NOALBS](#configure-noalbs)
- Run the executable

# Chat Commands

This script gives you the option to enable some simple chat commands to help you manage your stream from your own Twitch chat, here is how to use them:

> Please note: Admins are all the users in the `admins` array in `chat` config section, MODs are all of your MODs, and Public is anyone in your chat.

| Default Role | Command                  | Description                                                                                             | Example            |
| :----------: | ------------------------ | :------------------------------------------------------------------------------------------------------ | :----------------- |
|    Admins    | !start                   | on-demand command to start streaming in OBS.                                                            | !start             |
|    Admins    | !stop                    | on-demand command to stop streaming in OBS.                                                             | !stop              |
|    Admins    | !record                  | on-demand command to toggle recording in OBS.                                                           | !record            |
|    Admins    | !collection (name)       | changes the scene collection and profile.                                                               | !collection twitch |
|    Admins    | !alias (alias) (command) | add an alias for a command.                                                                             | !alias ss switch   |
|    Admins    | !alias rem (alias)       | removes an alias for a command.                                                                         | !alias rem ss      |
|    Admins    | !switch (scene)          | switches to the provided SCENE ([fuzzy match](https://wikipedia.org/wiki/Approximate_string_matching)). | !switch INTRO      |
|    Admins    | !source (value)          | Toggles an OBS source item visibility on the current scene                                              | !source media      |
|    Admins    | !live                    | switch to the live scene.                                                                               | !live              |
|    Admins    | !privacy                 | switch to the privacy scene.                                                                            | !privacy           |
|    Admins    | !starting                | switch to the starting scene.                                                                           | !starting          |
|    Admins    | !ending                  | switch to the ending scene.                                                                             | !ending            |
|    Admins    | !noalbs prefix (prefix)  | change noalbs command prefix.                                                                           | !noalbs prefix #   |
|    Admins    | !noalbs retry (value)    | changes the retry value for the switcher.                                                               | !noalbs retry 5    |
|    Admins    | !noalbs lang (value)     | changes the chat response language.                                                                     | !noalbs lang zh_tw |
|     MODs     | !trigger (value)         | changes the low bitrate threshold to the defined value.                                                 | !trigger 800       |
|     MODs     | !otrigger (value)        | changes the offline bitrate threshold to the defined value.                                             | !otrigger 200      |
|     MODs     | !rtrigger (value)        | changes the RTT based low threshold to the defined value.                                               | !rtrigger 1500     |
|     MODs     | !ortrigger (value)       | changes the RTT based offline threshold to the defined value.                                           | !ortrigger 2000    |
|     MODs     | !sourceinfo              | gives you details about the SOURCE in chat.                                                             | !sourceinfo        |
|     MODs     | !serverinfo              | gives you details about the SERVER in chat.                                                             | !serverinfo        |
|     MODs     | !fix                     | tries to fix the stream.                                                                                | !fix               |
|     MODs     | !refresh                 | tries to fix the stream.                                                                                | !refresh           |
|    Public    | !bitrate                 | returns the current bitrate.                                                                            | !bitrate           |

You can also enable/disable certain features from chat, see below:

| Default Role | Command              | Description                                                | Example         |
| :----------: | -------------------- | :--------------------------------------------------------- | :-------------- |
|    Admins    | !public (on/off)     | enables/disables the use of Public commands.               | !public off     |
|    Admins    | !mod (on/off)        | enables/disables the use of MOD commands.                  | !mod on         |
|    Admins    | !notify (on/off)     | enables/disables the notifications in chat.                | !notify off     |
|    Admins    | !autostop (on/off)   | enables/disables the auto stop feature when you host/raid. | !autostop on    |
|    Admins    | !noalbs (start/stop) | NOALBS start/stop switching scenes.                        | !noalbs stop    |
|    Admins    | !noalbs instant      | toggle instant switching from offline scene.               | !noalbs instant |

# Configure NOALBS

Open the `.env` file. If you have a custom Twitch account created for a bot fill in your Twitch Account bot username and oauth.

Use <https://twitchapps.com/tmi> to get your oauth from Twitch.

> We recommend using your Twitch BOT account for this, but if you do not have a Twitch Bot account just use your Main Twitch Account.

The `config.json` file holds all the user configurations.

## EXAMPLE CONFIG.JSON (DO NOT COPY PASTA)

```json
{
  "user": { 
    "id": null,                                     // Leave ID null, this is for future use.
    "name": "example",                              // You can change "example" to your name if you like, for instance "b3ck"
    "passwordHash": null                            // Leave passwordHash null, this is for future use.
  },
  "switcher": {
    "bitrateSwitcherEnabled": true,                 // Enable or Disable auto scene switching in OBS based on your current bitrate.
    "onlySwitchWhenStreaming": false,               // Enable or Disable the requirement switching only if OBS has streaming active.
    "instantlySwitchOnRecover": true,               // Bypass retryAttempts and instantly switch to live on bitrate recovery.
    "autoSwitchNotification": true,                 // Enable or Disable chat notifications when auto switching scenes.
    "retryAttempts": 5,                             // Number of retry attempts NOALBS will check bitrate before actually switching.
    "triggers": {
      "low": 500,                                   // Low Bitrate threshold in kbps.
      "rtt": 1000,                                  // RTT threshold in ms for SRT.
      "offline": 450                                // Bitrate in kbps to switch to your offline scene.
    },
    "switchingScenes": {
      "normal": "Live",                             // Scene you want to use in OBS when your bitrate is above your low bitrate threshold.
      "low": "Low",                                 // Scene you want to use in OBS when your bitrate is below your low bitrate threshold.
      "offline": "Disconnected"                     // Scene you want to use in OBS when your bitrate is below your offline threshold.
    },
    "streamServers": [
      {
        "streamServer": {
          "type": "Belabox",                        // This can either; Nginx, NodeMediaServer, Nimble, SrtLiveServer or Belabox
          "statsUrl": "http://example.com/stats",   // URL to your stats page.
          "publisher": "example"                    // Publisher that you're using for the server.
        },
        "name": "BELABOX cloud",                    // Name you want to give you server which will be shown in chat when you do !bitrate
        "priority": 0,                              // Priority you want NOALBS to see this server in, 0 behing the highest.
        "overrideScenes": null,                     // Optional field to override the default scenes.
        "dependsOn": null,                          // Optional field explained here: https://github.com/715209/nginx-obs-automatic-low-bitrate-switching#depends-on
        "enabled": true                             // Enable or Disable the server, this allows you to have mutiple servers saved, then pick and choose which ones you want enabled.
      }
    ]
  },
  "software": {
    "type": "Obs",                                  // NOALBS supports OBS WebSocket v4 and v5. To still use v4 use type ObsOld.
    "host": "localhost",                            // Host of your OBS, most commonly used is; localhost.
    "password": "example",                          // Password to the OBS Websockets.
    "port": 4455,                                    // Port to the OBS Websockets.
    "collections": {
      "twitch": {
        "profile": "twitch_profile",
        "collection": "twitch_scenes"
      },
      "kick": {
        "profile": "kick_profile",
        "collection": "kick_scenes"
      }
    }
  },
  "chat": {
    "platform": "Twitch",                           // Twitch is the only current platform supported as of right now.
    "username": "example",                          // Username of your main Twitch Account.
    "admins": [                                     // List of admins in the form of an array, the last name in the array doesn't need a comma.
      "username1",
      "username2",
      "username3"
    ],
    "language": "EN",                               // Language variants: `DE`, `DK`, `EN`, `ES`, `FR`, `IT`, `NB`, `NL`, `PL`, `PTBR`, `RU`, `SV`, `TR`, `ZHTW`
    "prefix": "!",                                  // Prefix you want to use in chat to send commands to NOALBS from chat.
    "enablePublicCommands": false,                  // Enable or Disable public commands to anyone can use !bitrate in chat.
    "enableModCommands": true,                      // Enable or Disable mod only commands.
    "enableAutoStopStreamOnHostOrRaid": true,       // Enable or Disable auto stop stream in OBS when raiding or hosting.
    "commands": {                                   // Command Options to override defaults to be used in chat.
      "Fix": {                                      // Full Command Name
        "permission": null,                         // null = Administrators/Default, Public = Public, Vip = VIP, Mod = Moderators, Admin = Administrators
        "userPermissions": ["715209"],              // Allow users to perform this command skipping permission check.
        "alias": [
          "f"                                       // Alias to be used in chat, so for this example, !f would replace !fix
        ]
      },
      "Switch": {
        "permission": "Mod",
        "alias": [
          "ss"
        ]
      },
      "Bitrate": {
        "permission": null,
        "alias": [
          "b"
        ]
      }
    }
  },
  "optionalScenes": {                               // Optional scenes you can switch to from chat with these simple commands;
    "starting": null,                               // !starting
    "ending": null,                                 // !ending
    "privacy": "privacy",                           // !privacy
    "refresh": null                                 // !refresh
  },
  "optionalOptions": {
    "twitchTranscodingCheck": false,                // Enable or Disable transcoding check on twitch for your stream. (Non-Partners Only)
    "twitchTranscodingRetries": 5,                  // How many times you would like the transcoding checked before giving up.
    "twitchTranscodingDelaySeconds": 15,            // How long you want to wait to check inbetween checks.
    "offlineTimeout": null,                         // OBS will stop streaming when you you've been in your offline scene for this amount of time.
    "recordWhileStreaming": false,                  // Automatically record when you start streaming.
    "switchToStartingSceneOnStreamStart": false,    // Automatically switch to the starting scene when you start streaming.
    "switchFromStartingSceneToLiveScene": false     // Automatically switch to live scene when you start streaming IRL.
  }
}
```

## More In-Depth Explanations of each section below

### Software section

```json
"software": {
  "type": "Obs",
  "host": "localhost",
  "password": "password",
  "port": 4455,
  "collections": {
    "twitch": {
      "profile": "twitch_profile",
      "collection": "twitch_scenes"
    },
    "kick": {
      "profile": "kick_profile",
      "collection": "kick_scenes"
    }
  }
}
```

- `type`: Replace this with the software you're using.

NOALBS supports OBS WebSocket v4 and v5. To still use v4 use type `ObsOld`.

- `collections`: Optional configurable scene collections and profiles to be used with the `!collection` command.

## Stream servers section

Currently NOALBS supports [NGINX](#using-nginx), [Nimble](#using-nimble-streamer-server-with-srt-protocol), [Node Media Server](#using-an-external-node-media-server), [SRT Live Server](#using-sls-srt-live-server), [BELABOX](#using-belabox-cloud), [MediaMTX](#using-mediamtx) and [OBS Sources](#using-an-obs-source).
You can have as many servers as you want to use in the config.

Example stream server object:

```json
{
  "streamServer": {
    "type": "Nginx",
    "statsUrl": "http://localhost/stats",
    "application": "publish",
    "key": "live"
  },
  "name": "nginx",
  "priority": 0,
  "overrideScenes": {
    "normal": "normal",
    "low": "low",
    "offline": "offline"
  },
  "dependsOn": null,
  "enabled": true
}
```

- `streamServer`: Replace the entire `streamServer` section with the one of [these](#stream-server-objects).
- `type`: Nginx, NodeMediaServer, Nimble, SrtLiveServer, Belabox, or Mediamtx
- `name`: A unique name to distinguish the server
- `priority`: Decides which stream server to monitor when multiple are online. 0 is consired the highest.
- `overrideScenes`: Optional field to override the default scenes
- `dependsOn`: Optional field explained [here](#depends-on)

## Stream server objects

### Using NGINX

```JSON
  "streamServer": {
    "type": "Nginx",
    "statsUrl": "http://localhost/stats",
    "application": "publish",
    "key": "live"
  },
```

---

### Using an external Node-Media-Server

```JSON
"streamServer": {
  "type": "NodeMediaServer",
  "statsUrl": "http://localhost:8000/api/streams",
  "application": "publish",
  "key": "live",
  "auth": {
    "username": "admin",
    "password": "admin"
  }
},
```

- `auth`: Optional field

---

### Using Nimble Streamer Server (with SRT protocol)

Nimble must have [API access enabled](https://wmspanel.com/nimble/api) and be configured as a SRT receiver - see ["Set up receiving of SRT"](https://blog.wmspanel.com/2017/07/setup-srt-secure-reliable-transport-nimble-streamer.html) and have an outgoing stream ("Add outgoing stream" on same page)

```JSON
"streamServer": {
  "type": "Nimble",
  "statsUrl": "http://nimble:8082",
  "id": "0.0.0.0:1234",
  "application": "live",
  "key": "srt",
},
```

- `statsUrl`: URL to nimble API
- `id`: UDP listener ID (Usually IP:Port)
- `application`: Outgoing stream "Application Name"
- `key`: Outgoing stream "Stream Name"

> Switches on low bitrate or high RTT (high RTT seems to be a more accurate way of determining if the stream is bad with this)
You can change the high RTT trigger value inside config.json

---

### Using SLS (SRT-LIVE-SERVER)

> Big Thanks to [oozebrood](https://www.twitch.tv/oozebrood), [matthewwb2](https://www.twitch.tv/matthewwb2), and [kyle___d](https://www.twitch.tv/kyle___d) for all of the hard work they've put into getting SRT to the masses!
If you're using either [Matt's modified version](https://gitlab.com/mattwb65/srt-live-server) or
[my edit of Matt's version](https://hub.docker.com/r/b3ckontwitch/sls-b3ck-edit) of SLS then follow this section;

```JSON
"streamServer": {
  "type": "SrtLiveServer",
  "statsUrl": "http://localhost:8181/stats",
  "publisher": "publish/live/feed1"
},
```

- `stats`: URL to SLS stats page (ex; <http://localhost:8181/stats> )
- `publisher`: StreamID of the where you are publishing the feed. (ex; publish/live/feed1 )

- Publisher, what is a publisher? it's a combination of `domain_publisher`/`app_publisher`/`<whatever-you-want>`.
  - So if your `domain_publisher` was "uplive.sls.com", and your `app_publisher` was "live", it would be `uplive.sls.com/live/<whatever-you-want>`.
  - You could literally call you domain_publisher 'billy', app_publisher 'bob', and then set your streamid (publisher) to 'billy/bob/thorton' if you wanted to.
  - Publisher is also what you entered in the config under `default_sid`.  Unless you are streaming to a different 'StreamID' of course, ex; `publish/live/tinkerbell`.

See Example Below from the `sls.conf` file in the SLS main directory:

![image](https://user-images.githubusercontent.com/1740542/94368739-7955b600-00ab-11eb-9946-20b66f9f4fb2.png)

So in actuality your 'publisher' is your default `StreamID`, like in the example above it's `billy/bob/thorton`.

> Switches on low bitrate or high RTT (high RTT seems to be a more accurate way of determining if the stream is bad with this)
You can change the high RTT trigger value inside config.json:

#### How do I publish to the SLS Server?

see [HERE](https://gitlab.com/mattwb65/srt-live-server#1test-with-ffmpeg)

#### How do I pull the SRT feed into OBS?

- Add Media Source
- Un-check `Local File`
- Make sure `Restart playback when source becomes active` is checked.
- Change Network Buffering to `1 MB`
- In the "Input" field enter in: `srt://<SERVER-IP>:<PORT>/?streamid=<PUBLISHER>`
  - Using the example from the above image, it would be something like this if you are using SLS on the same machine: `srt://localhost:30000/?streamid=jojo/bob/thorton`
- In the `Input Format` field enter in: `mpegts`
- Change `Reconnect Delay` to `3S` (Three Seconds)
- Make sure `Show nothing when playback ends` is checked.
- Check `Close file when inactive`
- Optional: If your streams color looks washed out change your `YUV Color Range` to `Partial`
- Sidenote: Do not Use `Use hardware decoding when available`, `Apply alpha in linear space`, or `Seekable`, As I have only ran into issues with these options enabled.

Below is an example when used with a BelaBox Reciever:

![image](https://user-images.githubusercontent.com/1740542/147401054-9b99d8ea-4388-441d-a965-4e0924de30e2.png)

Remember this is just an example, your ports and streamid may differ.

---

### Using BELABOX cloud

```JSON
  "streamServer": {
    "type": "Belabox",
    "statsUrl": "http://belabox-stats-url/yourkey",
    "publisher": "yourkey"
  },
```

You should be able to find the details in your [BELABOX cloud](https://cloud.belabox.net) account.

---

### Using MediaMTX

```JSON
  "streamServer": {
    "type": "Mediamtx",
    "statsUrl": "http://localhost:9997/v3/paths/get/{name}",
    "auth": {
      "username": "user",
      "password": "pass"
    }
  },
```

- `statsUrl`: The API stats page. Replace `{name}` with the name of your stream.
- `auth`: Optional field

For more details, refer to the [MediaMTX documentation](https://github.com/bluenviron/mediamtx#table-of-contents).

---

### Using RIST

```JSON
  "streamServer": {
    "type": "Rist",
    "statsUrl": "http://localhost:8681/stats"
  },
```

- `statsUrl`: URL to RIST stats page (ex; <http://localhost:8681/stats> )

---

### Using Xiu

```JSON
  "streamServer": {
    "type": "Xiu",
    "statsUrl": " http://localhost:8000/api/query_stream",
    "application": "live",
    "key": "source",
  },
```

- `statsUrl`: URL to stats page (ex; <http://localhost:8000/api/query_stream> )

For more details, refer to the [Xiu documentation](https://www.rustxiu.com/docs/httpapi/http-api/).

---

### Using an OBS Source

```JSON
  "streamServer": {
    "type": "Obs",
    "source": "Media/VLC Source"
  },
```

## Depends on

When a `dependsOn` field is found, monitor the status of the given server. If that server goes offline the `backupScenes` will be used.

```JSON
"dependsOn": {
  "name": "nginx",
  "backupScenes": {
    "normal": "Scene 3",
    "low": "low",
    "offline": "Scene 2"
  }
}
```

- `name`: The exact name this stream server depends on
- `backupScenes`: Scenes that will be used when the depended on server is offline

# Languages

To change the language that you see in chat responses, locate or add the `"language": "EN",` line above your `"prefix": "!",` parameter under the `chat` section in the NOALBS `config.json` file. The language variant must be in **UPPERCASE** and should be one of the following:

- `DE`: German
- `DK`: Danish
- `EN`: English
- `ES`: Spanish
- `FR`: French
- `IT`: Italian
- `NB`: Norwegian Bokmål
- `NL`: Dutch
- `PL`: Polish
- `PTBR`: Portuguese (Brazil)
- `RU`: Russian
- `SV`: Swedish
- `TR`: Turkish
- `ZHTW`: Traditional Chinese (Taiwan)

Example excerpt from the NOALBS `config.json` file:
```json
"language": "EN",
"prefix": "!",
"enablePublicCommands": true,
"enableModCommands": true,
```

# Building from source

Download and install:

- [Git](http://git-scm.com/)
- [Rust](https://www.rust-lang.org)

Then:

- `git clone <repository-url>` or download from GitHub.
- `cd <repository-name>`
- `cargo run` or `cargo run --release`

# FAQ

## Need additional help?

You can always contact us on discord @ b3ck#3517 or 715209#0600 and we'll do our best to help out.

## I want to keep using the old version

You can find the old NOALBS version in the [master branch](https://github.com/715209/nginx-obs-automatic-low-bitrate-switching/tree/master).

## NGINX Setup

If you already have NGINX-RTMP server

- Replace your `nginx.conf` with the one given here.
- Put `stat.xsl` in your nginx `html` folder.

Otherwise here is a Windows version of NGINX+RTMP Server

- [Hosted on Github](https://github.com/715209/nginx-obs-automatic-low-bitrate-switching/raw/master/nginx/nginx_1.7.11.3_Gryphon_With_NOALBS.config_files_03162020.zip)
- Everything is ready to go inside this zip, just extract and click on the `nginx_start.bat` file to start NGINX, you can use `nginx_stop.bat` to stop NGINX. HTTP server runs on Port `80`, RTMP server runs on `1935`, if you need to edit the config file it's in the `/conf` folder, named `nginx.conf`.

---

## How to publish to your NGINX RTMP Server

Using the default config where the server is listening on port `1935` and the name of the application in NGINX is `publish`;

<sub>_(example config, do not copy)_</sub>

```NGINX
rtmp {
    server {
        listen 1935;
        (...)
        
        # Stream to "rtmp://IPHERE/publish/live".
        application publish {
            live on;
            (...)
        }
    }
}
```

- If the app or device requires the key separately put `rtmp://(SERVER-IP):1935/publish` in the RTMP URL and `live` in the key.
- Otherwise if the app or device doesn't require the key separately put `rtmp://(SERVER-IP):1935/publish/live` in the RTMP URL.

Most of these rules apply to the rest of the other types of servers;

- RTMP will usually have an `application` and a `key`.
- SRT will use a `publisher` ID or `streamid`, some applications or devices will only require your server IP and PORT;
  - Example; `srt://(SERVER-IP):30000` but if it the app or device supports `streamid` it will always be separate.

Either way, pay close attention to your app or device requirements, as you will need to setup accordingly to them and your configuration on the server.

---

## How to pull RTMP stream into OBS

Update your OBS to v26+ and follow the steps below:

1.) In OBS create the following scenes:

- `LIVE`, `LOW`, `BRB`, `REFRESH`
- I highly recommend creating a `STARTUP` & `PRIVACY` scene, the `STARTUP` scene can contain whatever you want to start your stream on and then switch to `LIVE` when you're ready, the `PRIVACY` scene can be whatever you want to put the stream on when you need privacy, the main thing is that it's out of NOALBS scope and won't automatically switch scenes.
- The normal flow is to have your OBS on `STARTUP` when you start stream and when you're ready either you or an instructed MOD can !switch LIVE, when you need privacy use !switch PRIVACY.

2.) In your `LIVE` scene, add a 'Media Source', match the image below:

![image](https://user-images.githubusercontent.com/1740542/113238763-05b97f00-926f-11eb-86d2-d9aabf94bf08.png)

- Right click on the 'Media Source' > Transform > Stretch to screen (this will stretch the video source no matter the resolution, ex; 480p, 720p, 1080p etc.)

3.) Copy and Paste(Reference) the 'Media Source' from the `LIVE` scene into your `LOW` scene.

- Do the same transformation procedure from step (2).

4.) Go over all of your scenes and make them your own.

---

## How to run with multiple users

In the `.env` file add the line `CONFIG_DIR=configs` where `configs` is the folder that holds all the config files. The name of the config is ignored so you can name it anything you want.

---

## How to log to a file instead

In the `.env` file add the line `LOG_DIR=logs` and `LOG_FILE_NAME=noalbs.log` where `LOG_DIR` is the folder that holds all the log files and `LOG_FILE_NAME` the prefix used for the file name. A new log file will be generated daily.

---

## Help it won't change scenes

It will only change scenes when OBS is set on a scene that's in the config.  
(This is so that it wont change when you are on for example your 'intro' or 'locked-brb' scene)
