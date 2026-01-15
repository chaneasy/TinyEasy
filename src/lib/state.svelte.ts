interface Settings {
	level: "low" | "medium" | "high";
}

interface CompressionFile {
	id: string;
	name: string;
	path?: string;
	sourceFile?: File;
	status: "pending" | "compressing" | "completed" | "skipped" | "error";
	originalSize: number;
	compressedSize?: number;
	outputPath?: string;
	error?: string;
	relativePath?: string;
}

interface SelectedFileEntry {
	name: string;
	path: string;
	size: number;
	relativePath?: string;
}

import toast from "svelte-5-french-toast";

import pLimit from "p-limit";

class AppState {
	files = $state<CompressionFile[]>([]);
	settings = $state<Settings>({
		level: "medium",
	});
	processing = $state(false);
	progressTotal = $state(0);
	progressDone = $state(0);

	showCompressionModal = $state(false);

	addFiles(newFiles: (File | { file: File; relativePath: string })[]) {
		// Note: Supports dragging files/folders; only accepts "image files", and marks non-PNG images as skipped (displayed but not compressed).
		const fileArray = Array.from(newFiles);
		const newEntries = fileArray.map((item) => {
			let file: File;
			let relativePath: string | undefined;

			if (item instanceof File) {
				file = item;
				relativePath = file.name;
			} else {
				file = item.file;
				relativePath = item.relativePath;
			}

			const lowerName = file.name.toLowerCase();
			const isImageByMime =
				typeof file.type === "string" && file.type.startsWith("image/");
			const isImageByExt =
				lowerName.endsWith(".png") ||
				lowerName.endsWith(".jpg") ||
				lowerName.endsWith(".jpeg") ||
				lowerName.endsWith(".webp") ||
				lowerName.endsWith(".gif") ||
				lowerName.endsWith(".bmp") ||
				lowerName.endsWith(".tif") ||
				lowerName.endsWith(".tiff") ||
				lowerName.endsWith(".avif") ||
				lowerName.endsWith(".heic") ||
				lowerName.endsWith(".heif");
			const isImage = isImageByMime || isImageByExt;
			const isPng = lowerName.endsWith(".png") || file.type === "image/png";
			const isJpg =
				lowerName.endsWith(".jpg") ||
				lowerName.endsWith(".jpeg") ||
				file.type === "image/jpeg";

			if (!isImage) {
				return null;
			}

			return {
				id: crypto.randomUUID(),
				name: file.name,
				path: (file as any).path,
				sourceFile: file,
				status: isPng || isJpg ? ("pending" as const) : ("skipped" as const),
				originalSize: file.size,
				error:
					isPng || isJpg
						? undefined
						: "Non-PNG/JPG images are displayed only, not compressed",
				relativePath,
			};
		});
		const filtered = newEntries.filter(
			(x): x is NonNullable<typeof x> => x !== null
		);
		this.files = [...this.files, ...filtered];
		if (filtered.length > 0) {
			toast.success(`Added ${filtered.length} files`);
		}
	}

	addFileEntries(entries: SelectedFileEntry[]) {
		// Note: Electron returns local path when selecting files/folders; PNG/JPG can be compressed, other images are marked as skipped (displayed but not compressed).
		const newEntries = entries.map((entry) => {
			const lowerName = entry.name.toLowerCase();
			const isPng = lowerName.endsWith(".png");
			const isJpg = lowerName.endsWith(".jpg") || lowerName.endsWith(".jpeg");
			const isImageByExt =
				lowerName.endsWith(".png") ||
				lowerName.endsWith(".jpg") ||
				lowerName.endsWith(".jpeg") ||
				lowerName.endsWith(".webp") ||
				lowerName.endsWith(".gif") ||
				lowerName.endsWith(".bmp") ||
				lowerName.endsWith(".tif") ||
				lowerName.endsWith(".tiff") ||
				lowerName.endsWith(".avif") ||
				lowerName.endsWith(".heic") ||
				lowerName.endsWith(".heif");

			return {
				id: crypto.randomUUID(),
				name: entry.name,
				path: entry.path,
				status:
					isPng || isJpg
						? ("pending" as const)
						: isImageByExt
						? ("skipped" as const)
						: ("error" as const),
				originalSize: entry.size,
				error:
					isPng || isJpg
						? undefined
						: isImageByExt
						? "Non-PNG/JPG images are displayed only, not compressed"
						: "Unsupported file type",
				relativePath: entry.relativePath,
			};
		});
		this.files = [...this.files, ...newEntries];
		if (newEntries.length > 0) {
			toast.success(`Added ${newEntries.length} files`);
		}
	}

	updateFile(id: string, updates: Partial<CompressionFile>) {
		const index = this.files.findIndex((f) => f.id === id);
		if (index !== -1) {
			// Create a new object to ensure reactivity if needed, though $state is fine with mutation if deep proxy
			// Svelte 5 $state is deep reactive for arrays/objects usually?
			// Actually, array mutation is fine.
			Object.assign(this.files[index], updates);
		}
	}

	clearCompleted() {
		// Note: Clear completed files and show Toast (no independent button provided on UI side).
		const completed = this.files.filter((f) => f.status === "completed").length;
		if (completed === 0) {
			toast("No completed files to clear", { duration: 2500 });
			return;
		}
		this.files = this.files.filter((f) => f.status !== "completed");
		toast.success(`Cleared ${completed} completed files`);
	}

	clearAll() {
		this.files = [];
	}

	async confirmCompression(mode: "overwrite" | "select-path") {
		this.showCompressionModal = false;
		if (mode === "overwrite") {
			await this.startCompression({ overwrite: true });
		} else {
			if (window.electron) {
				try {
					const path = await window.electron.invoke("select-folder");
					if (path) {
						await this.startCompression({ outputDir: path });
					}
				} catch (error: any) {
					console.error("Error selecting folder:", error);
					if (error.message?.includes("No handler registered")) {
						toast.error("Please restart the app to use this new feature", {
							duration: 5000,
						});
					} else {
						toast.error("Failed to select folder: " + error.message);
					}
				}
			} else {
				// Fallback for web?
				toast.error("Selecting folder is only supported in Electron app");
			}
		}
	}

	async startCompression(
		options: { overwrite?: boolean; outputDir?: string } = {}
	) {
		if (this.processing) return;
		this.processing = true;
		this.progressDone = 0;
		this.progressTotal = this.files.filter(
			(f) => f.status === "pending" || f.status === "skipped"
		).length;

		// Use p-limit to control concurrency (e.g. 5 concurrent tasks)
		const limit = pLimit(5);

		const toastId = toast.loading("Compressing...");
		let okCount = 0;
		let errCount = 0;

		const tasks = this.files.map((file) => {
			if (file.status === "pending" || file.status === "skipped") {
				return limit(async () => {
					const wasSkipped = file.status === "skipped";
					if (!wasSkipped) {
						this.updateFile(file.id, { status: "compressing" });
					}
					try {
						// Use window.electron if available
						if (window.electron) {
							const compressOptions = {
								level: this.settings.level,
								originalSize: file.originalSize,
								overwrite: options.overwrite,
								outputDir: options.outputDir,
								relativePath: file.relativePath,
							};

							let result: any;

							if (options.overwrite && !file.path) {
								// 说明：跳过压缩的图片在“覆盖原图”模式下不需要写回文件，允许无 path 直接完成
								if (wasSkipped) {
									okCount += 1;
									this.progressDone += 1;
									return;
								}
								// Try to fallback if path is missing but sourceFile is present (shouldn't happen with our fix)
								// But if it does happen, we can't overwrite because we don't know where to write.
								// However, the error message is what we want to avoid if possible.
								// Actually, if file.path is missing, it means we don't know the original location.
								// So we cannot overwrite it.
								this.updateFile(file.id, {
									status: "error",
									error: "Requires a local file path. ",
								});
								errCount += 1;
								this.progressDone += 1;
								return;
							}

							if (wasSkipped) {
								if (options.overwrite) {
									okCount += 1;
									this.progressDone += 1;
									return;
								}

								if (file.path) {
									result = await window.electron.invoke(
										"copy-image-to-output",
										{
											filePath: file.path,
											options: compressOptions,
										}
									);
								} else if (file.sourceFile) {
									const ab = await file.sourceFile.arrayBuffer();
									result = await window.electron.invoke("save-file-buffer", {
										fileName: file.name,
										buffer: ab,
										options: compressOptions,
									});
								} else {
									this.updateFile(file.id, {
										status: "error",
										error: "Cannot read file path or content",
									});
									errCount += 1;
									this.progressDone += 1;
									return;
								}
							} else {
								if (file.path) {
									result = await window.electron.invoke("compress-image", {
										filePath: file.path,
										options: compressOptions,
									});
								} else if (file.sourceFile) {
									const ab = await file.sourceFile.arrayBuffer();
									result = await window.electron.invoke(
										"compress-image-buffer",
										{
											fileName: file.name,
											buffer: ab,
											options: compressOptions,
										}
									);
								} else {
									this.updateFile(file.id, {
										status: "error",
										error: "Cannot read file path or content",
									});
									errCount += 1;
									this.progressDone += 1;
									return;
								}
							}

							if (result.success) {
								okCount += 1;
								this.updateFile(file.id, {
									status: wasSkipped ? "skipped" : "completed",
									compressedSize: wasSkipped
										? file.compressedSize
										: result.compressedSize,
									outputPath: result.outputPath,
								});
								this.progressDone += 1;
							} else {
								const errMsg =
									typeof result?.error === "string" ? result.error : "";
								const isUnsupported =
									errMsg.includes("Only PNG/JPG supported") ||
									errMsg.toLowerCase().includes("unsupported");

								// 说明：压缩不支持时，按“原样同步保存/拷贝”处理，避免整批失败
								if (isUnsupported) {
									if (options.overwrite) {
										okCount += 1;
										this.updateFile(file.id, {
											status: "completed",
											compressedSize: file.originalSize,
											outputPath: file.path,
											error: undefined,
										});
										this.progressDone += 1;
										return;
									}

									let fallback: any;
									if (file.path) {
										fallback = await window.electron.invoke(
											"copy-image-to-output",
											{
												filePath: file.path,
												options: compressOptions,
											}
										);
									} else if (file.sourceFile) {
										const ab = await file.sourceFile.arrayBuffer();
										fallback = await window.electron.invoke(
											"save-file-buffer",
											{
												fileName: file.name,
												buffer: ab,
												options: compressOptions,
											}
										);
									}

									if (fallback?.success) {
										okCount += 1;
										this.updateFile(file.id, {
											status: "completed",
											compressedSize: file.originalSize,
											outputPath: fallback.outputPath,
											error: undefined,
										});
										this.progressDone += 1;
										return;
									}
								}

								errCount += 1;
								this.updateFile(file.id, {
									status: "error",
									error: result.error,
								});
								this.progressDone += 1;
								return;
							}
						} else {
							// Dev mode / Browser fallback
							console.warn("Electron IPC not available");
							await new Promise((r) => setTimeout(r, 1000));
							this.updateFile(file.id, {
								status: "error",
								error: "Electron not available",
							});
							errCount += 1;
							this.progressDone += 1;
						}
					} catch (e) {
						this.updateFile(file.id, { status: "error", error: String(e) });
						errCount += 1;
						this.progressDone += 1;
					}
				});
			}
			return Promise.resolve();
		});

		await Promise.all(tasks);

		this.processing = false;
		toast.dismiss(toastId);
		if (okCount > 0 && errCount === 0) {
			toast.success(`Compression completed: ${okCount} successful`);
		} else if (okCount > 0 && errCount > 0) {
			toast(
				`Compression completed: ${okCount} successful, ${errCount} failed`,
				{
					duration: 5000,
				}
			);
		} else {
			toast.error("No files to compress or all failed");
		}
	}

	get pendingCount() {
		return this.files.filter((f) => f.status === "pending").length;
	}

	get progressPct() {
		const total = this.progressTotal;
		if (total <= 0) return 0;
		const done = Math.min(this.progressDone, total);
		return (done / total) * 100;
	}
}

export const appState = new AppState();
