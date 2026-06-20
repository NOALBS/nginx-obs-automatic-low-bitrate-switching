| Default Role | Command                   | Description                                                                                             | Example               |
| :----------: | ------------------------- | :------------------------------------------------------------------------------------------------------ | :-------------------- |
|    Admins    | !start                    | on-demand command to start streaming in OBS.                                                            | !start                |
|    Admins    | !stop                     | on-demand command to stop streaming in OBS.                                                             | !stop                 |
|    Admins    | !record                   | on-demand command to toggle recording in OBS.                                                           | !record               |
|    Admins    | !collection (name)        | changes the scene collection and profile.                                                               | !collection twitch    |
|    Admins    | !alias (alias) (command)  | add an alias for a command.                                                                             | !alias ss switch      |
|    Admins    | !alias rem (alias)        | removes an alias for a command.                                                                         | !alias rem ss         |
|    Admins    | !switch (scene)           | switches to the provided SCENE ([fuzzy match](https://wikipedia.org/wiki/Approximate_string_matching)). | !switch INTRO         |
|    Admins    | !source (value)           | Toggles an OBS source item visibility on the current scene                                              | !source media         |
|    Admins    | !live                     | switch to the live scene.                                                                               | !live                 |
|    Admins    | !privacy                  | switch to the privacy scene.                                                                            | !privacy              |
|    Admins    | !starting                 | switch to the starting scene.                                                                           | !starting             |
|    Admins    | !ending                   | switch to the ending scene.                                                                             | !ending               |
|    Admins    | !streamserver (name)      | Toggles the stream server on or off.                                                                    | !streamserver belabox |
|    Admins    | !noalbs prefix (prefix)   | change noalbs command prefix.                                                                           | !noalbs prefix #      |
|    Admins    | !noalbs retry (value)     | changes the retry value for the switcher.                                                               | !noalbs retry 5       |
|    Admins    | !noalbs lang (value)      | changes the chat response language.                                                                     | !noalbs lang zh_tw    |
|    Admins    | !noalbs ignore (username) | adds a user to the ignore list.                                                                         | !noalbs ignore 715209 |
|    Admins    | !noalbs rem (username)    | removes a user from the ignore list.                                                                    | !noalbs rem 715209    |
|     MODs     | !trigger (value)          | changes the low bitrate threshold to the defined value.                                                 | !trigger 800          |
|     MODs     | !otrigger (value)         | changes the offline bitrate threshold to the defined value.                                             | !otrigger 200         |
|     MODs     | !rtrigger (value)         | changes the RTT based low threshold to the defined value.                                               | !rtrigger 1500        |
|     MODs     | !ortrigger (value)        | changes the RTT based offline threshold to the defined value.                                           | !ortrigger 2000       |
|     MODs     | !sourceinfo               | gives you details about the SOURCE in chat.                                                             | !sourceinfo           |
|     MODs     | !serverinfo               | gives you details about the SERVER in chat.                                                             | !serverinfo           |
|     MODs     | !fix                      | tries to fix the stream.                                                                                | !fix                  |
|     MODs     | !refresh                  | tries to fix the stream.                                                                                | !refresh              |
|    Public    | !bitrate                  | returns the current bitrate.                                                                            | !bitrate              |

You can also enable/disable certain features from chat, see below:

| Default Role | Command              | Description                                                | Example         |
| :----------: | -------------------- | :--------------------------------------------------------- | :-------------- |
|    Admins    | !public (on/off)     | enables/disables the use of Public commands.               | !public off     |
|    Admins    | !mod (on/off)        | enables/disables the use of MOD commands.                  | !mod on         |
|    Admins    | !notify (on/off)     | enables/disables the notifications in chat.                | !notify off     |
|    Admins    | !autostop (on/off)   | enables/disables the auto stop feature when you host/raid. | !autostop on    |
|    Admins    | !noalbs (start/stop) | NOALBS start/stop switching scenes.                        | !noalbs stop    |
|    Admins    | !noalbs instant      | toggle instant switching from offline scene.               | !noalbs instant |
