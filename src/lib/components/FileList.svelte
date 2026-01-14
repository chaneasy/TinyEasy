<script lang="ts">
	import { appState } from "../state.svelte";
	import { Badge, Card } from "flowbite-svelte";
	import { onDestroy } from "svelte";

	let { class: className = "" } = $props<{ class?: string }>();

	let objectUrls = $state<Record<string, string>>({});

	let lastObjectUrls: Record<string, string> = {};

	$effect(() => {
		const next: Record<string, string> = {};

		for (const file of appState.files) {
			const existing = lastObjectUrls[file.id];
			if (existing) {
				next[file.id] = existing;
				continue;
			}
			if (file.sourceFile) {
				next[file.id] = URL.createObjectURL(file.sourceFile);
			}
		}

		for (const [id, url] of Object.entries(lastObjectUrls)) {
			if (!next[id]) URL.revokeObjectURL(url);
		}

		lastObjectUrls = next;
		objectUrls = next;
	});

	onDestroy(() => {
		for (const url of Object.values(lastObjectUrls)) URL.revokeObjectURL(url);
	});

	function formatBytes(bytes: number, decimals = 2) {
		if (!+bytes) return "0 Bytes";
		const k = 1024;
		const dm = decimals < 0 ? 0 : decimals;
		const sizes = ["Bytes", "KB", "MB", "GB"];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
	}

	function statusBadge(status: string) {
		switch (status) {
			case "pending":
				return { color: "gray", text: "Pending" };
			case "compressing":
				return { color: "yellow", text: "Processing" };
			case "completed":
				return { color: "green", text: "Done" };
			case "skipped":
				return { color: "gray", text: "Skipped" };
			default:
				return { color: "red", text: "Failed" };
		}
	}

	function toFileUrl(filePath: string) {
		const normalized = filePath.replace(/\\/g, "/");
		if (normalized.startsWith("file://")) return normalized;
		if (normalized.startsWith("/")) return `file://${encodeURI(normalized)}`;
		return `file:///${encodeURI(normalized)}`;
	}

	function previewSrc(file: any) {
		const url = objectUrls[file.id];
		if (url) return url;
		if (typeof file.path === "string" && file.path.length > 0)
			return toFileUrl(file.path);
		return "";
	}

	function fileExt(name: string) {
		const dot = name.lastIndexOf(".");
		return dot === -1 ? "" : name.slice(dot + 1).toUpperCase();
	}

	function compressionPct(originalSize: number, compressedSize?: number) {
		if (!compressedSize || originalSize <= 0) return null;
		return (1 - compressedSize / originalSize) * 100;
	}

	const completedCount = $derived(
		appState.files.filter((f) => f.status === "completed").length
	);

	function getCardBgClass(status: string) {
		switch (status) {
			case "completed":
				return "bg-green-50 dark:bg-green-900/20";
			case "error":
				return "bg-red-50 dark:bg-red-900/20";
			case "skipped":
				return "bg-gray-50 dark:bg-gray-800/50";
			default:
				return "bg-white dark:bg-gray-800";
		}
	}
</script>

<div
	class={`flex w-full flex-col overflow-hidden rounded-xl border border-gray-200 bg-white p-0 dark:border-gray-700 dark:bg-gray-900 ${className}`}
>
	<div
		class="flex shrink-0 items-center justify-between gap-4 border-b border-gray-200 px-4 py-3 dark:border-gray-700"
	>
		<div class="text-base font-semibold">Images</div>
		<div class="flex items-center gap-3">
			<div class="text-sm text-gray-500 dark:text-gray-400">
				Total {appState.files.length}
			</div>
		</div>
	</div>

	<div class="flex-1 overflow-y-auto p-4 scrollbar-hidden">
		{#if appState.files.length === 0}
			<div
				class="rounded-lg border border-gray-200 bg-white p-10 text-center text-sm text-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400"
			>
				No images, please import above
			</div>
		{:else}
			<div
				class="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-5"
			>
				{#each appState.files as file (file.id)}
					<Card
						class={`relative w-full overflow-hidden p-2 transition-colors duration-300 ${getCardBgClass(file.status)}`}
						padding="none"
						shadow="sm"
					>
						{#if file.status === "compressing"}
							<div
								class="animate-progress absolute bottom-0 left-0 right-0 z-0 bg-green-50 dark:bg-green-900/20"
							></div>
						{/if}
						<div class="relative z-10 flex flex-col gap-2 p-2">
							<div
								class="h-24 w-full shrink-0 overflow-hidden rounded-lg border border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800"
							>
								{#if previewSrc(file)}
									<img
										class="h-full w-full object-contain"
										src={previewSrc(file)}
										alt={file.name}
									/>
								{:else}
									<div
										class="flex h-full w-full items-center justify-center text-xs font-semibold text-gray-500 dark:text-gray-400"
									>
										{fileExt(file.name)}
									</div>
								{/if}
							</div>

							<div
								class="flex min-w-0 flex-1 flex-col items-center text-center"
							>
								<div class="w-full min-w-0">
									<div
										class="truncate text-sm font-medium"
										title={file.path ?? ""}
									>
										{file.name}
									</div>
								</div>

								<div class="mt-1 flex shrink-0 flex-col items-center gap-1">
									<div class="text-[10px] text-gray-500 dark:text-gray-400">
										{formatBytes(file.originalSize, 1)}
										{#if file.compressedSize}
											<span class="mx-0.5">→</span>
											{formatBytes(file.compressedSize, 1)}
										{/if}
									</div>
									<Badge color={statusBadge(file.status).color as any} size="sm"
										>{statusBadge(file.status).text}</Badge
									>
									{#if compressionPct(file.originalSize, file.compressedSize) !== null}
										{#if (compressionPct(file.originalSize, file.compressedSize) as number) >= 0}
											<div
												class="text-xs font-semibold text-green-600 dark:text-green-400"
											>
												-{Math.abs(
													compressionPct(
														file.originalSize,
														file.compressedSize
													) as number
												).toFixed(1)}%
											</div>
										{:else}
											<div
												class="text-xs font-semibold text-yellow-600 dark:text-yellow-400"
											>
												+{Math.abs(
													compressionPct(
														file.originalSize,
														file.compressedSize
													) as number
												).toFixed(1)}%
											</div>
										{/if}
									{/if}
								</div>

								{#if file.status === "error" && file.error}
									<div
										class="mt-1 w-full truncate text-[10px] text-red-500 dark:text-red-400"
										title={file.error}
									>
										{file.error}
									</div>
								{/if}
							</div>
						</div>
					</Card>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	@keyframes progress-fill {
		0% {
			height: 0%;
		}
		100% {
			height: 100%;
		}
	}
	.animate-progress {
		animation: progress-fill 2s ease-in-out infinite;
	}
</style>
