# BozoChat Quick Start Guide

## For the Admin (You)

### Step 1: Get Discord Bot Token

1. Go to https://discord.com/developers/applications
2. Click "New Application" → Name it "BozoChat"
3. Go to "Bot" tab
4. Click "Add Bot"
5. Under "Privileged Gateway Intents":
   - ✅ Enable "Message Content Intent"
6. Click "Reset Token" → Copy the token
7. Save it somewhere safe!

### Step 2: Setup Server

1. Copy `.env.example` to `.env`
2. Open `.env` in notepad
3. Paste your token:
   ```
   DISCORD_TOKEN=your_token_here
   PORT=3001
   ```
4. Save and close

### Step 3: Start Server

Double-click `start-bot.bat`

You should see:
```
✓ BozoChat server started on port 3001
✓ Discord bot connected: BozoChat#1234
✓ 2 commands registered successfully
🤖 Bot ready! Invite URL:
https://discord.com/api/oauth2/authorize?client_id=...
```

### Step 4: Invite Bot to Discord

1. Copy the invite URL from the console
2. Paste in browser
3. Select your server
4. Authorize

### Step 5: Test Server

In Discord type: `/status`

The bot should respond with connected clients (probably 0 for now).

---

## For Your Friends (Clients)

### Option A: Use Development Mode (For Testing)

1. Copy the entire `bozochat` folder to their PC
2. Double-click `start-client-dev.bat`
3. System tray icon appears
4. Right-click icon → "Settings"
5. Enter server URL: `ws://YOUR_IP:3001`
6. Click "Save"

### Option B: Build & Share Installer (Recommended)

**On your PC:**

1. Double-click `build-client.bat`
2. Wait for build to complete
3. Go to `dist/` folder
4. Find `BozoChat Setup 1.0.0.exe`
5. Share this file with friends (Discord, Google Drive, etc.)

**On friend's PC:**

1. Download the installer
2. Double-click to install
3. Launch BozoChat
4. System tray icon appears
5. Right-click icon → "Settings"
6. Enter server URL: `ws://ADMIN_IP:3001`
7. Enter a username (optional)
8. Click "Save"

---

## Usage

### Send a Notification

In Discord:
```
/send
```

Fill in:
- **media**: Click to upload image/video
- **message**: "Hey everyone, check this out!"
- **duration**: 5 (seconds)

Press Enter → **BOOM!** It appears on everyone's screen!

### Check Status

```
/status
```

Shows how many clients are connected.

---

## Finding Your IP Address

### Local Network Only (Same WiFi)

**Windows:**
1. Press `Win + R`
2. Type `cmd` and press Enter
3. Type `ipconfig`
4. Look for "IPv4 Address" (usually `192.168.x.x`)
5. Share this with friends: `ws://192.168.x.x:3001`

### Over Internet (Different Networks)

**You need to port forward:**

1. **Find your local IP** (see above)

2. **Login to router** (usually `192.168.1.1` or `192.168.0.1`)

3. **Find Port Forwarding settings**
   - Might be called "Virtual Server" or "NAT Forwarding"

4. **Add new rule:**
   - External Port: `3001`
   - Internal Port: `3001`
   - Internal IP: Your local IP from step 1
   - Protocol: TCP

5. **Find your public IP:**
   - Go to https://whatismyipaddress.com/
   - Copy the IP shown

6. **Share with friends:** `ws://YOUR_PUBLIC_IP:3001`

---

## Testing

### Test 1: Server Running?

In Discord: `/status`

Should show bot is online.

### Test 2: Client Connected?

On client:
1. Right-click tray icon
2. Menu should show "(Connected)"
3. Click "Test Notification"
4. Should see test message on screen

### Test 3: Send Real Notification

In Discord:
```
/send [upload image] "Testing!" 3
```

Should appear on all connected clients!

---

## Common Issues

### "No clients connected"

- Clients need to be running and showing "Connected" in tray
- Check server URL in client settings
- Try `ws://localhost:3001` on same PC as server

### "Can't connect to server"

- Is the server running?
- Is the URL correct? Must start with `ws://`
- Firewall blocking port 3001?
- Try turning off Windows Firewall temporarily to test

### "Bot not responding in Discord"

- Check `.env` has correct token
- Bot invited to server?
- Server console showing errors?
- Restart `start-bot.bat`

### "Overlay doesn't appear"

- Is client connected? (check tray icon)
- Try "Test Notification" from tray menu
- Check overlay position in settings
- May be behind fullscreen apps

---

## Tips

- Keep the server running on one PC (or use a VPS)
- Clients can be on/off anytime
- `/send` works even if only 1 client connected
- Duration: 1-30 seconds
- Supports: JPG, PNG, GIF, MP4, WEBM
- Click overlay or press ESC to dismiss early

---

**You're all set! Have fun!** 🎉
