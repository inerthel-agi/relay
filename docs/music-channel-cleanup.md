# Music channel cleanup and Loop

Available in Relay 1.3.4 on Windows.

## Optionally protect a welcome message

1. Open **Music** in Relay and select the Discord music channel.
2. If you want to keep a welcome message, copy its link from Discord into
   **Protected welcome message (optional)**. A message ID also works. Relay verifies
   that the message exists in the selected channel.
3. Enable **Automatically clean music messages** and save the music settings.

The bot needs **View Channel**, **Read Message History**, and **Manage Messages**.
Cleanup is disabled by default. The protected welcome message is optional, including
when cleanup is enabled: leave the field empty to protect no welcome message. If you
provide a message, Relay verifies it in the selected channel and never deletes it.
Recreating the music channel disables cleanup and clears the old welcome message
reference. Save the music settings again to enable cleanup, with or without a
protected welcome message.

Requests disappear after processing. Search results disappear after a selection or
after 120 seconds without a selection. Error replies expire after 120 seconds.
Mode choices and confirmations are private Discord responses. Playback cards remain
while a track is queued, playing, or waiting to repeat. They disappear when removed
from the queue, skipped, cleared, or completed with Loop off.

## Clean existing history

1. Save the music settings.
2. Select **Preview channel cleanup**.
3. Review the count, then choose **Delete these messages** or **Cancel**.

The preview examines up to the latest 1,000 messages. It excludes the configured
protected welcome message, when present, and tracked active interactions. Confirmation uses that exact snapshot,
expires after 120 seconds, and cannot be reused. Messages arriving after the
preview are not added to it. Changing the channel or protected message invalidates
the preview. Deletion stops on the first Discord error; the panel reports the result.
Repeat the preview if older messages remain. No cleanup confirmation is posted to Discord.

## Repeat a track

The requester can toggle **Loop: OFF / ON** beside **Skip**. Loop starts disabled.
It repeats the selected full track, preview, or custom range without another YouTube
search. Each repetition receives a fresh playback ID and joins the shared scheduler
behind waiting work, while keeping the same Discord card.

Turning Loop off lets the current pass finish. If a repetition is still queued,
turning Loop off removes it. Skip removes the track immediately, including a queued
repetition. Stale or duplicated playback-end notifications cannot repeat an old track.

## Media and TTS channel retention

In **Overview → Input routing**, media and TTS each have a separate **Delete messages
after 24 hours** switch and an optional welcome message link or ID. Save routing to
apply them. Both switches default to off. An empty welcome field means every message
older than 24 hours is eligible; a supplied message is verified in that channel and
preserved. If that message cannot be verified later, cleanup pauses for that channel.

While the bot is connected, Relay checks history every five minutes, including messages
sent before Relay started. Each pass scans up to 1,000 messages per channel and resumes
older history on subsequent passes. Deletions are sequential; large backlogs can take
several passes. No message younger than 24 hours is deleted by these switches. Bot
permissions are the same as for music cleanup. Recreating a channel disables its
cleanup option and clears its old welcome message reference.

## Limits

- Cleanup runs while Relay is running. An interrupted cleanup can leave messages;
  use the history preview after restarting.
- The existing volume control applies to YouTube, media audio/video, and TTS.
- Discord permission failures are reported in Relay. They do not create a public
  cleanup message.
- Actual Discord deletion, permissions, and OBS playback should be checked in a
  disposable channel before enabling cleanup in a shared channel.
