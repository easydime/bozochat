# BozoChat

Send images and videos to your friends' screens via Discord!

## Architecture

```
┌─────────────────────────────────┐
│  Discord Bot + WebSocket Server  │  Node.js — src/bot/
└────────────────┬────────────────┘
                 │ WebSocket (ws://)
        ┌────────┴────────┐
        │  BozoChat Client │  ← src/client-rust/  (Rust, Windows)
        │  BozoChat Client │     system tray + transparent overlay
        │  BozoChat Client │     one per friend
        └─────────────────┘
```

## Quick Start

### 1. Server Setup (Admin)

1. **Create a Discord Bot**
   - Go to https://discord.com/developers/applications
   - Create a New Application → Bot tab → Add Bot
   - Enable **Message Content Intent**
   - Copy the bot token

2. **Configure the server**
   ```bash
   copy .env.example .env
   # Edit .env — set DISCORD_TOKEN=your_token_here
   ```

3. **Start the server**
   ```bash
   start-bot.bat
   # or: npm run bot
   ```

4. **Invite the bot to your Discord server**
   - The console will print an invite URL — open it and authorize the bot

### 2. Client Setup (Friends)

**Requirements:** Windows 10/11 with [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on Windows 11)

**Build the executable:**
```bash
cd src/client-rust
cargo build --release
# Output: src/client-rust/target/release/bozochat-client.exe
```

Share `bozochat-client.exe` with your friends — no installer needed, single file.

### 3. Configure the Client

1. Double-click `bozochat-client.exe` — it appears in the system tray
2. Right-click the tray icon → **Settings**
3. Enter the server URL: `ws://your-server-ip:3001`
4. (Optional) Set a username, overlay position, display monitor
5. Click **Save**

## Usage

### In Discord

```
/send [upload file] "Check this out!" 5
```

| Parameter | Description |
|-----------|-------------|
| `media`   | Image or video file (jpg, png, gif, webp, mp4, webm, mov) |
| `message` | Text to display alongside the media |
| `duration`| How long to show in seconds (default: 5) |

### What Happens

1. You run `/send` in Discord
2. Bot downloads the media and base64-encodes it
3. Bot broadcasts via WebSocket to all connected clients
4. Every connected client shows the overlay notification
5. Videos play to completion automatically, then the overlay closes

## Client Settings

| Setting | Description |
|---------|-------------|
| Server URL | WebSocket address of the bot server (`ws://ip:3001`) |
| User ID | Optional identifier shown in logs |
| Display Monitor | Which screen to show the overlay on (multi-monitor support) |
| Overlay Position | Top-left / Top-right / Bottom-left / Bottom-right / Center |
| Default Duration | Auto-close time for images/text (ms) |
| Auto Start | Launch with Windows |

## File Structure

```
bozochat/
├── src/
│   ├── bot/
│   │   └── index.js                  # Discord bot + WebSocket server
│   └── client-rust/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs               # Entry point, event loop
│           ├── overlay.rs            # Overlay window + notification queue
│           ├── settings.rs           # Settings window
│           ├── tray.rs               # System tray icon + menu
│           ├── config.rs             # Config (load/save ~/.bozochat/config.json)
│           └── websocket.rs          # WebSocket client (tokio)
│   └── client/renderer/
│       ├── overlay.html              # Transparent overlay UI (embedded in binary)
│       └── settings.html             # Settings UI (embedded in binary)
├── bozoicon.ico                      # App icon (embedded in binary)
├── .env.example                      # Server environment template
├── package.json                      # Bot dependencies
├── start-bot.bat                     # Start the bot server
└── README.md
```

## Building the Client

**Requirements:** Rust 1.75+, Windows 10/11

```bash
cd src/client-rust

# Debug build (shows console)
cargo build

# Release build (no console, optimized)
cargo build --release
```

The release binary at `target/release/bozochat-client.exe` is self-contained — the HTML/CSS/JS UI is embedded at compile time.

## Port Forwarding (Home Hosting)

To let friends outside your network connect:

1. Forward **port 3001** on your router to your PC's local IP
2. Find your public IP at https://whatismyipaddress.com/
3. Friends use `ws://your-public-ip:3001` as the server URL

## Troubleshooting

**Server won't start**
- Check `.env` exists with a valid `DISCORD_TOKEN`
- Verify port 3001 is not already in use
- Ensure the bot has Message Content Intent enabled

**Client won't connect**
- Confirm the server is running
- Use format `ws://ip:3001` (not `http://`)
- Test locally first: `ws://localhost:3001`
- Check Windows Firewall isn't blocking port 3001

**Overlay doesn't appear**
- Check the tray icon status (green = connected)
- Try **Test Notification** from the tray right-click menu
- If behind a fullscreen app: the overlay uses `HWND_TOPMOST` but DirectX exclusive fullscreen apps cannot be overlaid (Windows limitation)

**WebView2 missing**
- Download from https://developer.microsoft.com/en-us/microsoft-edge/webview2/
- On Windows 11 it is pre-installed

## Security

> This is designed for private use with trusted friends.

- All connected clients receive every notification broadcast
- No authentication is implemented — use on a trusted network or add a firewall rule
- Config is stored in plain JSON at `~/.bozochat/config.json`

## Credits

Built with:
- [Rust](https://www.rust-lang.org/) — client runtime
- [wry](https://github.com/tauri-apps/wry) — WebView2 wrapper
- [winit](https://github.com/rust-windowing/winit) — window management
- [tokio](https://tokio.rs/) — async runtime
- [Discord.js](https://discord.js.org/) — Discord bot
- [ws](https://github.com/websockets/ws) — WebSocket server

## License

MIT
