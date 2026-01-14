<script lang="ts">
	import { appState } from "../state.svelte";
	import { Dropzone } from "flowbite-svelte";
	import { UploadOutline } from "flowbite-svelte-icons";

	// Note: Allow parent component to pass in class to control placeholder/height of the drop zone (e.g. fill container)
	let { class: className = "" } = $props<{ class?: string }>();

	let isDragging = $state(false);
	let useElectronPicker = $state(false);

	$effect(() => {
		useElectronPicker = typeof window !== "undefined" && !!window.electron;
	});

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

	function isImageFile(file: File) {
		if (file.type && file.type.startsWith("image/")) return true;
		const dot = file.name.lastIndexOf(".");
		if (dot === -1) return false;
		const ext = file.name.slice(dot).toLowerCase();
		return IMAGE_EXTS.has(ext);
	}

	async function traverseEntry(
		entry: any,
		out: (File | { file: File; relativePath: string })[],
		parentPath?: string
	) {
		if (!entry) return;
		if (entry.isFile) {
			const file = await new Promise<File>((resolve, reject) => {
				entry.file(resolve, reject);
			});
			if (isImageFile(file)) {
				const fullPath =
					typeof entry.fullPath === "string" && entry.fullPath.length > 0
						? entry.fullPath
						: `/${file.name}`;
				let relativePath = fullPath.startsWith("/")
					? fullPath.slice(1)
					: fullPath;
				const parts = relativePath.split("/").filter(Boolean);
				if (parts.length > 1) {
					relativePath = parts.slice(1).join("/");
				}

				// Try to reconstruct absolute path if parentPath is provided
				if (parentPath && window.electron) {
					// entry.fullPath usually starts with / and includes the dropped folder name
					// parentPath is the absolute path of the dropped folder (without trailing slash)
					// entry.fullPath: /DroppedFolder/sub/file.png
					// We want: /path/to/DroppedFolder/sub/file.png
					// However, entry.fullPath includes the dropped folder name as the first segment.
					// parentPath is the path TO the dropped folder.
					// So if we dropped /Users/me/Photos, parentPath is /Users/me/Photos
					// entry.fullPath is /Photos/img.png
					// Wait, if we drop a folder, the root entry name IS the folder name.
					// So if we drop /Users/me/Photos, the root entry name is "Photos".
					// entry.fullPath is /Photos/img.png
					// So we need the PARENT of the dropped folder to construct the full path?
					// OR, we can just use the fact that we know the absolute path of the root entry.
					// Let's change the logic: pass the 'rootAbsolutePath' and 'rootEntryName'.
				}

				// If we have parentPath (which is the absolute path of the directory containing this file), we can set .path
				if (parentPath) {
					// But we need to be careful about matching the structure.
					// Let's solve this in collectDroppedFiles instead.
				}

				out.push({ file, relativePath });
			}
			return;
		}
		if (entry.isDirectory) {
			const reader = entry.createReader();
			while (true) {
				const entries: any[] = await new Promise((resolve, reject) => {
					reader.readEntries(resolve, reject);
				});
				if (!entries || entries.length === 0) break;
				for (const child of entries) {
					await traverseEntry(child, out);
				}
			}
		}
	}

	async function collectDroppedFiles(e: DragEvent) {
		const dt = e.dataTransfer;
		if (!dt) return [] as (File | { file: File; relativePath: string })[];

		const rootPathsFromUriList: string[] = [];
		try {
			const uriList = dt.getData("text/uri-list") || dt.getData("text/plain");
			if (typeof uriList === "string" && uriList.length > 0) {
				for (const rawLine of uriList.split(/\r?\n/)) {
					const line = rawLine.trim();
					if (!line || line.startsWith("#")) continue;
					if (!line.startsWith("file://")) continue;
					const withoutScheme = line.replace(/^file:\/\//, "");
					const decoded = decodeURIComponent(withoutScheme);
					const normalized =
						decoded.startsWith("/") && /^\/[a-zA-Z]:\//.test(decoded)
							? decoded.slice(1)
							: decoded;
					rootPathsFromUriList.push(normalized);
				}
			}
		} catch {}

		// 说明：在 Electron 里拖拽文件夹时，dt.files 往往已经包含所有子文件且自带绝对路径（File.path）
		// 优先走这条路径，避免 webkitGetAsEntry().file() 生成的 File 丢失 path 导致覆盖保存失败
		if (window.electron) {
			const allFiles = Array.from(dt.files ?? []).filter(isImageFile);
			const paths = allFiles
				.map((f) => {
					const p =
						typeof window.electron.getPathForFile === "function"
							? window.electron.getPathForFile(f)
							: ((f as any)?.path as string);
					if (typeof p === "string" && p.length > 0) {
						Object.defineProperty(f, "path", {
							value: p,
							writable: true,
							configurable: true,
						});
					}
					return p;
				})
				.filter((p) => typeof p === "string" && p.length > 0);

			if (allFiles.length > 0 && paths.length > 0) {
				const pathByNameSize = new Map<string, string>();
				for (const f of allFiles) {
					const p =
						typeof window.electron.getPathForFile === "function"
							? window.electron.getPathForFile(f)
							: (f as any)?.path;
					if (typeof p === "string" && p.length > 0) {
						const key = `${f.name}:${f.size}`;
						if (!pathByNameSize.has(key)) pathByNameSize.set(key, p);
					}
				}

				const sep = paths.some((p) => p.includes("\\")) ? "\\" : "/";

				const dirname = (p: string) => {
					const normalized = p.endsWith(sep) ? p.slice(0, -1) : p;
					const idx = normalized.lastIndexOf(sep);
					if (idx <= 0) return normalized;
					return normalized.slice(0, idx);
				};

				const basename = (p: string) => {
					const normalized = p.endsWith(sep) ? p.slice(0, -1) : p;
					const idx = normalized.lastIndexOf(sep);
					return idx === -1 ? normalized : normalized.slice(idx + 1);
				};

				const splitSegments = (p: string) => p.split(sep).filter(Boolean);
				const joinSegments = (segs: string[]) => {
					if (segs.length === 0) return "";
					if (sep === "/") return `/${segs.join("/")}`;
					if (segs.length === 1 && /^[a-zA-Z]:$/.test(segs[0]))
						return `${segs[0]}\\`;
					return segs.join("\\");
				};

				const commonDir = (() => {
					const dirSegsList = paths.map((p) => splitSegments(dirname(p)));
					let common = dirSegsList[0] ?? [];
					for (const segs of dirSegsList.slice(1)) {
						let i = 0;
						while (
							i < common.length &&
							i < segs.length &&
							common[i] === segs[i]
						) {
							i += 1;
						}
						common = common.slice(0, i);
						if (common.length === 0) break;
					}
					return joinSegments(common);
				})();

				const toRelative = (fullPath: string) => {
					if (!commonDir) return basename(fullPath);
					const prefix = commonDir.endsWith(sep)
						? commonDir
						: `${commonDir}${sep}`;
					if (fullPath.startsWith(prefix)) return fullPath.slice(prefix.length);
					return basename(fullPath);
				};

				return allFiles.map((file) => {
					let p = (file as any)?.path as string;
					if (typeof p !== "string" || p.length === 0) {
						const inferred = pathByNameSize.get(`${file.name}:${file.size}`);
						if (typeof inferred === "string" && inferred.length > 0) {
							Object.defineProperty(file, "path", {
								value: inferred,
								writable: true,
								configurable: true,
							});
							p = inferred;
						}
					}
					if (typeof p === "string" && p.length > 0) {
						return { file, relativePath: toRelative(p) };
					}
					return { file, relativePath: file.name };
				});
			}
		}

		const items = Array.from(dt.items ?? []);
		const hasEntry = items.some(
			(it: any) => typeof it.webkitGetAsEntry === "function"
		);

		if (!hasEntry) {
			return Array.from(dt.files ?? []).filter(isImageFile);
		}

		// 说明：Electron 下拖拽文件/文件夹时，dt.files 里往往带有系统绝对路径（File.path），但 webkitGetAsEntry().file() 产生的 File 可能会丢失该属性
		const fallbackRootPathByName = new Map<string, string>();
		const fallbackRootAllPaths: string[] = [];
		for (const p of rootPathsFromUriList) {
			if (typeof p === "string" && p.length > 0) {
				const base = p.split(/[\\/]/).filter(Boolean).slice(-1)[0];
				if (base && !fallbackRootPathByName.has(base)) {
					fallbackRootPathByName.set(base, p);
				}
				fallbackRootAllPaths.push(p);
			}
		}
		if (window.electron) {
			for (const f of Array.from(dt.files ?? [])) {
				const p =
					typeof window.electron.getPathForFile === "function"
						? window.electron.getPathForFile(f)
						: (f as any)?.path;
				if (
					typeof p === "string" &&
					p.length > 0 &&
					!fallbackRootPathByName.has(f.name)
				) {
					fallbackRootPathByName.set(f.name, p);
					fallbackRootAllPaths.push(p);
				}
			}
		}

		const out: (File | { file: File; relativePath: string })[] = [];
		for (const item of items as any[]) {
			const entry = item.webkitGetAsEntry?.();
			if (!entry) continue;

			// Get the corresponding file object to find the absolute path (Electron only)
			let rootPath = "";
			if (window.electron) {
				const f = item.getAsFile?.();
				if (f) {
					const p =
						typeof window.electron.getPathForFile === "function"
							? window.electron.getPathForFile(f)
							: (f as any).path;
					if (typeof p === "string" && p.length > 0) rootPath = p;
				}
				// 说明：有些情况下（尤其拖拽文件夹），getAsFile() 可能拿不到 path，这里用 dt.files 里同名项兜底
				if (!rootPath) {
					const fallback = fallbackRootPathByName.get(entry.name);
					if (typeof fallback === "string" && fallback.length > 0) {
						rootPath = fallback;
					}
				}
				if (!rootPath && fallbackRootAllPaths.length > 0) {
					for (const p of fallbackRootAllPaths) {
						const sep = p.includes("\\") ? "\\" : "/";
						const needleMid = `${sep}${entry.name}${sep}`;
						const midIdx = p.indexOf(needleMid);
						if (midIdx !== -1) {
							rootPath = p.slice(0, midIdx + sep.length + entry.name.length);
							break;
						}
						const needleEnd = `${sep}${entry.name}`;
						if (p.endsWith(needleEnd)) {
							rootPath = p;
							break;
						}
					}
				}
			}

			// We need a custom traverse function that can attach paths
			await traverseEntryWithRoot(entry, out, rootPath);
		}
		return out;
	}

	async function traverseEntryWithRoot(
		entry: any,
		out: (File | { file: File; relativePath: string })[],
		absolutePath: string // The absolute path of the current entry
	) {
		if (!entry) return;

		if (entry.isFile) {
			const file = await new Promise<File>((resolve, reject) => {
				entry.file(resolve, reject);
			});

			if (isImageFile(file)) {
				// If we have absolute path, attach it to the file object
				if (absolutePath && window.electron) {
					// NOTE: We MUST ensure the path is correct.
					// When traversing, 'absolutePath' passed here IS the full path of this file.
					Object.defineProperty(file, "path", {
						value: absolutePath,
						writable: true,
						configurable: true,
					});
				}

				// Calculate relative path for display
				// entry.fullPath is like /Root/sub/file.png
				// We want sub/file.png

				const fullPath =
					typeof entry.fullPath === "string" && entry.fullPath.length > 0
						? entry.fullPath
						: `/${file.name}`;

				let relativePath = fullPath.startsWith("/")
					? fullPath.slice(1)
					: fullPath;

				const parts = relativePath.split("/").filter(Boolean);
				if (parts.length > 1) {
					relativePath = parts.slice(1).join("/");
				}

				out.push({ file, relativePath });
			}
			return;
		}

		if (entry.isDirectory) {
			const reader = entry.createReader();
			while (true) {
				const entries: any[] = await new Promise((resolve, reject) => {
					reader.readEntries(resolve, reject);
				});
				if (!entries || entries.length === 0) break;

				for (const child of entries) {
					// Construct child absolute path
					// If parent is /User/me/Photos, child 'sub' is /User/me/Photos/sub
					let childPath = "";
					if (absolutePath) {
						// Simple path join for now, assuming forward slashes or handling by OS
						// But Electron usually runs on node, so we might need proper separator.
						// However, in browser context we don't have 'path' module easily.
						// We can guess based on the input path separator.

						// FIX: Check if absolutePath ends with separator to avoid double slash
						// or missing slash.
						const sep = absolutePath.includes("\\") ? "\\" : "/";

						// If absolutePath is just "/" (root), we shouldn't add another slash if using /
						// But typically paths are like /Users/foo

						if (absolutePath.endsWith(sep)) {
							childPath = `${absolutePath}${child.name}`;
						} else {
							childPath = `${absolutePath}${sep}${child.name}`;
						}
					}

					await traverseEntryWithRoot(child, out, childPath);
				}
			}
		}
	}

	async function handleDrop(e: DragEvent) {
		e.preventDefault();
		isDragging = false;
		const files = await collectDroppedFiles(e);
		if (files.length > 0) {
			if (window.electron) {
				const pathEntries = files
					.map((item) => {
						const file = item instanceof File ? item : item.file;
						const relativePath =
							item instanceof File ? file.name : item.relativePath;

						// In Electron environment, if we traverse directory entries, the File object created
						// by entry.file() might NOT have the 'path' property set correctly (it might be empty string).
						// However, FileSystemFileEntry usually has fullPath property, but it's relative to the root of the file system?
						// Wait, actually for Electron drag-and-drop, we need to handle it carefully.

						// If 'path' is missing on the File object (common when using entry.file()),
						// we cannot easily recover the full system path just from the File object itself in standard web API.
						// BUT in Electron, usually `File.path` IS available if it comes from drag and drop.
						// The issue is when we use `webkitGetAsEntry` -> `entry.file()`, the resulting File object
						// loses the original path property that the top-level dropped file had.

						// FIX: We need to manually construct the path or ensure we're getting it.
						// When using entry.file(), the resulting file object DOES NOT have the full system path attached to it as .path property.
						// This is a known limitation/behavior.

						// HOWEVER, we can use a workaround:
						// If we are in Electron, we should try to use the path if available.
						let p = (file as any)?.path;

						// If p is missing, it means we are likely dealing with a file from a folder traversal.
						// In this case, we need to know the parent directory's path to reconstruct the full path.
						// But we don't have it easily here.

						// Alternative approach:
						// When traversing, we can pass down the full path if we knew the root path.
						// But `webkitGetAsEntry` gives us an Entry, which has `fullPath` (relative to the drop root).
						// It does NOT give us the absolute system path of the drop root.

						// WAIT! In Electron, `DataTransferItem.getAsFile()?.path` gives the full path of the dropped item.
						// So for the root items, we know their full paths.
						// `entry` also has `name` and `isDirectory`/`isFile`.

						// We need to modify `collectDroppedFiles` to preserve full paths during traversal if possible.
						// But `collectDroppedFiles` uses standard web APIs.

						// Let's modify `collectDroppedFiles` to try to attach full path if possible,
						// or we need to change how we traverse.

						// Actually, there is a simpler way for Electron?
						// No, recursive traversal is required for folders.

						// The only way to get the full path of a file inside a dropped folder in Electron
						// is to know the full path of the dropped folder, and then append the relative path.
						// The `entry.fullPath` gives path relative to the drop root (starting with /).

						// So we need to map the root entries to their system paths.

						if (typeof p !== "string" || p.length === 0) {
							// Try to use the 'path' from the item if it was a direct file
							// But here 'item' structure in 'files' array is: { file: File, relativePath: string }
							// If it was a direct file, file.path should be there.
							// If it was from traversal, file.path is likely missing.

							// If we can't find the path, we can't process it in Electron backend.
							// We must fix `collectDroppedFiles` or `traverseEntry` to carry the path.
							return null;
						}

						return {
							name: file.name,
							path: p,
							size: file.size,
							relativePath,
						};
					})
					.filter((x): x is NonNullable<typeof x> => x !== null);

				const bufferEntries = files.filter((item) => {
					const file = item instanceof File ? item : item.file;
					const p = (file as any)?.path;
					return typeof p !== "string" || p.length === 0;
				});

				if (pathEntries.length > 0) appState.addFileEntries(pathEntries);
				if (bufferEntries.length > 0) appState.addFiles(bufferEntries);
				return;
			}

			appState.addFiles(files);
		}
	}

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		isDragging = true;
	}

	function handleDragLeave() {
		isDragging = false;
	}

	async function handleSelect() {
		if (window.electron) {
			const entries = await window.electron.invoke("select-files");
			if (Array.isArray(entries) && entries.length > 0) {
				appState.addFileEntries(entries);
			}
			return;
		}
	}

	function handleInput(e: Event) {
		const input = e.target as HTMLInputElement;
		if (input.files) {
			const files = Array.from(input.files).map((f) => ({
				file: f,
				relativePath: f.name,
			}));
			appState.addFiles(files);
			input.value = "";
		}
	}

	function handleOverlayKeydown(e: KeyboardEvent) {
		if (e.key === "Enter" || e.key === " ") {
			e.preventDefault();
			handleSelect();
		}
	}
</script>

<div
	class={`relative ${className}`}
	role="presentation"
	ondragleave={handleDragLeave}
>
	<Dropzone
		multiple
		accept="image/*"
		class="h-full w-full cursor-pointer"
		disabled={useElectronPicker}
		onDrop={handleDrop}
		onDragOver={handleDragOver}
		onChange={handleInput}
	>
		<div
			class="flex h-full w-full flex-col items-center justify-center gap-3 rounded-lg border-2 border-dashed p-10 text-center"
			class:ring-2={isDragging}
			class:ring-blue-500={isDragging}
		>
			<UploadOutline class="h-10 w-10 text-gray-500 dark:text-gray-400" />
			<div class="space-y-1">
				<h3 class="text-base font-semibold">Drop PNG or JPG files here</h3>
			</div>
		</div>
	</Dropzone>

	{#if useElectronPicker}
		<div
			class="absolute inset-0 cursor-pointer"
			role="button"
			tabindex="0"
			aria-label="Select file or folder"
			onclick={handleSelect}
			onkeydown={handleOverlayKeydown}
			ondrop={handleDrop}
			ondragover={handleDragOver}
			ondragleave={handleDragLeave}
		></div>
	{/if}
</div>
