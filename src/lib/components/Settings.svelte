<script lang="ts">
	import { appState } from "../state.svelte";
	import { Radio, Tooltip } from "flowbite-svelte";
	import { InfoCircleOutline } from "flowbite-svelte-icons";

	type Level = "low" | "medium" | "high";

	const LEVEL_CONFIG: Record<
		Level,
		{
			png: { opt: number; strip: "safe" | "all" };
			jpg: { max: number; stripAll: true; progressive: true };
		}
	> = {
		low: {
			png: { opt: 2, strip: "safe" },
			jpg: { max: 85, stripAll: true, progressive: true },
		},
		medium: {
			png: { opt: 4, strip: "safe" },
			jpg: { max: 60, stripAll: true, progressive: true },
		},
		high: {
			png: { opt: 6, strip: "all" },
			jpg: { max: 40, stripAll: true, progressive: true },
		},
	};
</script>

<div class="flex flex-col gap-4">
	<div class="space-y-2">
		<div class="text-sm font-medium">Compression Level</div>
		<div class="flex flex-col gap-2">
			<div class="flex items-center justify-between gap-3">
				<Radio name="level" value="low" bind:group={appState.settings.level}
					>Low</Radio
				>
				<div class="flex items-center">
					<button
						type="button"
						class="inline-flex items-center justify-center rounded-md p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:hover:bg-gray-800 dark:hover:text-gray-200"
						aria-label="View configuration for Low level"
					>
						<InfoCircleOutline class="h-4 w-4" />
					</button>
					<Tooltip placement="left">
						<div class="space-y-2 text-xs">
							<div>
								<div class="font-semibold">PNG</div>
								<div>
									oxipng --opt {LEVEL_CONFIG.low.png.opt} --strip {LEVEL_CONFIG
										.low.png.strip}
								</div>
							</div>
							<div>
								<div class="font-semibold">JPG</div>
								<div>
									jpegoptim --max {LEVEL_CONFIG.low.jpg.max} --strip-all --all-progressive
								</div>
							</div>
						</div>
					</Tooltip>
				</div>
			</div>
			<div class="flex items-center justify-between gap-3">
				<Radio name="level" value="medium" bind:group={appState.settings.level}
					>Medium</Radio
				>
				<div class="flex items-center">
					<button
						type="button"
						class="inline-flex items-center justify-center rounded-md p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:hover:bg-gray-800 dark:hover:text-gray-200"
						aria-label="View configuration for Medium level"
					>
						<InfoCircleOutline class="h-4 w-4" />
					</button>
					<Tooltip placement="left">
						<div class="space-y-2 text-xs">
							<div>
								<div class="font-semibold">PNG</div>
								<div>
									oxipng --opt {LEVEL_CONFIG.medium.png.opt} --strip {LEVEL_CONFIG
										.medium.png.strip}
								</div>
							</div>
							<div>
								<div class="font-semibold">JPG</div>
								<div>
									jpegoptim --max {LEVEL_CONFIG.medium.jpg.max} --strip-all --all-progressive
								</div>
							</div>
						</div>
					</Tooltip>
				</div>
			</div>
			<div class="flex items-center justify-between gap-3">
				<Radio name="level" value="high" bind:group={appState.settings.level}
					>High</Radio
				>
				<div class="flex items-center">
					<button
						type="button"
						class="inline-flex items-center justify-center rounded-md p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:hover:bg-gray-800 dark:hover:text-gray-200"
						aria-label="View configuration for High level"
					>
						<InfoCircleOutline class="h-4 w-4" />
					</button>
					<Tooltip placement="left">
						<div class="space-y-2 text-xs">
							<div>
								<div class="font-semibold">PNG</div>
								<div>
									oxipng --opt {LEVEL_CONFIG.high.png.opt} --strip {LEVEL_CONFIG
										.high.png.strip}
								</div>
							</div>
							<div>
								<div class="font-semibold">JPG</div>
								<div>
									jpegoptim --max {LEVEL_CONFIG.high.jpg.max} --strip-all --all-progressive
								</div>
							</div>
						</div>
					</Tooltip>
				</div>
			</div>
		</div>
	</div>
</div>
