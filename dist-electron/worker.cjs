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
                error: "Only PNG/JPG are supported (oxipng for PNG, mozjpeg/cjpeg for JPG)",
            });
            return;
        }
        const fileBuffer = await promises_1.default.readFile(filePath);
        const dir = options.outputDir && options.outputDir.length > 0
            ? options.outputDir
            : path_1.default.dirname(filePath);
        const newFileName = path_1.default.basename(filePath);
        const outputPath = path_1.default.join(dir, newFileName);
        await promises_1.default.mkdir(dir, { recursive: true });
        const shouldOverwrite = options.overwrite === true;
        const outputIsInput = path_1.default.resolve(outputPath) === path_1.default.resolve(filePath);
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
            if (shouldOverwrite) {
                await promises_1.default.copyFile(tmpPath, outputPath);
            }
            else if (after.size >= before.size) {
                if (!outputIsInput) {
                    await promises_1.default.copyFile(filePath, outputPath);
                }
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
                compressedSize: shouldOverwrite
                    ? after.size
                    : didFallback
                        ? before.size
                        : after.size,
                didFallback,
            });
            return;
        }
        const level = normalizeLevel(options.level);
        const maxQuality = level === "high" ? 40 : level === "low" ? 85 : 60;
        const stripAll = true;
        const progressive = true;
        const tmpInputPath = `${outputPath}.tmp_in${inputExt}`;
        const tmpOutputPath = `${outputPath}.tmp_out${inputExt}`;
        try {
            await promises_1.default.unlink(tmpInputPath);
        }
        catch { }
        try {
            await promises_1.default.unlink(tmpOutputPath);
        }
        catch { }
        await promises_1.default.copyFile(filePath, tmpInputPath);
        try {
            const mod = (await import("mozjpeg"));
            const cjpegPath = typeof mod === "string"
                ? mod
                : typeof mod?.default === "string"
                    ? mod.default
                    : "";
            if (!cjpegPath) {
                throw new Error("mozjpeg binary path not found");
            }
            const args = [];
            args.push("-quality", String(maxQuality));
            if (stripAll)
                args.push("-optimize");
            if (progressive)
                args.push("-progressive");
            args.push("-outfile", tmpOutputPath, tmpInputPath);
            await execFileAsync(cjpegPath, args);
        }
        catch (e) {
            try {
                await promises_1.default.unlink(tmpInputPath);
            }
            catch { }
            try {
                await promises_1.default.unlink(tmpOutputPath);
            }
            catch { }
            const msg = e instanceof Error ? e.message : String(e);
            worker_threads_1.parentPort?.postMessage({
                success: false,
                filePath,
                error: `JPEG compression failed (mozjpeg/cjpeg): ${msg}`,
            });
            return;
        }
        const [before, after] = await Promise.all([
            promises_1.default.stat(filePath),
            promises_1.default.stat(tmpOutputPath),
        ]);
        let didFallback = false;
        if (shouldOverwrite) {
            await promises_1.default.copyFile(tmpOutputPath, outputPath);
        }
        else if (after.size >= before.size) {
            if (!outputIsInput) {
                await promises_1.default.copyFile(filePath, outputPath);
            }
            didFallback = true;
        }
        else {
            await promises_1.default.copyFile(tmpOutputPath, outputPath);
        }
        try {
            await promises_1.default.unlink(tmpInputPath);
        }
        catch { }
        try {
            await promises_1.default.unlink(tmpOutputPath);
        }
        catch { }
        worker_threads_1.parentPort?.postMessage({
            success: true,
            filePath,
            outputPath,
            originalSize: options.originalSize ?? fileBuffer.length,
            compressedSize: shouldOverwrite
                ? after.size
                : didFallback
                    ? before.size
                    : after.size,
            didFallback,
            toolUsed: "cjpeg",
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
