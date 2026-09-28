# Relay v1.3.6 modules

Relay v1.3.6 keeps Messages, Media, and Music and adds Sounds and reactions. The panel exposes each area separately so a change in one module does not stop the others.

## Messages

Messages are visual notifications from the configured message channel. Select the notification currently shown and click **Pin** to keep it visible while you answer it. Click **Unpin** to remove it and resume queued notifications.

Pinning does not stop media, music, or reactions. Relay clears the pin when it restarts, so an old message cannot remain on screen after a restart.

## Media library

The library stores a local copy of an image, GIF, or video. Use **Import** for a file on the PC, or save an item from History. Search by name, rename an item, play it through the normal media queue, or delete the managed copy.

The original file can be moved or deleted after import. Relay keeps its own copy. The library accepts up to 1,000 items, with a 120-character name limit, images and GIFs under 20 MB, and videos under 50 MB. The local privacy rules still run before a file is saved.

## Music requests

The Music module shows waiting titles with their requester and position. **Up**, **Down**, and **Remove** apply only to waiting titles. The title currently playing keeps its position.

The default limit is three waiting requests per Discord member. Set **Maximum pending requests per member** to `0` to disable that limit, or choose a value from `1` to `10`. The current title does not count toward this setting.

Duplicate protection is enabled by default. Relay compares the canonical YouTube video ID of waiting requests, so choosing the same video again is refused even when its title text differs. Turn off **Reject duplicate pending music** when repeated requests are wanted. The limit and duplicate check are made while the request enters the queue, so simultaneous requests cannot bypass them.

## Sounds and reactions

Create a reaction by importing a local sound and giving it a name. You can attach an image or GIF already stored in the Media library. Enable the module and each reaction before allowing Discord members to use it.

Reactions are disabled by default. One reaction can play at a time, for at most 30 seconds. The initial settings are a 10-second global cooldown, a 30-second cooldown per member, and music reduced to 25% of its selected volume during playback. Relay restores the chosen music volume when the reaction ends, is stopped, or fails.

Sounds longer than 30 seconds open an excerpt chooser. Set the start and end in seconds, preview the selection, then choose **Trim and add**. Relay creates a new excerpt of up to 30 seconds and leaves the original file unchanged. Sounds at or below 30 seconds are imported directly and need no extra processing; FFmpeg is required when Relay cuts a longer sound.

Discord access uses named channel and role selectors. Add more channels or roles when needed, then refresh the lists to load names from the connected bot. `/relay reaction` lists enabled reactions only after the module is enabled. The local **Test** button plays the imported sound and optional visual in the panel; it bypasses Discord cooldowns and does not send an event to OBS or Discord.

Optionally set a **Protected instructions message** with an existing Discord message link or ID. This excludes the message from Relay cleanup. Leave it empty to protect no message. Other bots and manual Discord deletions are unaffected. Relay does not create or publish the message.

Enabling reactions leaves room for up to 14 enabled custom commands alongside Relay's default commands. Connect the reaction Browser Source or enable its Windows widget before using **Play**. Local tests work without a live output.

The **Relay Reactions** Browser Source uses `http://127.0.0.1:<port>/reactions`. Copy the displayed URL from Relay and keep the source on the same PC. The optional Windows reaction widget uses the same local route and its own position settings.

If Discord is offline when reaction settings are saved, Relay writes the settings locally and marks the slash-command schema for synchronization when the bot connects again.

## What survives a restart

The Media library and imported reaction sounds are stored in Relay's application data directory. The History list, waiting queues, active pins, and active reactions are held in memory and are cleared when Relay exits.
