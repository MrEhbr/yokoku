# Telegram

With a Telegram bot set up, Yokoku sends notifications to a chat with you, a
group or a channel, in place of [ntfy](./ntfy.md). Set
`notifications.service` to `telegram`, or pick **Telegram** under **Send
through** on the Settings page's **Notifications**.

## Connect

| Setting | |
|---|---|
| `notifications.telegram.token` | The bot's token |
| `notifications.telegram.chat_id` | The chat to write to; empty for no notifications |
| `notifications.telegram.url` | The Bot API: `https://api.telegram.org`, or the address of your own Bot API server |

1. Message [@BotFather](https://t.me/BotFather), send `/newbot` and follow
   its questions. It answers with the bot's token, like
   `123456789:AAF…`.
2. Open a chat with your new bot and send it any message: a bot may only
   write to someone who wrote to it first. For a group, add the bot to the
   group; for a channel, make it an administrator.
3. Open `https://api.telegram.org/bot<token>/getUpdates` in a browser, with
   the token in place of `<token>`, and copy `chat.id` from the message you
   sent. A group's id starts with `-`. A public channel can use its
   `@name` instead.

**Send test notification** under **Telegram** shows *Sent a test
notification to* the chat, or why it failed:

| Message | Cause |
|---|---|
| Set the bot token first | `notifications.telegram.token` is empty |
| Set the chat first | `notifications.telegram.chat_id` is empty |
| notification service unavailable | Telegram isn't reachable, or the address is wrong |
| notification service refused the notification: HTTP 401 Unauthorized: Unauthorized | The token is wrong |
| notification service refused the notification: HTTP 400 Bad Request: Bad Request: chat not found | The chat id is wrong, or you haven't messaged the bot yet |
| notification service refused the notification: HTTP 403 Forbidden: Forbidden: bot was blocked by the user | Unblock the bot in Telegram |

The test works with either service chosen; notifications only go through
the one under **Send through**.

## What Notifies

The same switches as for ntfy choose [what notifies](./ntfy.md#what-notifies).
Each notification's title is in bold. Imports that need review and failed
imports ring; the others arrive silently.
