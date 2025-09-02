## **Enable only replies in your own chat during shared chats & activate the Twitch chat badge, please follow these steps:**
_Create a Twitch app:_

Go to:
> [https://dev.twitch.tv/console/extensions/create](https://dev.twitch.tv/console/extensions/create)

_In the app settings, set OAuth Redirect URLs to:_

`https://twitchtokengenerator.com`

<img width="1013" height="578" alt="Image" src="https://github.com/user-attachments/assets/64068ec1-c2d4-4311-a058-d4347015d575" />

_Copy credentials into .env:_
**From your Twitch app, copy the Client ID and Client Secret, and place them into your .env file:**
```
TWITCH_APP_CLIENT_ID=your_client_id_here
TWITCH_APP_CLIENT_SECRET=your_client_secret_here
```


**Generate an Access Token:**
Go to [https://twitchtokengenerator.com/](https://twitchtokengenerator.com/)

_Enter your Client ID and Client Secret from the Twitch app._

<img width="1276" height="309" alt="Image" src="https://github.com/user-attachments/assets/4ca7b6dd-207e-46ea-91fe-20da04cd757d" />

**_Under Token Scopes, either manually set the following to YES:_**
```
chat:read
chat:edit
moderation:read
channel:manage:raids
moderator:read:chatters
channel:bot
user:bot
user:read:chat
user:write:chat
```
### **_Or simply click “Select All” (recommended)._**

**Finally, press “Generate Token!”**

_Update .env with the token_

> Copy the generated **_Access Token_** and insert it into your .env file:
`TWITCH_BOT_OAUTH=your_generated_access_token`
