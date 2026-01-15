import { app } from 'electron';
import fs from 'fs/promises';
import path from 'path';

app.whenReady().then(async () => {
  // Set app name to match the project
  app.name = 'tinyeasy';
  const userDataPath = app.getPath('userData');
  console.log(`Cleaning cache at: ${userDataPath}`);
  
  const cacheDirs = ['Cache', 'Code Cache', 'GPUCache', 'DawnCache'];
  
  for (const dir of cacheDirs) {
    const fullPath = path.join(userDataPath, dir);
    try {
      // Check if exists first to avoid confusing logs, though rm force handles it
      await fs.rm(fullPath, { recursive: true, force: true });
      console.log(`✓ Cleaned: ${dir}`);
    } catch (err) {
      console.error(`✗ Failed to clean ${dir}:`, err.message);
    }
  }
  
  console.log('Done.');
  app.quit();
});
