# YouTube Data API setup for Relay

Relay music search needs a **Google Cloud API key** for [YouTube Data API v3](https://developers.google.com/youtube/v3).

Relay uses an **API key**, not OAuth. You do **not** need a client ID, client secret, or authorized redirect URI.

This document is English-only. The same steps appear in the Relay **Music** panel.

---

## 1. Create a Google Cloud project

1. Open the [Google Cloud Console](https://console.cloud.google.com/).
2. Sign in with a Google account.
3. Top bar → **Select a project** → **New project**.
4. Name it (for example `Relay Music`) → **Create**.
5. Select that project so the top bar shows its name. Every later step must stay in this project.

Useful links (with the correct project selected):

| Step | Link |
|---|---|
| API Library | https://console.cloud.google.com/apis/library |
| Credentials | https://console.cloud.google.com/apis/credentials |
| YouTube quotas | https://console.cloud.google.com/apis/api/youtube.googleapis.com/quotas |

---

## 2. Enable YouTube Data API v3

1. Open **APIs & Services** → **Library** (or the API Library link above).
2. Search for **YouTube Data API v3**.
3. Open it → **Enable**.
4. Wait until the console confirms the API is enabled.

Without this step, Relay will get `403` / `accessNotConfigured` errors.

---

## 3. Create and restrict the API key

1. Open **APIs & Services** → **Credentials**.
2. **+ Create credentials** → **API key** (not “OAuth client ID”).
3. Copy the key once, then open **Edit API key** / **Restrict key**.
4. Under **API restrictions**, choose **Restrict key** and select only **YouTube Data API v3**.
5. Under **Application restrictions**:
   - leave **None** for a simple home setup, or
   - use **IP addresses** if you have a stable public IP.
   - Do **not** use HTTP referrers: Relay calls Google from the desktop app, not from a website.
6. **Save**.

Treat the key like a password. Never commit it, paste it in Discord, or show it on stream.

---

## 4. Paste the key into Relay

1. Open Relay → **Music**.
2. Paste the key into **YouTube API key** and choose a **Music channel**.
3. Save. The key field clears afterward: it is stored in **Windows Credential Manager** and never shown again.
4. In the Discord music channel, type a search (for example `jennie mantra`), pick a result, then choose preview (~30 s) or full track.

Notes:

- Search prefers **relevance** (like youtube.com). Newest uploads only fill gaps afterward.
- Results shorter than **60 seconds** or longer than **5 minutes** are filtered out (cuts Shorts spam; keeps jukebox-length tracks).
- Titles with Shorts/fyp hashtag spam are dropped.
- Add **Relay Visual** from the Music / Overview pages (`http://localhost:<port>/obs/visual`, suggested 1920×1080) and enable **Control audio via OBS**. Use `localhost` (not `127.0.0.1`) so YouTube allows the embed. Jukebox video is included in that composite — remove any old separate `/youtube` source to avoid double audio.
- Also add **Relay Audio** (`http://127.0.0.1:<port>/obs/audio`) for Discord file audio + TTS voice.
- Size the Windows **Now Playing** card under Music → **Overlay OBS / Windows**.
- Stop with Discord **Skip**, the panel skip control, or your global skip shortcut.

---

## 5. Troubleshooting

| Symptom | Check |
|---|---|
| “YouTube is not configured” | Key saved on the Music page; restart Relay after saving if needed |
| `403` / accessNotConfigured | API enabled on the **same** project as the key |
| Quota errors | [YouTube Data API quotas](https://console.cloud.google.com/apis/api/youtube.googleapis.com/quotas) |
| Empty results | Query too vague, or only videos outside the 60s–5min jukebox window matched |
| OBS YouTube / Visual black or silent | Use `http://localhost:<port>/obs/visual` (not `127.0.0.1`); copy the URL from Overview or Music. Enable **Control audio via OBS**. Remove duplicate old `/youtube` sources. |
| Leaked key | Create a **new** key in Google Cloud, save it in Relay, delete the old key |

The free quota is usually enough for personal streaming.
