"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const electron_1 = require("electron");
electron_1.contextBridge.exposeInMainWorld('electron', {
    ping: () => electron_1.ipcRenderer.invoke('ping'),
    on: (channel, callback) => {
        electron_1.ipcRenderer.on(channel, callback);
    },
    off: (channel, callback) => {
        electron_1.ipcRenderer.removeListener(channel, callback);
    },
    invoke: (channel, ...args) => electron_1.ipcRenderer.invoke(channel, ...args),
    getPathForFile: (file) => electron_1.webUtils.getPathForFile(file),
});
