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
}

interface SelectedFileEntry {
	name: string;
	path: string;
	size: number;
}

import { toast } from "svelte-french-toast";

class AppState {
	files = $state<CompressionFile[]>([]);
	settings = $state<Settings>({
		level: "medium",
	});
	processing = $state(false);

	addFiles(newFiles: FileList | File[]) {
		// Note: Supports dragging files/folders; only accepts "image files", and marks non-PNG images as skipped (displayed but not compressed).
		const fileArray = Array.from(newFiles);
		const newEntries = fileArray.map((file) => {
			const lowerName = file.name.toLowerCase();
			const isImageByMime = typeof file.type === "string" && file.type.startsWith("image/");
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
				error: isPng || isJpg ? undefined : "Non-PNG/JPG images are displayed only, not compressed",
			};
		});
		const filtered = newEntries.filter((x): x is NonNullable<typeof x> => x !== null);
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
				status: isPng || isJpg ? ("pending" as const) : isImageByExt ? ("skipped" as const) : ("error" as const),
				originalSize: entry.size,
				error: isPng || isJpg ? undefined : isImageByExt ? "Non-PNG/JPG images are displayed only, not compressed" : "Unsupported file type",
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

  async startCompression() {
    if (this.processing) return;
    this.processing = true;

    // Process one by one or parallel?
    // Requirement says "Worker thread", main process handles it.
    // We can fire multiple requests. Main process handles concurrency?
    // Our main process spawns a new Worker for EACH request currently.
    // So parallel is fine, but let's limit concurrency to avoid freezing UI or OOM.
    // For simplicity, let's do sequential or limited parallel (e.g. 2).
    // Sequential for now to be safe and simple.
    
		const toastId = toast.loading("Compressing...");
		let okCount = 0;
		let errCount = 0;

		for (const file of this.files) {
      if (file.status === 'pending') {
        this.updateFile(file.id, { status: 'compressing' });
        try {
          // Use window.electron if available
          if (window.electron) {
						const options = {
							level: this.settings.level,
							originalSize: file.originalSize,
						};

						let result: any;

						if (file.path) {
							result = await window.electron.invoke('compress-image', { filePath: file.path, options });
						} else if (file.sourceFile) {
							const ab = await file.sourceFile.arrayBuffer();
							result = await window.electron.invoke('compress-image-buffer', {
								fileName: file.name,
								buffer: ab,
								options,
							});
						} else {
							this.updateFile(file.id, { status: 'error', error: 'Cannot read file path or content' });
							errCount += 1;
							continue;
						}
            
            if (result.success) {
								okCount += 1;
               this.updateFile(file.id, { 
                 status: 'completed', 
                 compressedSize: result.compressedSize,
                 outputPath: result.outputPath
               });
            } else {
								errCount += 1;
               this.updateFile(file.id, { status: 'error', error: result.error });
            }
          } else {
              // Dev mode / Browser fallback
              console.warn('Electron IPC not available');
              await new Promise(r => setTimeout(r, 1000));
              this.updateFile(file.id, { status: 'error', error: 'Electron not available' });
							errCount += 1;
          }
        } catch (e) {
           this.updateFile(file.id, { status: 'error', error: String(e) });
					 errCount += 1;
        }
      }
    }
    this.processing = false;
		toast.dismiss(toastId);
		if (okCount > 0 && errCount === 0) {
			toast.success(`Compression completed: ${okCount} successful`);
		} else if (okCount > 0 && errCount > 0) {
			toast(`Compression completed: ${okCount} successful, ${errCount} failed`, { duration: 5000 });
		} else {
			toast.error("No files to compress or all failed");
		}
  }
  
  get pendingCount() {
      return this.files.filter(f => f.status === 'pending').length;
  }
}

export const appState = new AppState();
