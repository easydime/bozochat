const { app, BrowserWindow, Tray, Menu, ipcMain, screen, nativeImage } = require('electron');
const path = require('path');
const WebSocket = require('ws');
const Store = require('electron-store');

const store = new Store();
let tray = null;
let overlayWindow = null;
let settingsWindow = null;
let ws = null;
let reconnectInterval = null;

// Default configuration
const DEFAULT_CONFIG = {
  serverUrl: 'ws://localhost:3001',
  userId: '',
  overlayPosition: 'center',
  defaultDuration: 5000,
  autoStart: false
};

function getConfig() {
  return { ...DEFAULT_CONFIG, ...store.store };
}

function saveConfig(config) {
  store.set(config);
}

// Create transparent overlay window
function createOverlayWindow() {
  const { width, height } = screen.getPrimaryDisplay().workAreaSize;

  overlayWindow = new BrowserWindow({
    width: 700,
    height: 500,
    transparent: true,
    frame: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: false,
    show: false,
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false
    }
  });

  overlayWindow.setIgnoreMouseEvents(true);
  overlayWindow.loadFile(path.join(__dirname, 'renderer', 'overlay.html'));

  // Position overlay
  const config = getConfig();
  positionOverlay(config.overlayPosition);

  overlayWindow.on('closed', () => {
    overlayWindow = null;
  });
}

// Position overlay on screen
function positionOverlay(position) {
  if (!overlayWindow) return;

  const { width: screenWidth, height: screenHeight } = screen.getPrimaryDisplay().workAreaSize;
  const windowWidth = 700;
  const windowHeight = 500;
  const margin = 40;

  let x, y;
  switch (position) {
    case 'top-left':
      x = margin;
      y = margin;
      break;
    case 'top-right':
      x = screenWidth - windowWidth - margin;
      y = margin;
      break;
    case 'bottom-left':
      x = margin;
      y = screenHeight - windowHeight - margin;
      break;
    case 'bottom-right':
      x = screenWidth - windowWidth - margin;
      y = screenHeight - windowHeight - margin;
      break;
    case 'center':
    default:
      x = (screenWidth - windowWidth) / 2;
      y = (screenHeight - windowHeight) / 2;
      break;
  }

  overlayWindow.setPosition(Math.floor(x), Math.floor(y));
}

// Create settings window
function createSettingsWindow() {
  if (settingsWindow) {
    settingsWindow.focus();
    return;
  }

  settingsWindow = new BrowserWindow({
    width: 550,
    height: 650,
    resizable: false,
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false
    },
    icon: path.join(__dirname, 'test.jpg')
  });

  settingsWindow.loadFile(path.join(__dirname, 'renderer', 'settings.html'));
  settingsWindow.setMenu(null);

  settingsWindow.on('closed', () => {
    settingsWindow = null;
  });
}

// WebSocket connection
function connectWebSocket() {
  const config = getConfig();

  if (!config.serverUrl) {
    console.log('Server URL not configured');
    return;
  }

  console.log('Connecting to:', config.serverUrl);

  ws = new WebSocket(config.serverUrl);

  ws.on('open', () => {
    console.log('Connected to BozoChat server');
    updateTrayStatus('connected');

    // Send authentication if userId is set
    if (config.userId) {
      ws.send(JSON.stringify({
        type: 'auth',
        userId: config.userId
      }));
    }
  });

  ws.on('message', (data) => {
    try {
      const message = JSON.parse(data.toString());
      handleServerMessage(message);
    } catch (error) {
      console.error('Error parsing message:', error);
    }
  });

  ws.on('close', () => {
    console.log('Disconnected from server');
    updateTrayStatus('disconnected');

    // Auto-reconnect after 5 seconds
    if (reconnectInterval) clearTimeout(reconnectInterval);
    reconnectInterval = setTimeout(() => {
      connectWebSocket();
    }, 5000);
  });

  ws.on('error', (error) => {
    console.error('WebSocket error:', error.message);
  });
}

// Handle server messages
function handleServerMessage(message) {
  console.log('Received message:', message.type);

  switch (message.type) {
    case 'notification':
      displayNotification(message.data);
      break;
    case 'ping':
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'pong' }));
      }
      break;
    case 'server-shutdown':
      console.log('Server is shutting down');
      break;
  }
}

// Display notification
function displayNotification(data) {
  if (!overlayWindow) return;

  console.log('Displaying notification from:', data.sender);

  overlayWindow.webContents.send('show-notification', data);
  overlayWindow.show();

  // Auto-hide after duration
  const duration = data.duration || getConfig().defaultDuration;
  setTimeout(() => {
    if (overlayWindow && overlayWindow.isVisible()) {
      overlayWindow.hide();
    }
  }, duration);
}

// Create system tray icon
function createTray() {
  // Create a simple icon (you'll want to replace this with an actual icon file)
  const icon = nativeImage.createFromPath(path.join(__dirname, 'test.jpg'));

  tray = new Tray(icon.resize({ width: 16, height: 16 }));

  updateTrayMenu();

  tray.setToolTip('BozoChat Client');

  // Double-click to open settings
  tray.on('double-click', () => {
    createSettingsWindow();
  });
}

// Update tray menu
function updateTrayMenu() {
  const config = getConfig();
  const isConnected = ws && ws.readyState === WebSocket.OPEN;

  const contextMenu = Menu.buildFromTemplate([
    {
      label: `BozoChat ${isConnected ? '(Connected)' : '(Disconnected)'}`,
      enabled: false,
      icon: isConnected ? null : null
    },
    { type: 'separator' },
    {
      label: 'Settings',
      click: () => {
        createSettingsWindow();
      }
    },
    {
      label: isConnected ? 'Disconnect' : 'Reconnect',
      click: () => {
        if (isConnected) {
          if (ws) ws.close();
        } else {
          connectWebSocket();
        }
      }
    },
    { type: 'separator' },
    {
      label: 'Test Notification',
      click: () => {
        displayNotification({
          sender: 'System',
          message: 'This is a test notification!',
          mediaType: 'text',
          duration: 3000
        });
      }
    },
    { type: 'separator' },
    {
      label: 'Quit',
      click: () => {
        app.quit();
      }
    }
  ]);

  tray.setContextMenu(contextMenu);
}

// Update tray status
function updateTrayStatus(status) {
  updateTrayMenu();

  // You could also update the tray icon based on connection status
  if (status === 'connected') {
    tray.setTitle('●'); // Green dot indicator
  } else {
    tray.setTitle('○'); // Empty circle
  }
}

// IPC Handlers
ipcMain.handle('get-config', () => {
  return getConfig();
});

ipcMain.handle('save-config', (event, config) => {
  saveConfig(config);

  // Reconnect if server URL changed
  if (ws) {
    ws.close();
  }
  connectWebSocket();

  // Reposition overlay if needed
  if (overlayWindow && config.overlayPosition) {
    positionOverlay(config.overlayPosition);
  }

  return { success: true };
});

ipcMain.on('hide-overlay', () => {
  if (overlayWindow) {
    overlayWindow.hide();
  }
});

// App initialization
app.whenReady().then(() => {
  createOverlayWindow();
  createTray();
  connectWebSocket();

  // Auto-start configuration (Windows)
  if (process.platform === 'win32') {
    const config = getConfig();
    app.setLoginItemSettings({
      openAtLogin: config.autoStart,
      path: process.execPath
    });
  }
});

// Prevent app from quitting when all windows are closed
app.on('window-all-closed', (e) => {
  // Do nothing - keep app running in tray
});

// Clean quit
app.on('before-quit', () => {
  if (ws) {
    ws.close();
  }
  if (reconnectInterval) {
    clearTimeout(reconnectInterval);
  }
});

app.on('activate', () => {
  if (overlayWindow === null) {
    createOverlayWindow();
  }
});
