const { contextBridge, ipcRenderer } = require('electron');

// Expose protected methods to the renderer process
contextBridge.exposeInMainWorld('electronAPI', {
  // Config management
  getConfig: () => ipcRenderer.invoke('get-config'),
  saveConfig: (config) => ipcRenderer.invoke('save-config', config),

  // Overlay control
  hideOverlay: () => ipcRenderer.send('hide-overlay'),

  // Notification listener
  onShowNotification: (callback) => {
    ipcRenderer.on('show-notification', (event, data) => callback(data));
  },

  // Remove notification listener
  removeNotificationListener: () => {
    ipcRenderer.removeAllListeners('show-notification');
  }
});
