

# nginx-obs-automatic-low-bitrate-switching

Simple app to automatically switch scenes based on the current bitrate on the nginx stats page.

Uses OBS plugin <a href="https://github.com/Palakis/obs-websocket">obs-websocket</a>.

## Prerequisities

- [Git](http://git-scm.com/)
- [Node.js](http://nodejs.org/) (with NPM)

## Installation

- `git clone <repository-url>`
- Change into the new directory.
- `npm install --production`
- Replace your `nginx.conf` with the one given here.
- Put `stat.xsl` in your nginx folder.

## Config

Edit `config.json` to your own settings.
 - Use https://twitchapps.com/tmi to get your oauth from Twitch for use with chat commands.
> We recommend using your main Twitch BOT account for this, but if you do not have a Twitch Bot account just use your Main Twitch Account.

Also if you are going to be using your Twitch BOT Account please make sure it is an 'Editor' of your channel, see example;

Go to this URL: [Twitch User Roles](https://www.twitch.tv/dashboard/roles/), Find you bot on the list, add checkmark to 'Editor', done.

![alt text](https://i.imgur.com/yRlBe5U.png "Setting your bot as Editor")

## How to run

Run the node app by running: `npm start`. Then stream to `rtmp://IPHERE/publish/live`

## Chat Commands

The script monitors twitch chat and features some simple chat commands to help you manage your stream from your own Twitch chat, here is how to use them:

>| Prefix  | Command        | Description          | Example  |
>|:-------:| ------------- |:-------------| :----------------------|
>| !       | host (channelname) | hosts said channel, and stops streaming in OBS. | !host 715209 |
>| !       | unhost      | unhosts and starts streaming in OBS.      |   !unhost  |
>| !       | start | on-demand command to start streaming in OBS.      |    !start |
>| !       | stop | on-demand command to stop streaming in OBS.      |    !stop |
>| !       | switch (scene) | switches to the provided scene (case senstive).      |    !switch INTRO|

## Help I can't stream

Make sure the node app is running. It won't allow you to connect to the rtmp server without it.

## Help it won't change scenes

It will only change scenes when OBS is set on a scene that's in the config.  
(This is so that it wont change when you're on like your intro or locked-brb scene)
