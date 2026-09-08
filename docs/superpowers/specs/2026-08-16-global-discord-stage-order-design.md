# Global Discord Stage Order

## Goal

Every user-generated output shown by Relay must follow one global Discord chronology across OBS and Windows. Preparation speed must never decide priority.

## Ordering

The total order is:

1. Discord creation timestamp.
2. Discord message snowflake.
3. Content position inside the message:
   - text/TTS;
   - stickers in Discord order;
   - attachments in Discord order;
   - delayed embeds in embed order.
4. A server insertion sequence only for events without a Discord snowflake.

Example: while a video created at 21:59 is active, text created at 22:00 and an image created at 22:01 arrive. The video finishes normally, then the text is shown, then the image.

## Scheduler behavior

- Register a placeholder as soon as Discord input is accepted, before TTS synthesis, privacy scanning, downloads, caching, transcoding, or YouTube lookup.
- Mark the placeholder ready only when its payload is safe and displayable.
- Never interrupt active content.
- Dispatch only the oldest eligible placeholder when the shared stage becomes idle.
- Reserve the chosen lane before broadcasting so media, TTS, stickers, and music cannot race for the free stage.
- Wait until every connected client displaying the logical item releases its lane before dispatching the next item.
- Keep the current WebSocket event payloads and frontend queues compatible where possible.

## Slow and failed preparation

- A non-ready head may block strict order for at most 20 seconds.
- After 20 seconds, demote it behind currently ready items instead of deleting it.
- If it later becomes ready, reinsert it using the readiness time while preserving its intra-message order.
- A failed, denied, or deleted item is cancelled immediately.
- Content awaiting manual moderation does not block the live stage; approval creates a new ticket at approval time.
- A dispatched item not claimed by any output within 3 seconds is completed so disconnected outputs cannot freeze the queue.

## Controls

- `Clear`: cancel every pending ticket and clear the active output.
- `Skip`: end the active ticket and dispatch the next eligible ticket.
- `Replay`: enqueue at the replay request time without interrupting active content.
- Music: use the Discord selection/confirmation time; queued tracks participate in the same chronology and do not bypass older text or media.
- Output tests remain isolated from the user chronology.

## Delayed Discord embeds

When Discord reveals an embed after the original message event, insert it with the original Discord timestamp. It may become next after the active item, but it cannot retroactively preempt content already shown.

## Acceptance criteria

- Active content is never preempted by a newer item.
- Text 22:00 is displayed before image 22:01 even if the image prepares first.
- Equal timestamps use snowflake and intra-message order deterministically.
- A failed or permanently slow item cannot freeze the queue.
- OBS and Windows show the same logical order.
- Clear, skip, replay, music, reconnect, and disconnected outputs cannot strand the scheduler.
- Existing media history remains complete and replayable.
