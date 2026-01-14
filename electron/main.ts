import { app, BrowserWindow, ipcMain, dialog } from "electron";
import path from "path";
import { fileURLToPath } from "url";
import { Worker } from "worker_threads";
import { createRequire } from "module";
import fs from "fs/promises";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

// Handle creating/removing shortcuts on Windows when installing/uninstalling.
if (require("electron-squirrel-startup")) {
	app.quit();
}

let mainWindow: BrowserWindow | null = null;

const createWindow = () => {
	mainWindow = new BrowserWindow({
		title: "TinyEasy",
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

	const devServerUrl =
		process.env.VITE_DEV_SERVER_URL ??
		(app.isPackaged ? undefined : "http://localhost:5173/");

	if (devServerUrl) {
		mainWindow.loadURL(devServerUrl);
		mainWindow.webContents.openDevTools();
	} else {
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
const runCompressionWorker = (task: any) => {
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

	const isImagePath = (p: string) =>
		IMAGE_EXTS.has(path.extname(p).toLowerCase());

	const walkDir = async (dirPath: string, out: string[]) => {
		const entries = await fs.readdir(dirPath, { withFileTypes: true });
		for (const ent of entries) {
			const fullPath = path.join(dirPath, ent.name);
			if (ent.isDirectory()) {
				await walkDir(fullPath, out);
			} else if (ent.isFile()) {
				if (isImagePath(fullPath)) out.push(fullPath);
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

	if (result.canceled) return [];

	const collectedPaths: string[] = [];
	for (const p of result.filePaths) {
		const st = await fs.stat(p);
		if (st.isDirectory()) {
			await walkDir(p, collectedPaths);
		} else if (st.isFile()) {
			if (isImagePath(p)) collectedPaths.push(p);
		}
	}

	const files = await Promise.all(
		collectedPaths.map(async (filePath) => {
			const stat = await fs.stat(filePath);
			return { name: path.basename(filePath), path: filePath, size: stat.size };
		})
	);

	return files;
});

ipcMain.handle("compress-image", async (event, { filePath, options }) => {
	try {
		if (!filePath || typeof filePath !== "string") {
			return { success: false, error: "filePath 为空或非法" };
		}

		const ext = path.extname(filePath).toLowerCase();
		if (ext !== ".png" && ext !== ".jpg" && ext !== ".jpeg") {
			return {
				success: false,
				error: "仅支持 PNG/JPG（PNG 用 oxipng，JPG 用 jpegoptim）",
			};
		}

		try {
			await fs.access(filePath);
		} catch (e) {
			return { success: false, error: `无法读取输入文件：${filePath}` };
		}

		const outputDir = path.join(app.getPath("downloads"), "EasyPNG");
		await fs.mkdir(outputDir, { recursive: true });

		const result: any = await runCompressionWorker({
			filePath,
			options: { ...options, outputDir },
		});
		return result;
	} catch (error) {
		return {
			success: false,
			error: error instanceof Error ? error.message : String(error),
		};
	}
});

ipcMain.handle(
	"compress-image-buffer",
	async (event, { fileName, buffer, options }) => {
		try {
			if (!buffer) {
				return { success: false, error: "buffer 为空" };
			}

			const outputDir = path.join(app.getPath("downloads"), "EasyPNG");
			await fs.mkdir(outputDir, { recursive: true });

			const inputDir = path.join(app.getPath("temp"), "EasyPNG");
			await fs.mkdir(inputDir, { recursive: true });

			const baseName =
				typeof fileName === "string" && fileName.length > 0
					? path.basename(fileName)
					: `upload_${Date.now()}`;

			const data =
				buffer instanceof Uint8Array
					? buffer
					: buffer instanceof ArrayBuffer
					? new Uint8Array(buffer)
					: new Uint8Array(buffer?.buffer ?? buffer);

			const pngMagicOk =
				data.length >= 8 &&
				data[0] === 0x89 &&
				data[1] === 0x50 &&
				data[2] === 0x4e &&
				data[3] === 0x47 &&
				data[4] === 0x0d &&
				data[5] === 0x0a &&
				data[6] === 0x1a &&
				data[7] === 0x0a;

			const jpegMagicOk =
				data.length >= 3 &&
				data[0] === 0xff &&
				data[1] === 0xd8 &&
				data[2] === 0xff;

			if (!pngMagicOk && !jpegMagicOk) {
				return {
					success: false,
					error: "仅支持 PNG/JPG（PNG 用 oxipng，JPG 用 jpegoptim）",
				};
			}

			const ext = pngMagicOk ? ".png" : ".jpg";
			const normalizedBase = baseName.toLowerCase();
			const baseNoExt = baseName.replace(/\.[^/.]+$/, "");
			const hasPngExt = normalizedBase.endsWith(".png");
			const hasJpegExt =
				normalizedBase.endsWith(".jpg") || normalizedBase.endsWith(".jpeg");
			const safeName =
				(ext === ".png" && hasPngExt) || (ext === ".jpg" && hasJpegExt)
					? baseName
					: `${baseNoExt}${ext}`;
			const inputPath = path.join(inputDir, `${Date.now()}_${safeName}`);

			await fs.writeFile(inputPath, data);

			const result: any = await runCompressionWorker({
				filePath: inputPath,
				options: {
					...options,
					originalSize: options?.originalSize ?? data.byteLength,
					outputDir,
				},
			});
			return result;
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : String(error),
			};
		}
	}
);
