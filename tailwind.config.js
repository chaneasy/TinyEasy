import flowbitePlugin from "flowbite/plugin";

export default {
	content: [
		"./index.html",
		"./src/**/*.{html,js,svelte,ts}",
		"./node_modules/flowbite/**/*.js",
		"./node_modules/flowbite-svelte/**/*.{html,js,svelte,ts}",
	],
	darkMode: "class",
	theme: {
		extend: {},
	},
	plugins: [flowbitePlugin],
};
