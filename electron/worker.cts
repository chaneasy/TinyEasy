import { parentPort } from "worker_threads";
import fs from "fs/promises";
import path from "path";
import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

interface CompressionTask {
	filePath: string;
	options: {
		originalSize: number;
		level?: "low" | "medium" | "high";
		outputDir?: string;
	};
}

if (!parentPort) {
	throw new Error("Must be run as a worker");
}

function normalizeLevel(level: CompressionTask["options"]["level"]) {
	return level === "low" || level === "high" ? level : "medium";
}

parentPort.on("message", async (task: CompressionTask) => {
	try {
		const { filePath, options } = task;
		const inputExt = path.extname(filePath).toLowerCase();
		
		if (inputExt !== ".png" && inputExt !== ".jpg" && inputExt !== ".jpeg") {
			parentPort?.postMessage({
				success: false,
				filePath,
				error: "Only PNG/JPG are supported (oxipng for PNG, jpegoptim for JPG)",
			});
			return;
		}

		const fileBuffer = await fs.readFile(filePath);

		const dir =
			options.outputDir && options.outputDir.length > 0
				? options.outputDir
				: path.dirname(filePath);
		const name = path.basename(filePath, inputExt);
		const newFileName = `${name}_min${inputExt}`;
		const outputPath = path.join(dir, newFileName);

		await fs.mkdir(dir, { recursive: true });

		if (inputExt === ".png") {
			const level = normalizeLevel(options.level);
			const optLevel = level === "high" ? 6 : level === "low" ? 2 : 4;
			const strip = level === "high" ? "all" : "safe";

			const mod = (await import("oxipng-bin")) as unknown as {
				default: string;
			};
			const oxipngPath = mod.default;

			const tmpPath = `${outputPath}.tmp`;
			try {
				await fs.unlink(tmpPath);
			} catch {}

			const args = ["--opt", String(optLevel)];
			args.push("--strip", strip);
			args.push("--out", tmpPath, filePath);

			await execFileAsync(oxipngPath, args);

			const [before, after] = await Promise.all([
				fs.stat(filePath),
				fs.stat(tmpPath),
			]);

			let didFallback = false;
			if (after.size >= before.size) {
				await fs.copyFile(filePath, outputPath);
				didFallback = true;
			} else {
				await fs.copyFile(tmpPath, outputPath);
			}

			try {
				await fs.unlink(tmpPath);
			} catch {}

			parentPort?.postMessage({
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
			await fs.unlink(tmpInputPath);
		} catch {}

		await fs.copyFile(filePath, tmpInputPath);

		const jpegoptimArgs = ["--max", String(maxQuality)];
		if (stripAll) jpegoptimArgs.push("--strip-all");
		if (progressive) jpegoptimArgs.push("--all-progressive");
		jpegoptimArgs.push(tmpInputPath);

		try {
			await execFileAsync("jpegoptim", jpegoptimArgs);
		} catch (e) {
			try {
				await fs.unlink(tmpInputPath);
			} catch {}
			const msg = e instanceof Error ? e.message : String(e);
			parentPort?.postMessage({
				success: false,
				filePath,
				error: msg.includes("ENOENT")
					? "jpegoptim not found, please install it first (macOS: brew install jpegoptim)"
					: msg,
			});
			return;
		}

		const [before, after] = await Promise.all([
			fs.stat(filePath),
			fs.stat(tmpInputPath),
		]);

		let didFallback = false;
		if (after.size >= before.size) {
			await fs.copyFile(filePath, outputPath);
			didFallback = true;
		} else {
			await fs.copyFile(tmpInputPath, outputPath);
		}

		try {
			await fs.unlink(tmpInputPath);
		} catch {}

		parentPort?.postMessage({
			success: true,
			filePath,
			outputPath,
			originalSize: options.originalSize ?? fileBuffer.length,
			compressedSize: didFallback ? before.size : after.size,
			didFallback,
		});

	} catch (error) {
		parentPort?.postMessage({
			success: false,
			filePath: task.filePath,
			error: error instanceof Error ? error.message : String(error),
		});
	}
});
