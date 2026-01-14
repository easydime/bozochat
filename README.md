# BozoChat

Send images and videos to your friends' screens via Discord!

## Architecture

```
┌─────────────────┐         ┌─────────────────┐
│  Discord Bot    │◄────────┤   Your Server   │
│  + WebSocket    │         │   (Node.js)     │
│     Server      │         └─────────────────┘
└────────┬────────┘
         │
         │ WebSocket
         │
    ┌────▼─────┐
    │  Client  │
    │  Client  │  ← Windows app with system tray
    │  Client  │     (one per friend)
    └──────────┘
```

## Quick Start

### 1. Server Setup (You - Admin)

1. **Create Discord Bot**
   - Go to https://discord.com/developers/applications
   - Create "New Application"
   - Go to "Bot" tab → "Add Bot"
   - Enable "Message Content Intent"
   - Copy the bot token

2. **Configure Server**
   ```bash
   # Copy environment template
   copy .env.example .env

   # Edit .env and add your bot token
   # DISCORD_TOKEN=your_token_here
   ```

3. **Start Server**
   ```bash
   # Double-click or run:
   start-bot.bat
   ```

4. **Invite Bot to Discord**
   - The server will show an invite URL in the console
   - Open it and add the bot to your server

### 2. Client Setup (Your Friends)

**Option A: Development Mode**
```bash
start-client-dev.bat
```

**Option B: Build Executable**
```bash
# Build the .exe installer
build-client.bat

# Share the installer from dist/ folder with friends
# They just double-click and install!
```

### 3. Configure Client

1. Right-click the BozoChat icon in system tray
2. Click "Settings"
3. Enter server URL (e.g., `ws://your-ip:3001`)
4. (Optional) Set a username
5. Click "Save"

## Usage

### In Discord

```
/send [upload image] "Check this out!" 5
```

Parameters:
- **media**: Image or video file (drag & drop)
- **message**: Text to display on screen
- **duration**: How long to show (seconds, default 5)

### What Happens

1. You use `/send` in Discord
2. Bot receives the command
3. Bot downloads the media
4. Bot broadcasts to all connected clients via WebSocket
5. **BOOM!** Everyone sees your image/video with message

## Configuration

### Server (.env)
```env
DISCORD_TOKEN=your_bot_token_here
PORT=3001
```

### Client (Settings Window)
- **Server URL**: WebSocket address (ws://server-ip:3001)
- **User ID**: Optional identifier
- **Overlay Position**: Where notifications appear
- **Default Duration**: Auto-close time
- **Auto Start**: Launch with Windows

## File Structure

```
bozochat/
├── src/
│   ├── bot/
│   │   └── index.js              # Discord bot + WebSocket server
│   └── client/
│       ├── main.js               # Electron main process (system tray)
│       └── renderer/
│           ├── overlay.html      # Notification overlay
│           └── settings.html     # Settings window
├── .env.example                  # Environment template
├── package.json                  # Dependencies & build config
├── start-bot.bat                 # Start server
├── start-client-dev.bat          # Start client (dev mode)
└── build-client.bat              # Build .exe installer
```

## Building for Production

### Requirements
- Node.js 16+
- npm

### Build Steps

1. **Install Dependencies**
   ```bash
   npm install
   ```

2. **Build Client Executable**
   ```bash
   npm run build:win
   ```

3. **Share with Friends**
   - The installer is in `dist/BozoChat Setup X.X.X.exe`
   - Send it to your friends
   - They install and connect to your server

## Port Forwarding (If Hosting from Home)

To let friends outside your network connect:

1. **Forward Port 3001** on your router
   - Login to router (usually 192.168.1.1)
   - Find "Port Forwarding" settings
   - Forward port 3001 to your computer's local IP

2. **Find Your Public IP**
   - Visit https://whatismyipaddress.com/

3. **Share with Friends**
   - They use `ws://your-public-ip:3001` as server URL

## Troubleshooting

### Server won't start
- Check `.env` file exists with valid Discord token
- Verify port 3001 is not in use
- Check bot has required Discord permissions

### Client won't connect
- Verify server is running first
- Check server URL format: `ws://ip:3001` (not http/https)
- Test with localhost first: `ws://localhost:3001`
- Check firewall isn't blocking port 3001

### Overlay doesn't show
- Check system tray icon is connected (green dot)
- Try "Test Notification" from tray menu
- Verify overlay position in settings
- Check if overlay is behind fullscreen apps

### Build fails
- Run `npm install` first
- Check Node.js version (16+)
- Delete `node_modules` and reinstall
- Check disk space for build output

## Security Notes

⚠️ **This is for private use with trusted friends!**

- Anyone connected can receive ANY media you send
- The bot has full access to send to all clients
- Consider adding authentication for public use
- Be careful with what you send!

## Advanced: Custom Icon

1. Create a 256x256 PNG icon
2. Convert to .ico using online tool
3. Save as `build/icon.ico`
4. Rebuild with `npm run build:win`

## Credits

Built with:
- [Electron](https://www.electronjs.org/) - Desktop app framework
- [Discord.js](https://discord.js.org/) - Discord bot library
- [ws](https://github.com/websockets/ws) - WebSocket library

## License

MIT - Do whatever you want with it!

---

