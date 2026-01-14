interface Window {
	electron: {
		invoke: (channel: string, ...args: any[]) => Promise<any>;
		on: (
			channel: string,
			listener: (event: any, ...args: any[]) => void
		) => void;
		off: (channel: string, listener: (...args: any[]) => void) => void;
		getPathForFile: (file: File) => string;
	};
}

declare module "mozjpeg";
