# ntfy

With [ntfy](https://ntfy.sh) set up, Yokoku sends notifications to a topic you
subscribe to in the ntfy app on your phone or desktop. It's optional: without
a topic, nothing is sent.

## Connect

| Setting | |
|---|---|
| `ntfy.url` | The server: `https://ntfy.sh`, or the address of your own |
| `ntfy.topic` | The topic to publish to; empty for no notifications |
| `ntfy.token` | An access token, for a topic that needs one |

On the public `ntfy.sh`, anyone who knows a topic's name can read it. Pick a
name that's hard to guess, like `yokoku-3f9c2a`, or reserve the topic with an
ntfy account and set its access token.

Subscribe to the same topic in the ntfy app, then **Send test notification**
on the Settings page's **Notifications** shows *Sent a test notification to*
the topic, or why it failed:

| Message | Cause |
|---|---|
| Set the topic first | `ntfy.topic` is empty |
| notification service unavailable | The server isn't reachable, or the address is wrong |
| notification service refused the notification: HTTP 403 Forbidden: forbidden | The topic needs an access token, or the token can't publish to it |
