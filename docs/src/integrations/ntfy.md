# ntfy

With [ntfy](https://ntfy.sh) set up, Yokoku sends notifications to a topic you
subscribe to in the ntfy app on your phone or desktop. It's optional: without
a topic, nothing is sent.

## Connect

| Setting | |
|---|---|
| `notifications.ntfy.url` | The server: `https://ntfy.sh`, or the address of your own |
| `notifications.ntfy.topic` | The topic to publish to; empty for no notifications |
| `notifications.ntfy.token` | An access token, for a topic that needs one |

On the public `ntfy.sh`, anyone who knows a topic's name can read it. Pick a
name that's hard to guess, like `yokoku-3f9c2a`, or reserve the topic with an
ntfy account and set its access token.

Subscribe to the same topic in the ntfy app, then **Send test notification**
on the Settings page's **Notifications** shows *Sent a test notification to*
the topic, or why it failed:

| Message | Cause |
|---|---|
| Set the topic first | `notifications.ntfy.topic` is empty |
| notification service unavailable | The server isn't reachable, or the address is wrong |
| notification service refused the notification: HTTP 403 Forbidden: forbidden | The topic needs an access token, or the token can't publish to it |

## What Notifies

Each kind has a switch under **Notify about**, all on at first:

| Switch | Notifies when | Example |
|---|---|---|
| `notifications.events.imported` | Files are placed in the library | **Frieren (2023)**: Imported S01E05 |
| `notifications.events.needs_review` | An import waits for you in the **Queue** | **Import needs review**: Frieren.S01E05.mkv |
| `notifications.events.failed` | An import failed | **Import failed**: Frieren.S01E05.mkv: no space left on device |
| `notifications.events.grabbed` | A torrent is added to Transmission | **Frieren (2023)**: Grabbed Frieren S01 1080p |
| `notifications.events.downloaded` | A torrent finished downloading | **Frieren (2023)**: Downloaded Frieren S01 1080p |

Imports that need review and failed imports ask for attention with ntfy's
high priority.

Notifications of one kind about one series or movie wait until 30 s pass
without another, then go out as one: a season imported at once, or episodes
grabbed one after another, give one notification listing them, like
*Imported 12 files*. A notification lists up to ten lines and counts the
rest. Those not sent yet are lost when Yokoku stops.

When ntfy can't be reached, a notification is tried again on the next runs of
the [`send_notifications`](../configuration/schedules.md) schedule, then
dropped after five tries.
