

# nginx-obs-automatic-low-bitrate-switching

Simple app to automatically switch scenes based on the current bitrate on the nginx stats page.

## Prerequisities

- [Git](http://git-scm.com/)
- [Node.js](http://nodejs.org/) (with NPM)

>This script uses OBS plugin "obs-websocket" inconjuction with "OBS-Studio" and "NGINX-RTMP" (see links below). 
- [NGINX-RTMP](https://github.com/arut/nginx-rtmp-module/)
- [OBS-Studio](https://github.com/obsproject/obs-studio/)
- [OBS-WEBSOCKET](https://github.com/Palakis/obs-websocket/)

## Installation

- `git clone <repository-url>`
- Change into the new directory.
- `npm install --production`
- Replace your `nginx.conf` with the one given here.
- Put `stat.xsl` in your nginx folder.

## Config

Edit `config.json` to your own settings.

Here is an example config with comments:
![alt text](https://i.imgur.com/mnxD5Mu.png "Configuration Comments")

 - Use https://twitchapps.com/tmi to get your oauth from Twitch for use with chat commands.
> We recommend using your main Twitch BOT account for this, but if you do not have a Twitch Bot account just use your Main Twitch Account.

Also if you are going to be using your Twitch BOT Account please make sure it is an 'Editor' of your channel, see example;

Go to this URL: [Twitch User Roles](https://www.twitch.tv/dashboard/roles/), Find you bot on the list, add checkmark to 'Editor', done.

![alt text](https://i.imgur.com/yRlBe5U.png "Setting your bot as Editor")

## How to run

Run the node app by running: `npm start`. Then stream to `rtmp://IPHERE/publish/live`

## Chat Commands

This script gives you the option to enable some simple chat commands to help you manage your stream from your own Twitch chat, here is how to use them:
>Please note: Admins are all the users in the "adminUsers" array in the config, MODs (if enabled in the config) are all of your MODs, and Public (if enabled in the config) is anyone in your chat.

>| Role  | Command           | Description          | Example  |
>|:-------:| ---------------- |:-------------| :----------------------|
>| Admins       | !host (channelname) | hosts said channel, and stops streaming in OBS after 5 seconds (changeable in config). | !host 715209 |
>| Admins       | !unhost      | unhosts whoever you are currently hosting.      |   !unhost  |
>| Admins       | !raid (channelname) | raids said channel and stops streaming in OBS after 15 seconds (changeable in config).      |   !raid 715209  |
>| Admins		| !start | on-demand command to start streaming in OBS.      |    !start |
>| Admins       | !stop | on-demand command to stop streaming in OBS.      |    !stop |
>| Admins       | !switch (scene) | switches to the provided SCENE (case senstive).      |    !switch INTRO|
>| MODs			| !refresh | changes to the REFRESH scene for the set interval. | !refresh |
>| Public       | !bitrate | returns current BITRATE | !bitrate |
>| Public       | !info | returns current SCENE and BITRATE. | !info |

## Help I can't stream

Make sure the node app is running. It won't allow you to connect to the rtmp server without it.

## Help it won't change scenes

It will only change scenes when OBS is set on a scene that's in the config.  
(This is so that it wont change when you're on like your intro or locked-brb scene)
