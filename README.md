```
    ███╗   ██╗ ██████╗  █████╗ ██╗     ██████╗ ███████╗
    ████╗  ██║██╔═══██╗██╔══██╗██║     ██╔══██╗██╔════╝
    ██╔██╗ ██║██║   ██║███████║██║     ██████╔╝███████╗
    ██║╚██╗██║██║   ██║██╔══██║██║     ██╔══██╗╚════██║
    ██║ ╚████║╚██████╔╝██║  ██║███████╗██████╔╝███████║
    ╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝ ╚══════╝
```

# nginx-obs-automatic-low-bitrate-switching

## How to video:
[![YouTube: How to Download, Install and Run NOALBS for IRL Livestreaming](https://img.youtube.com/vi/rglDAIm73cM/00.jpg)](https://www.youtube.com/watch?v=rglDAIm73cM)

---

Simple app to automatically switch scenes in OBS based on the current bitrate fetched from an RTMP server's stats page.

Don't feel like setting this all up by yourself? Check out these links for similar solutions!

-   [psynapticmedia.com](http://www.psynapticmedia.com/super-stream-system-by-psynaps/)
-   [norip.io](https://www.norip.io)

## Build Prerequisities

-   [Git](http://git-scm.com/)
-   [Node.js](http://nodejs.org/) (with NPM)

> This script uses OBS plugin "obs-websocket" inconjuction with "OBS-Studio". For monitoring "NGINX-RTMP" (see links below).

-   [OBS-Studio](https://github.com/obsproject/obs-studio/)
-   [OBS-WEBSOCKET](https://github.com/Palakis/obs-websocket/)

> It supports monitoring streams on either NGINX-RTMP server or Node-Media-Server. Node-Media-Server is also built into NOALBS for an easy all-in-one streaming solution.

-   [NGINX-RTMP](https://github.com/arut/nginx-rtmp-module/)
-   [Node-Media-Server](https://github.com/illuspas/Node-Media-Server/)

## Installation from Source

-   `git clone <repository-url>` or download from [RELEASES](https://github.com/715209/nginx-obs-automatic-low-bitrate-switching/releases)
-   Change into the new directory.
-   `npm install --production`

If you already have NGINX-RTMP server
-   Replace your `nginx.conf` with the one given here.
-   Put `stat.xsl` in your nginx folder.

Otherwise here is a Windows version of NGINX+RTMP Server
- [Hosted on Github](https://github.com/715209/nginx-obs-automatic-low-bitrate-switching/raw/master/nginx/nginx_1.7.11.3_Gryphon_With_NOALBS.config_files.zip)
- Everything is ready to go inside this zip, just extract and click on the `nginx_start.bat` file to start NGINX, you can use `nginx_stop.bat` to stop NGINX. HTTP server runs on Port `80`, RTMP server runs on `1935`, if you need to edit the config file it's in the `/conf` folder, named `nginx.conf`.

## Config

Edit `config.json` to your own settings.

Here is an example config with comments (click to enlarge):

**_PLEASE NOTE CONFIG MAY NOT REPRESENT CURRENT CONFIG_**

![alt text](https://i.imgur.com/cVbz1bN.png "Configuration Comments (Click to Enlarge)")

-   Use https://twitchapps.com/tmi to get your oauth from Twitch for use with chat commands.
    > We recommend using your main Twitch BOT account for this, but if you do not have a Twitch Bot account just use your Main Twitch Account.

Also if you are going to be using your Twitch BOT Account please make sure it is an 'Editor' of your channel, see example;

Go to this URL: [Twitch User Roles](https://www.twitch.tv/dashboard/roles/), Find your bot on the list, add checkmark to 'Editor', done.

![alt text](https://i.imgur.com/yRlBe5U.png "Setting your bot as Editor")

## How to run from source

Run the node app by running: `npm start`. Then stream to `rtmp://IPHERE/publish/live`

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
> |     MODs     | !fix                                  | tries to fix the stream.                                                                                | !fix                 |
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

## Running with Node-Media-Server
### Using the inbuilt server
Defining a nodeMediaServer block in config.json will enable a fully functional node-media-server RTMP server to accept incoming streams:

```
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
    }
```

> The `nodeMediaServer` object is passed directly as the node-media-server configuration, [more details here](https://github.com/illuspas/Node-Media-Server#npm-version-recommended). It will also automatically fill out the rtmp server type and stats fields.

> Note: This is probably best for local connections and testing only unless you [enable authentication](https://github.com/illuspas/Node-Media-Server#authentication)

### Using an external server
Modify the RTMP section in config.json like this to connect to a node-media-server running externally:

```
    "rtmp": {
        "server": "node-media-server",
        "stats": "http://localhost:8000/api/streams",
        "application": "publish",
        "key": "live"
    }
```

## Help it won't change scenes

It will only change scenes when OBS is set on a scene that's in the config.  
(This is so that it wont change when you are on for example your 'intro' or 'locked-brb' scene)
