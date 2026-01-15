# TinyEasy

[English](#english) | [中文](#中文)

TinyEasy is a lightweight desktop app for batch-compressing images. It focuses on fast local compression for PNG/JPG with simple, predictable options.

---

## English

### What is TinyEasy?

TinyEasy is an Electron desktop app built with Svelte 5 + Vite. It lets you import images (files or folders), compress them in batch, and save results either by overwriting originals or exporting to a chosen directory.

### Features

- Batch import via drag & drop, or system file/folder picker
- PNG compression powered by `oxipng`
- JPG/JPEG compression powered by `mozjpeg` (`cjpeg`)
- Three compression levels: Low / Medium / High
- Two output modes:
  - Overwrite Original
  - Select Save Path (keeps the folder structure when importing folders)
- Fallback behavior: if the compressed output is not smaller, TinyEasy keeps the original bytes (copies the original file to the output)
- Runs compression in a worker thread to keep the UI responsive

### Supported formats

- Compressed: `.png`, `.jpg`, `.jpeg`
- Displayed only (not compressed): other image formats will be listed, and can be copied to the output directory when using “Select Save Path”

### Compression levels

TinyEasy maps its UI levels to the following tooling defaults:

- PNG (`oxipng`)
  - Low: `--opt 2 --strip safe`
  - Medium: `--opt 4 --strip safe`
  - High: `--opt 6 --strip all`
- JPG/JPEG (`cjpeg`)
  - Low: `-quality 85 -optimize -progressive`
  - Medium: `-quality 60 -optimize -progressive`
  - High: `-quality 40 -optimize -progressive`

### Default output location

If you choose “Select Save Path” and do not pick a directory (or if the app has no previous selection), the default output directory is:

- `~/Downloads/EasyPNG` (macOS / Linux)
- `Downloads/EasyPNG` under your system Downloads folder (Windows)

### Development

#### Prerequisites

- Node.js 18+ (recommended)
- npm

#### Install

```bash
npm install
```

#### Run in development

```bash
npm run dev
```

#### Build (renderer + electron)

```bash
npm run build
```

#### Package installers / distributables

```bash
npm run dist
```

Build targets (via electron-builder):

- macOS: `dmg`, `zip`
- Windows: `nsis`
- Linux: `AppImage`

Build unpacked directory (no installer):

```bash
npm run pack
```

#### Type check

```bash
npm run check
```

### Project structure

- `src/`: renderer UI (Svelte)
- `electron/`: Electron main process + worker thread
- `dist-electron/`: compiled Electron entry (build output)
- `release/`: packaged artifacts (electron-builder output)

### Privacy

TinyEasy compresses files locally on your machine and does not upload images.

---

## 中文

### TinyEasy 是什么？

TinyEasy 是一个基于 Electron 的桌面端批量图片压缩工具，界面使用 Svelte 5 + Vite 构建。你可以导入文件或文件夹，然后批量压缩并选择覆盖原图或导出到指定目录。

### 功能特性

- 支持拖拽导入，或通过系统对话框选择文件/文件夹
- PNG 压缩使用 `oxipng`
- JPG/JPEG 压缩使用 `mozjpeg`（`cjpeg`）
- 三档压缩强度：低 / 中 / 高
- 两种输出方式：
  - 覆盖原图（Overwrite Original）
  - 选择保存路径（Select Save Path，会尽量保留文件夹结构）
- 兜底策略：若压缩后体积没有变小，则会保留原始字节（在导出模式下会拷贝原文件）
- 压缩任务在 Worker 线程执行，避免卡 UI

### 支持的格式

- 可压缩：`.png`、`.jpg`、`.jpeg`
- 仅展示不压缩：其他图片格式会被列出；在“选择保存路径”模式下可被原样拷贝到输出目录

### 压缩档位说明

TinyEasy 的三个档位对应的工具参数如下：

- PNG（`oxipng`）
  - 低：`--opt 2 --strip safe`
  - 中：`--opt 4 --strip safe`
  - 高：`--opt 6 --strip all`
- JPG/JPEG（`cjpeg`）
  - 低：`-quality 85 -optimize -progressive`
  - 中：`-quality 60 -optimize -progressive`
  - 高：`-quality 40 -optimize -progressive`

### 默认输出目录

当你选择“选择保存路径”但未指定目录（或应用没有记录上次选择）时，默认输出目录为：

- macOS / Linux：`~/Downloads/EasyPNG`
- Windows：系统“下载”目录下的 `EasyPNG`

### 本地开发

#### 环境要求

- Node.js 18+（推荐）
- npm

#### 安装依赖

```bash
npm install
```

#### 启动开发模式

```bash
npm run dev
```

#### 构建（渲染进程 + Electron）

```bash
npm run build
```

#### 打包生成安装包/分发产物

```bash
npm run dist
```

打包目标（electron-builder）：

- macOS：`dmg`、`zip`
- Windows：`nsis`
- Linux：`AppImage`

仅生成未打包目录（不生成安装包）：

```bash
npm run pack
```

#### 类型检查

```bash
npm run check
```

### 目录结构

- `src/`：渲染进程 UI（Svelte）
- `electron/`：Electron 主进程与 Worker 线程
- `dist-electron/`：Electron 编译产物
- `release/`：electron-builder 打包产物

### 隐私说明

TinyEasy 在本机本地处理图片，不会上传你的文件。
