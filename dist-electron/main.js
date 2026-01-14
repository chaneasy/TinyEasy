import { app, BrowserWindow, ipcMain, dialog } from "electron";
import path from "path";
import { fileURLToPath } from "url";
import { Worker } from "worker_threads";
import { createRequire } from "module";
import fs from "fs/promises";
import { randomUUID } from "crypto";
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
// Handle creating/removing shortcuts on Windows when installing/uninstalling.
if (require("electron-squirrel-startup")) {
    app.quit();
}
let mainWindow = null;
let lastSelectedOutputDir = null;
const createWindow = () => {
    const iconPath = app.isPackaged
        ? path.join(process.resourcesPath, "icon.png")
        : path.join(__dirname, "../build/icon.png");
    mainWindow = new BrowserWindow({
        title: "TinyEasy",
        icon: iconPath,
        width: 1024,
        height: 748,
        resizable: false,
        webPreferences: {
            preload: path.join(__dirname, "preload.cjs"),
            nodeIntegration: false,
            contextIsolation: true,
            webSecurity: false,
            sandbox: false,
        },
    });
    const devServerUrl = process.env.VITE_DEV_SERVER_URL ??
        (app.isPackaged ? undefined : "http://localhost:5173/");
    if (devServerUrl) {
        mainWindow.loadURL(devServerUrl);
        mainWindow.webContents.openDevTools();
    }
    else {
        mainWindow.loadFile(path.join(__dirname, "../dist/index.html"));
    }
    mainWindow.on("resized", () => {
        if (mainWindow) {
            const [width, height] = mainWindow.getSize();
            mainWindow.webContents.send("window-resized", { width, height });
        }
    });
};
if (!app.isPackaged) {
    app.commandLine.appendSwitch("no-sandbox");
}
app.on("ready", createWindow);
app.on("window-all-closed", () => {
    if (process.platform !== "darwin") {
        app.quit();
    }
});
app.on("activate", () => {
    if (mainWindow === null) {
        createWindow();
    }
});
// Worker handling
const runCompressionWorker = (task) => {
    return new Promise((resolve, reject) => {
        const workerPath = path.join(__dirname, "worker.cjs");
        const worker = new Worker(workerPath);
        worker.postMessage(task);
        worker.on("message", (result) => {
            resolve(result);
            worker.terminate();
        });
        worker.on("error", (error) => {
            reject(error);
            worker.terminate();
        });
        worker.on("exit", (code) => {
            if (code !== 0) {
                reject(new Error(`Worker stopped with exit code ${code}`));
            }
        });
    });
};
// IPC Handlers
ipcMain.handle("ping", () => "pong");
ipcMain.handle("select-files", async () => {
    // Note: Supports "Select File" and "Select Folder"; folders will recursively scan image files (non-PNG/JPG will also be returned, but not compressed later).
    const IMAGE_EXTS = new Set([
        ".png",
        ".jpg",
        ".jpeg",
        ".webp",
        ".gif",
        ".bmp",
        ".tif",
        ".tiff",
        ".avif",
        ".heic",
        ".heif",
    ]);
    const isImagePath = (p) => IMAGE_EXTS.has(path.extname(p).toLowerCase());
    const walkDir = async (dirPath, rootPath, out) => {
        const entries = await fs.readdir(dirPath, { withFileTypes: true });
        for (const ent of entries) {
            const fullPath = path.join(dirPath, ent.name);
            if (ent.isDirectory()) {
                await walkDir(fullPath, rootPath, out);
            }
            else if (ent.isFile()) {
                if (isImagePath(fullPath)) {
                    out.push({
                        path: fullPath,
                        // 只保留“选中目录内部”的结构，不包含顶层目录名
                        relativePath: path.relative(rootPath, fullPath),
                    });
                }
            }
        }
    };
    const result = await dialog.showOpenDialog({
        properties: ["openFile", "openDirectory", "multiSelections"],
        filters: [
            {
                name: "Images",
                extensions: [
                    "png",
                    "jpg",
                    "jpeg",
                    "webp",
                    "gif",
                    "bmp",
                    "tif",
                    "tiff",
                    "avif",
                    "heic",
                    "heif",
                ],
            },
        ],
    });
    if (result.canceled)
        return [];
    const collectedFiles = [];
    for (const p of result.filePaths) {
        const st = await fs.stat(p);
        if (st.isDirectory()) {
            await walkDir(p, p, collectedFiles);
        }
        else if (st.isFile()) {
            if (isImagePath(p)) {
                collectedFiles.push({
                    path: p,
                    relativePath: path.basename(p),
                });
            }
        }
    }
    const files = await Promise.all(collectedFiles.map(async (file) => {
        const stat = await fs.stat(file.path);
        return {
            name: path.basename(file.path),
            path: file.path,
            size: stat.size,
            relativePath: file.relativePath,
        };
    }));
    return files;
});
ipcMain.handle("select-folder", async () => {
    const result = await dialog.showOpenDialog({
        properties: ["openDirectory", "createDirectory"],
    });
    if (result.canceled)
        return null;
    const selected = result.filePaths[0] ?? null;
    if (typeof selected === "string" && selected.length > 0) {
        lastSelectedOutputDir = selected;
    }
    return selected;
});
ipcMain.handle("copy-image-to-output", async (event, { filePath, options }) => {
    try {
        if (!filePath || typeof filePath !== "string") {
            return { success: false, error: "filePath is empty" };
        }
        let outputDir = typeof options?.outputDir === "string" && options.outputDir.length > 0
            ? options.outputDir
            : typeof lastSelectedOutputDir === "string" &&
                lastSelectedOutputDir.length > 0
                ? lastSelectedOutputDir
                : path.join(app.getPath("downloads"), "EasyPNG");
        const relativePath = typeof options?.relativePath === "string" ? options.relativePath : "";
        if (relativePath) {
            const relativeDir = path.normalize(path.dirname(relativePath));
            if (relativeDir !== "." &&
                !path.isAbsolute(relativeDir) &&
                !relativeDir.startsWith("..")) {
                outputDir = path.join(outputDir, relativeDir);
            }
        }
        await fs.mkdir(outputDir, { recursive: true });
        const outputPath = path.join(outputDir, path.basename(filePath));
        if (path.resolve(outputPath) === path.resolve(filePath)) {
            const st = await fs.stat(filePath);
            return {
                success: true,
                filePath,
                outputPath,
                originalSize: st.size,
                compressedSize: st.size,
                didFallback: true,
            };
        }
        await fs.copyFile(filePath, outputPath);
        const st = await fs.stat(filePath);
        return {
            success: true,
            filePath,
            outputPath,
            originalSize: st.size,
            compressedSize: st.size,
            didFallback: true,
        };
    }
    catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error),
        };
    }
});
ipcMain.handle("compress-image", async (event, { filePath, options }) => {
    try {
        if (!filePath || typeof filePath !== "string") {
            return { success: false, error: "filePath is empty or invalid" };
        }
        console.log("Compressing:", filePath, "Options:", options);
        const ext = path.extname(filePath).toLowerCase();
        if (ext !== ".png" && ext !== ".jpg" && ext !== ".jpeg") {
            return {
                success: false,
                error: "Only PNG/JPG supported",
            };
        }
        try {
            await fs.access(filePath);
        }
        catch (e) {
            return { success: false, error: `Cannot read input file: ${filePath}` };
        }
        let outputDir = typeof options?.outputDir === "string" && options.outputDir.length > 0
            ? options.outputDir
            : typeof lastSelectedOutputDir === "string" &&
                lastSelectedOutputDir.length > 0
                ? lastSelectedOutputDir
                : undefined;
        if (!outputDir) {
            outputDir = path.join(app.getPath("downloads"), "EasyPNG");
        }
        if (options?.overwrite) {
            outputDir = path.dirname(filePath);
        }
        else {
            const relativePath = typeof options?.relativePath === "string" ? options.relativePath : "";
            if (relativePath) {
                const relativeDir = path.normalize(path.dirname(relativePath));
                if (relativeDir !== "." &&
                    !path.isAbsolute(relativeDir) &&
                    !relativeDir.startsWith("..")) {
                    outputDir = path.join(outputDir, relativeDir);
                }
            }
        }
        console.log("Final outputDir:", outputDir);
        await fs.mkdir(outputDir, { recursive: true });
        // If overwrite is true, we should output to a temp file first then move,
        // but runCompressionWorker handles output logic.
        // If we pass outputDir same as input dir, it might conflict if filename is same.
        // Worker adds prefix/suffix? No, usually workers just write to outputDir/filename.
        // Let's check worker logic if possible or assume standard behavior.
        // If we overwrite, we might need to handle atomic write.
        // For now, let's pass overwrite flag to worker if needed, or just let it write.
        const result = await runCompressionWorker({
            filePath,
            options: { ...options, outputDir },
        });
        // If overwrite was requested and worker wrote to a new file (if it didn't overwrite in place),
        // we might need to handle it. But typically compression tools might overwrite if forced.
        // However, mozjpeg/oxipng behavior varies.
        // Let's assume the worker writes to outputDir/filename.
        // If outputDir == inputDir, it overwrites.
        return result;
    }
    catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error),
        };
    }
});
ipcMain.handle("compress-image-buffer", async (event, { fileName, buffer, options }) => {
    try {
        if (!buffer) {
            return { success: false, error: "buffer is empty" };
        }
        if (options?.overwrite) {
            return {
                success: false,
                error: "Requires a local file path. ",
            };
        }
        let outputDir = typeof options?.outputDir === "string" && options.outputDir.length > 0
            ? options.outputDir
            : typeof lastSelectedOutputDir === "string" &&
                lastSelectedOutputDir.length > 0
                ? lastSelectedOutputDir
                : path.join(app.getPath("downloads"), "EasyPNG");
        const relativePath = typeof options?.relativePath === "string" ? options.relativePath : "";
        if (relativePath) {
            const relativeDir = path.normalize(path.dirname(relativePath));
            if (relativeDir !== "." &&
                !path.isAbsolute(relativeDir) &&
                !relativeDir.startsWith("..")) {
                outputDir = path.join(outputDir, relativeDir);
            }
        }
        await fs.mkdir(outputDir, { recursive: true });
        const inputDir = path.join(app.getPath("temp"), "EasyPNG");
        await fs.mkdir(inputDir, { recursive: true });
        const baseName = typeof fileName === "string" && fileName.length > 0
            ? path.basename(fileName)
            : `upload_${Date.now()}`;
        const data = buffer instanceof Uint8Array
            ? buffer
            : buffer instanceof ArrayBuffer
                ? new Uint8Array(buffer)
                : new Uint8Array(buffer?.buffer ?? buffer);
        const pngMagicOk = data.length >= 8 &&
            data[0] === 0x89 &&
            data[1] === 0x50 &&
            data[2] === 0x4e &&
            data[3] === 0x47 &&
            data[4] === 0x0d &&
            data[5] === 0x0a &&
            data[6] === 0x1a &&
            data[7] === 0x0a;
        const jpegMagicOk = data.length >= 3 &&
            data[0] === 0xff &&
            data[1] === 0xd8 &&
            data[2] === 0xff;
        if (!pngMagicOk && !jpegMagicOk) {
            return {
                success: false,
                error: "Only PNG/JPG supported",
            };
        }
        const ext = pngMagicOk ? ".png" : ".jpg";
        const normalizedBase = baseName.toLowerCase();
        const baseNoExt = baseName.replace(/\.[^/.]+$/, "");
        const hasPngExt = normalizedBase.endsWith(".png");
        const hasJpegExt = normalizedBase.endsWith(".jpg") || normalizedBase.endsWith(".jpeg");
        const safeName = (ext === ".png" && hasPngExt) || (ext === ".jpg" && hasJpegExt)
            ? baseName
            : `${baseNoExt}${ext}`;
        const tmpDir = path.join(inputDir, randomUUID());
        await fs.mkdir(tmpDir, { recursive: true });
        const inputPath = path.join(tmpDir, safeName);
        await fs.writeFile(inputPath, data);
        try {
            const result = await runCompressionWorker({
                filePath: inputPath,
                options: {
                    ...options,
                    originalSize: options?.originalSize ?? data.byteLength,
                    outputDir,
                },
            });
            return result;
        }
        finally {
            try {
                await fs.rm(tmpDir, { recursive: true, force: true });
            }
            catch { }
        }
    }
    catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error),
        };
    }
});
ipcMain.handle("save-file-buffer", async (event, { fileName, buffer, options }) => {
    try {
        if (!buffer)
            return { success: false, error: "buffer is empty" };
        let outputDir = typeof options?.outputDir === "string" && options.outputDir.length > 0
            ? options.outputDir
            : typeof lastSelectedOutputDir === "string" &&
                lastSelectedOutputDir.length > 0
                ? lastSelectedOutputDir
                : path.join(app.getPath("downloads"), "EasyPNG");
        const relativePath = typeof options?.relativePath === "string" ? options.relativePath : "";
        if (relativePath) {
            const relativeDir = path.normalize(path.dirname(relativePath));
            if (relativeDir !== "." &&
                !path.isAbsolute(relativeDir) &&
                !relativeDir.startsWith("..")) {
                outputDir = path.join(outputDir, relativeDir);
            }
        }
        await fs.mkdir(outputDir, { recursive: true });
        const baseName = typeof fileName === "string" && fileName.length > 0
            ? path.basename(fileName)
            : `upload_${Date.now()}`;
        const data = buffer instanceof Uint8Array
            ? buffer
            : buffer instanceof ArrayBuffer
                ? new Uint8Array(buffer)
                : new Uint8Array(buffer?.buffer ?? buffer);
        const outputPath = path.join(outputDir, baseName);
        await fs.writeFile(outputPath, data);
        return {
            success: true,
            outputPath,
            originalSize: data.byteLength,
            compressedSize: data.byteLength,
            didFallback: true,
        };
    }
    catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error),
        };
    }
});
