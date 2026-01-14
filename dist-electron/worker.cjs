"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const worker_threads_1 = require("worker_threads");
const promises_1 = __importDefault(require("fs/promises"));
const path_1 = __importDefault(require("path"));
const child_process_1 = require("child_process");
const util_1 = require("util");
const execFileAsync = (0, util_1.promisify)(child_process_1.execFile);
if (!worker_threads_1.parentPort) {
    throw new Error("Must be run as a worker");
}
function normalizeLevel(level) {
    return level === "low" || level === "high" ? level : "medium";
}
worker_threads_1.parentPort.on("message", async (task) => {
    try {
        const { filePath, options } = task;
        const inputExt = path_1.default.extname(filePath).toLowerCase();
        if (inputExt !== ".png" && inputExt !== ".jpg" && inputExt !== ".jpeg") {
            worker_threads_1.parentPort?.postMessage({
                success: false,
                filePath,
                error: "仅支持 PNG/JPG（PNG 用 oxipng，JPG 用 jpegoptim）",
            });
            return;
        }
        const fileBuffer = await promises_1.default.readFile(filePath);
        const dir = options.outputDir && options.outputDir.length > 0
            ? options.outputDir
            : path_1.default.dirname(filePath);
        const name = path_1.default.basename(filePath, inputExt);
        const newFileName = `${name}_min${inputExt}`;
        const outputPath = path_1.default.join(dir, newFileName);
        await promises_1.default.mkdir(dir, { recursive: true });
        if (inputExt === ".png") {
            const level = normalizeLevel(options.level);
            const optLevel = level === "high" ? 6 : level === "low" ? 2 : 4;
            const strip = level === "high" ? "all" : "safe";
            const mod = (await import("oxipng-bin"));
            const oxipngPath = mod.default;
            const tmpPath = `${outputPath}.tmp`;
            try {
                await promises_1.default.unlink(tmpPath);
            }
            catch { }
            const args = ["--opt", String(optLevel)];
            args.push("--strip", strip);
            args.push("--out", tmpPath, filePath);
            await execFileAsync(oxipngPath, args);
            const [before, after] = await Promise.all([
                promises_1.default.stat(filePath),
                promises_1.default.stat(tmpPath),
            ]);
            let didFallback = false;
            if (after.size >= before.size) {
                await promises_1.default.copyFile(filePath, outputPath);
                didFallback = true;
            }
            else {
                await promises_1.default.copyFile(tmpPath, outputPath);
            }
            try {
                await promises_1.default.unlink(tmpPath);
            }
            catch { }
            worker_threads_1.parentPort?.postMessage({
                success: true,
                filePath,
                outputPath,
                originalSize: options.originalSize ?? fileBuffer.length,
                compressedSize: didFallback ? before.size : after.size,
                didFallback,
            });
            return;
        }
        const level = normalizeLevel(options.level);
        const maxQuality = level === "high" ? 40 : level === "low" ? 85 : 60;
        const stripAll = true;
        const progressive = true;
        const tmpInputPath = `${outputPath}.tmp_in${inputExt}`;
        try {
            await promises_1.default.unlink(tmpInputPath);
        }
        catch { }
        await promises_1.default.copyFile(filePath, tmpInputPath);
        const jpegoptimArgs = ["--max", String(maxQuality)];
        if (stripAll)
            jpegoptimArgs.push("--strip-all");
        if (progressive)
            jpegoptimArgs.push("--all-progressive");
        jpegoptimArgs.push(tmpInputPath);
        try {
            await execFileAsync("jpegoptim", jpegoptimArgs);
        }
        catch (e) {
            try {
                await promises_1.default.unlink(tmpInputPath);
            }
            catch { }
            const msg = e instanceof Error ? e.message : String(e);
            worker_threads_1.parentPort?.postMessage({
                success: false,
                filePath,
                error: msg.includes("ENOENT")
                    ? "未找到 jpegoptim，请先安装（macOS: brew install jpegoptim）"
                    : msg,
            });
            return;
        }
        const [before, after] = await Promise.all([
            promises_1.default.stat(filePath),
            promises_1.default.stat(tmpInputPath),
        ]);
        let didFallback = false;
        if (after.size >= before.size) {
            await promises_1.default.copyFile(filePath, outputPath);
            didFallback = true;
        }
        else {
            await promises_1.default.copyFile(tmpInputPath, outputPath);
        }
        try {
            await promises_1.default.unlink(tmpInputPath);
        }
        catch { }
        worker_threads_1.parentPort?.postMessage({
            success: true,
            filePath,
            outputPath,
            originalSize: options.originalSize ?? fileBuffer.length,
            compressedSize: didFallback ? before.size : after.size,
            didFallback,
        });
    }
    catch (error) {
        worker_threads_1.parentPort?.postMessage({
            success: false,
            filePath: task.filePath,
            error: error instanceof Error ? error.message : String(error),
        });
    }
});
