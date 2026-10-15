# osu-echo

A local osu! server written in Rust. It lets you run your own private server on your computer for score saving, performance points (PP) calculation, local leaderboards, and beatmap downloads.

## Features

- **Score & Replay Tracking**: Automatically saves your plays, scores, and `.osr` replay files locally in a self-contained SQLite database.
- **Real-Time PP Calculation**: Computes live PP for all plays using `rosu-pp` across all game modes (Standard, Taiko, Catch, Mania).
- **Custom & Unranked Maps**: Full support for unranked maps, practice diffs, custom speed rates, and osu!trainer modifications.
- **Built-in osu!direct Mirror**: In-game beatmap search and 1-click downloads powered by the Catboy (Mino) mirror (no osu! supporter or external accounts required).
- **In-Game Bot Companions**:
  - **BanchoBot**: Manages leaderboard statuses, personal bests, score announcements, and profile statistics.
  - **Tillerino**: Send `/np` in chat or PM to get instant difficulty breakdowns and PP calculations for Nomod, HD, HR, DT, etc.
- **Local Leaderboards & Profiles**: Track personal bests, total score, hit accuracy, play count, and global rank estimates.
- **Discord Webhook Integration**: Automatically post submitted plays with Rich Embeds and configurable minimum PP thresholds directly to a Discord channel.
- **Web-Based Setup & Admin Panel**: First-run setup is a guided popup on the local admin page, with auto-detection of your osu! folder. Manage users, maps, API keys, settings and the website theme from the same page.
- **Official Profile Import**: Import your avatar, cover, profile details, scores and (optionally) rank history from your official osu! profile.
- **Exact PP**: pp is stored with 4 decimals internally and shown rounded on the site; hover the value to see the exact figure.
- **Local Profile Website**: A local osu!web-style profile site with four selectable themes (modern, 2019, 2017, 2016).
- **Automatic Windows Setup**: Adds the required hosts-file entries and trusts the local HTTPS certificate for you.

## Requirements

- **osu! client (Stable)**
- *Optional (only needed if compiling from source)*: **Rust (Cargo)**

## Getting Started

### 1. Get the Server

#### Option A: Download Pre-built Release (Recommended)
1. Download the latest `osu-echo-vX.X.X-windows-x86_64.zip` from [Releases](https://github.com/ptrchain/local-bancho-rust/releases).
2. Extract the zip into a folder.
3. Run `osu-echo.exe`.

#### Option B: Build from Source
```bash
git clone https://github.com/ptrchain/local-bancho-rust.git
cd local-bancho-rust
cargo run --release
```

### 2. First Launch & Setup

On Windows, the first launch asks for administrator permission to add the `osu.*` entries to your hosts file and trust the local HTTPS certificate. Accept the prompt, or start `osu-echo.exe` as administrator. If the website does not load afterwards, reload the page with **CTRL + F5, or CTRL + R**.

The server then creates its `.env` and `.data/` folder, starts, and opens the admin page in your browser (`http://osu.localhost:5000/admin`, or `http://localhost:5000/admin` if the hosts file is not ready yet). A guided setup walks you through:

1. **Account**: create your local account, or import your official osu! profile (avatar, cover, details, scores and optional rank history).
2. **API keys**: enter your osu! v1 key, osu! v2 client ID/secret and osu!daily key. Each key has a **Test** button, and each card links to where the key is obtained.
3. **Settings**: osu! folder (auto-detected), leaderboard options, Discord and server options under the **Other** tab.

> [!NOTE]
> **Privacy & Local Storage**: All API keys and account credentials entered during setup or configured in `.env` are stored strictly locally on your machine and are never transmitted to any third-party or remote server, apart from the requests needed to call the services the keys belong to.

Existing installs never show the guide again. To run it again, start the server with the `--setup` (or `-s`) flag:
```bash
osu-echo.exe --setup
```
You can also edit `.env` manually at any time, or change settings from the admin page's **Other** tab. Values set in `.env` take priority over values saved from the admin page.

### 3. Connect from osu!

To connect your osu! client to the local server:

1. Right-click your `osu!.exe` shortcut and select **Properties**.
2. In the **Target** field, add `-devserver localhost` to the end.
   Example:
   ```text
   "C:\Games\osu!\osu!.exe" -devserver localhost
   ```
3. Launch osu! using the shortcut.
4. Log in with any username and password. The server will create your profile automatically.

The admin page is only reachable from your own computer, at `osu.localhost:5000/admin`.

## In-Game Commands

You can send commands in chat channels or via private message to **BanchoBot** or **Tillerino**.

### BanchoBot Commands

| Command | Description |
|---|---|
| `!help` | List available commands |
| `!stats` / `!profile` | Show your performance stats, rank, and play count |
| `!recent` / `!r` | Show your most recent play with PP and accuracy |
| `!tops` / `!t` | Show your top 5 highest PP plays |
| `!mybest` / `!pb` | Show your best score on the current beatmap |
| `!leaderboard` / `!lb` | Show top local scores on the current beatmap |
| `!mode <std/taiko/ctb/mania>` | Switch active game mode |
| `!country <code>` | Set country flag on your profile (e.g. `!country US`, `!country DE`) |
| `!status <ranked/unranked/loved>` | Change the status of the current beatmap |
| `!friend <add/remove/list/sync>` | Manage friends or sync with official Bancho |
| `!recentfeed <on/off>` | Toggle live score announcements in the `#recent` channel |
| `!recalc` / `!recalculate` | Recalculate profile PP and stats from stored scores |
| `!restrictself [reason]` / `!unrestrict` | Simulate official Bancho ban / account restriction |
| `!avatar <url or path>` | Change your in-game profile avatar |
| `!config` | Show server settings and active configuration |
| `!roll [max]` | Roll a random number (default 1-100) |
| `!wipe` | Wipe profile stats and play count |
| `!clearscores` / `!clearmap` | Clear your scores on the current beatmap or /np selection |

### Tillerino Commands

Send commands via PM to **Tillerino** or type `/np` in any channel:

| Command | Description |
|---|---|
| `/np` | Show current song info, star rating, and 95%–100% PP breakdown |
| `!with <mods>` | Calculate PP with specific mods (e.g. `!with HDHR`, `!with DT`) |
| `!acc <value>` | Calculate exact PP for a custom accuracy (e.g. `!acc 98.5`) |
| `!r` / `!recommend` | Recommend a beatmap to play |

## Admin Panel & Website

Open `http://osu.localhost:5000/admin` while the server is running.

- **Stats, Users, Maps**: overview, create/edit/delete users, change map status.
- **Keys**: save and test your API keys.
- **Console**: view and clear the server log.
- **Other**: change the website theme (modern, 2019, 2017, 2016), edit server settings, and install/repair the Windows certificate and hosts entries.

The profile site is at `http://osu.localhost:5000/`. Profile pp is shown rounded; hover it to see the exact value. Official ranks are looked up with the exact pp.

## Command-Line Options

```text
Usage: osu-echo [OPTIONS]

Options:
  -h, --help           Print help information
  -v, --version        Print version information
  -d, --debug          Enable verbose debug logging (or hold Shift when starting)
  -s, --setup          Re-run the first-run setup guide on the admin page
      --reconfigure    Alias for --setup
      --trust-cert     Install and trust the local TLS certificate in the Windows Root store
```

## Data & Backups

All server data is stored locally in the `.data/` directory next to the executable:
- `server.db`: SQLite database holding user profiles, scores, beatmap cache, and stats.
- `replays/`: Saved `.osr` replay files.
- `avatars/`: User avatars.

To back up your progress or move to another machine, simply copy the `.data/` folder.

## Credits & Acknowledgements

Special thanks to:
- [local-osu-server](https://github.com/jeevanjohnson/local-osu-server) by [Jeevan Johnson](https://github.com/jeevanjohnson), [ptrchain's fork](https://github.com/ptrchain/osu-echo) and its contributors for the original inspiration and foundational work that helped make this project possible.

## License

This project is licensed under the MIT License.
unofficial / not affiliated with ppy
