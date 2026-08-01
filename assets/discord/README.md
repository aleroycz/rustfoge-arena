# Discord Rich Presence assets

Discord doesn't have a public API for uploading Rich Presence art -- it has
to be done by hand, once, through the Developer Portal. The game code
already references the asset *keys* below; you just need to upload the
matching file under each key so Discord can serve them.

1. Go to https://discord.com/developers/applications/1532447557668704336/rich-presence/assets
   (application ID `1532447557668704336`, already wired into
   `games/arena-client/src/discord.rs`).
2. Click "Add Image(s)", upload each file from this folder, and set its
   asset key to the name below (exact match, no extension):

| File | Asset key |
|---|---|
| `logo_large.png` | `logo_large` |
| `status_menu.png` | `status_menu` |
| `status_lobby.png` | `status_lobby` |
| `status_playing.png` | `status_playing` |
| `status_victory.png` | `status_victory` |

3. Save. Changes usually propagate within a few minutes.

Until these are uploaded, Discord just shows the presence text with no
images -- the game itself doesn't need any code change either way.

Regenerate all five any time with `python scripts/generate_discord_assets.py`
(uses Pillow, no other dependencies).
