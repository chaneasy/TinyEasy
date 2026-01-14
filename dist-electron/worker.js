import { parentPort } from 'worker_threads';
import fs from 'fs/promises';
import path from 'path';
import { Transformer, CompressionType } from '@napi-rs/image';
if (!parentPort) {
    throw new Error('Must be run as a worker');
}
parentPort.on('message', async (task) => {
    try {
        const { filePath, options } = task;
        const fileBuffer = await fs.readFile(filePath);
        let outputBuffer;
        const transformer = new Transformer(fileBuffer);
        // Apply compression based on format
        if (options.format === 'png') {
            // Map level/quality to CompressionType
            // CompressionType: Default=0, Fast=1, Best=2
            let compressionType = CompressionType.Default;
            if (options.level === 'high')
                compressionType = CompressionType.Best;
            if (options.level === 'low')
                compressionType = CompressionType.Fast;
            outputBuffer = await transformer.png({
                compressionType,
            });
        }
        else if (options.format === 'jpeg' || options.format === 'jpg') {
            outputBuffer = await transformer.jpeg(options.quality);
        }
        else if (options.format === 'webp') {
            outputBuffer = await transformer.webp(options.quality);
        }
        else {
            outputBuffer = await transformer.png();
        }
        // Overwrite or save to new location?
        // Requirement says "Output file location shortcut access", implies it's saved.
        // For now, let's overwrite or add a suffix.
        // Requirement: "Batch rename output files" is an additional feature.
        // Let's create a 'compressed' folder or just overwrite for simplicity if not specified?
        // Safer to create a new file or replace.
        // Let's replace for now or use a suffix "_compressed".
        // Better: let the main process decide the output path?
        // For simplicity, let's save to the same folder with a prefix "min_".
        const dir = path.dirname(filePath);
        const ext = path.extname(filePath);
        const name = path.basename(filePath, ext);
        const newFileName = `${name}_min${ext}`;
        const outputPath = path.join(dir, newFileName);
        await fs.writeFile(outputPath, outputBuffer);
        const newSize = outputBuffer.length;
        parentPort?.postMessage({
            success: true,
            filePath,
            outputPath,
            originalSize: options.originalSize,
            compressedSize: newSize,
        });
    }
    catch (error) {
        parentPort?.postMessage({
            success: false,
            filePath: task.filePath,
            error: error instanceof Error ? error.message : String(error),
        });
    }
});
