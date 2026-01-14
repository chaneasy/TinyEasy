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

	async function traverseEntry(entry: any, out: File[]) {
		if (!entry) return;
		if (entry.isFile) {
			const file = await new Promise<File>((resolve, reject) => {
				entry.file(resolve, reject);
			});
			if (isImageFile(file)) out.push(file);
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
		if (!dt) return [] as File[];

		const items = Array.from(dt.items ?? []);
		const hasEntry = items.some(
			(it: any) => typeof it.webkitGetAsEntry === "function"
		);
		if (!hasEntry) {
			return Array.from(dt.files ?? []).filter(isImageFile);
		}

		const out: File[] = [];
		for (const item of items as any[]) {
			const entry = item.webkitGetAsEntry?.();
			if (!entry) continue;
			await traverseEntry(entry, out);
		}
		return out;
	}

	async function handleDrop(e: DragEvent) {
		e.preventDefault();
		isDragging = false;
		const files = await collectDroppedFiles(e);
		if (files.length > 0) {
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
			appState.addFiles(input.files);
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
				<h3 class="text-base font-semibold">Click or drag images/folders</h3>
				<p class="text-sm text-gray-500 dark:text-gray-400">(PNG and JPG only)</p>
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
