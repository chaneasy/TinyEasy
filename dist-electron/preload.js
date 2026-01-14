import { contextBridge, ipcRenderer } from 'electron';
contextBridge.exposeInMainWorld('electron', {
    ping: () => ipcRenderer.invoke('ping'),
    // Add more IPC methods here
    on: (channel, callback) => {
        ipcRenderer.on(channel, callback);
    },
    off: (channel, callback) => {
        ipcRenderer.removeListener(channel, callback);
    },
    invoke: (channel, ...args) => ipcRenderer.invoke(channel, ...args),
});
