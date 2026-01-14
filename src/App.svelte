<script lang="ts">
	import DropZone from "./lib/components/DropZone.svelte";
	import Settings from "./lib/components/Settings.svelte";
	import FileList from "./lib/components/FileList.svelte";
	import { appState } from "./lib/state.svelte";
	import { Button, Spinner } from "flowbite-svelte";
	import { Toaster } from "svelte-french-toast";
	import { PlayOutline, TrashBinOutline } from "flowbite-svelte-icons";
	import { onMount, onDestroy } from "svelte";
	import toast from "svelte-french-toast";

	let cleanupSystemThemeListener: (() => void) | undefined;
	let cleanupWindowResizeListener: (() => void) | undefined;

	onMount(() => {
		// Listen for window resize
		if (window.electron?.on) {
			const handleResize = (
				_: any,
				size: { width: number; height: number }
			) => {
				toast(`Window size: ${size.width}x${size.height}`, {
					duration: 2000,
					icon: "📏",
				});
			};

			window.electron.on("window-resized", handleResize);
			cleanupWindowResizeListener = () => {
				window.electron.off("window-resized", handleResize);
			};
		}

		// 说明：去掉手动切换按钮与本地存储，默认跟随系统暗黑/浅色模式，并监听系统模式变化。
		const media = window.matchMedia?.("(prefers-color-scheme: dark)");
		if (!media) return;

		const apply = () => {
			document.documentElement.classList.toggle("dark", media.matches);
		};

		apply();

		const handler = () => apply();
		if (typeof media.addEventListener === "function") {
			media.addEventListener("change", handler);
			cleanupSystemThemeListener = () =>
				media.removeEventListener("change", handler);
		} else {
			(media as any).addListener?.(handler);
			cleanupSystemThemeListener = () =>
				(media as any).removeListener?.(handler);
		}
	});

	onDestroy(() => {
		cleanupSystemThemeListener?.();
		cleanupWindowResizeListener?.();
	});

	const totalCount = $derived(appState.files.length);
	const pendingCount = $derived(
		appState.files.filter((f) => f.status === "pending").length
	);
	const completedCount = $derived(
		appState.files.filter((f) => f.status === "completed").length
	);
	const skippedCount = $derived(
		appState.files.filter((f) => f.status === "skipped").length
	);
	const errorCount = $derived(
		appState.files.filter((f) => f.status === "error").length
	);

	const completedOriginalBytes = $derived(
		appState.files.reduce(
			(sum, f) =>
				f.status === "completed" && f.compressedSize
					? sum + f.originalSize
					: sum,
			0
		)
	);
	const savedBytes = $derived(
		appState.files.reduce(
			(sum, f) =>
				f.status === "completed" && f.compressedSize
					? sum + (f.originalSize - f.compressedSize)
					: sum,
			0
		)
	);
	const savedPct = $derived(
		completedOriginalBytes > 0 ? (savedBytes / completedOriginalBytes) * 100 : 0
	);

	function formatBytes(bytes: number, decimals = 2) {
		if (!+bytes) return "0 Bytes";
		const k = 1024;
		const dm = decimals < 0 ? 0 : decimals;
		const sizes = ["Bytes", "KB", "MB", "GB"];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
	}
</script>

<Toaster position="top-center" toastOptions={{ style: "text-align:center;" }} />

<main
	class="h-dvh bg-gradient-to-b from-gray-50 to-white text-gray-900 dark:from-gray-950 dark:to-gray-900 dark:text-white"
>
	<div class="mx-auto flex h-full max-w-6xl flex-col gap-4 p-4">
		<div class="min-h-0 flex-1">
			<div
				class="flex h-full min-h-0 flex-col gap-4 lg:flex-row lg:justify-between"
			>
				<!-- 说明：左右两侧固定容器高度，内容在内部滚动，避免撑高外层 -->
				<div class="h-full min-h-0 w-full overflow-hidden lg:flex-[4_1_0%]">
					<div class="flex h-full min-h-0 flex-col gap-4">
						{#if totalCount === 0}
							<!-- 说明：空状态下拖拽区域撑满左侧容器高度 -->
							<div
								class="flex h-full w-full flex-col rounded-xl border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-900 sm:p-5"
							>
								<div class="text-base font-semibold">导入</div>
								<div class="mt-4 min-h-0 flex-1">
									<DropZone class="h-full w-full" />
								</div>
							</div>
						{:else}
							<div class="flex min-h-0 w-full flex-1 flex-col rounded-xl">
								<FileList class="h-full w-full" />
							</div>
						{/if}
					</div>
				</div>

				<div class="h-full min-h-0 w-full overflow-hidden lg:flex-[1_1_0%]">
					<div
						class="h-full min-h-0 w-full overflow-hidden rounded-xl border border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-900"
					>
						<!-- Note: The right container scrolls internally, and the outer layer has a fixed height -->
						<div
							class="h-full w-full overflow-y-auto p-4 scrollbar-hidden sm:p-5"
						>
							<div>
								<div class="text-base font-semibold">Settings</div>
							</div>

							<div class="mt-4 grid grid-cols-1 gap-3">
								<div
									class="rounded-lg border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
								>
									<div class="text-xs text-gray-500 dark:text-gray-400">
										Completed
									</div>
									<div class="mt-1 text-xl font-semibold">{completedCount}</div>
								</div>
								<div
									class="rounded-lg border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
								>
									<div class="text-xs text-gray-500 dark:text-gray-400">
										Pending
									</div>
									<div class="mt-1 text-xl font-semibold">{pendingCount}</div>
								</div>
								<div
									class="rounded-lg border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
								>
									<div class="text-xs text-gray-500 dark:text-gray-400">
										Saved Size
									</div>
									<div class="mt-1 text-xl font-semibold">
										{formatBytes(savedBytes, 1)}
									</div>
								</div>
								<div
									class="rounded-lg border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
								>
									<div class="text-xs text-gray-500 dark:text-gray-400">
										Saved Ratio
									</div>
									<div class="mt-1 text-xl font-semibold">
										{savedPct.toFixed(1)}%
									</div>
								</div>
							</div>

							<div class="mt-4 flex flex-col gap-2">
								<!-- Note: Button style is synchronized with "Clear All", unified using outline style -->
								<Button
									outline
									class="w-full !border-blue-600 !text-blue-600 hover:!bg-blue-600 hover:!text-white dark:!border-blue-500 dark:!text-blue-400 dark:hover:!bg-blue-500 dark:hover:!text-white"
									disabled={appState.processing || appState.pendingCount === 0}
									onclick={() => appState.startCompression()}
								>
									{#if appState.processing}
										<Spinner size="4" class="me-2" />
										Processing...
									{:else}
										<PlayOutline class="me-2 h-5 w-5" />
										开始压缩
									{/if}
								</Button>

								<Button
									color="red"
									outline
									class="w-full"
									disabled={appState.processing || appState.files.length === 0}
									onclick={() => appState.clearAll()}
								>
									<TrashBinOutline class="me-2 h-5 w-5" />
									清空全部
								</Button>
							</div>

							<hr class="my-5 border-gray-200 dark:border-gray-700" />

							<Settings />
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
</main>
