#!/usr/bin/env python3
"""Render isolated iced windows through the app's debug-only screenshot API.

No OS screen capture, saved user configuration, API Key or live Tinify request is used.
An available desktop display is required. Screenshot files are excluded from packaging.
"""
import argparse
import json
import math
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import tempfile
import zlib

ROOT = Path(__file__).resolve().parent.parent
SCENES = {
    'light-en-empty': ('light', 'en', 'empty', False),
    'dark-zh-empty': ('dark', 'zh-CN', 'empty', True),
    'light-en-import': ('light', 'en', 'import', True),
    'dark-zh-import': ('dark', 'zh-CN', 'import', True),
    'light-zh-gallery': ('light', 'zh-CN', 'gallery', False),
    'light-en-gallery': ('light', 'en', 'gallery', False),
    'dark-zh-gallery-small': ('dark', 'zh-CN', 'gallery', True),
    'light-en-filter-empty': ('light', 'en', 'filter-empty', True),
    'dark-zh-filter-empty': ('dark', 'zh-CN', 'filter-empty', True),
    'light-en-states-small': ('light', 'en', 'states', True),
    'light-en-done': ('light', 'en', 'done', True),
    'dark-en-done': ('dark', 'en', 'done', True),
    'dark-zh-done': ('dark', 'zh-CN', 'done', True),
    'light-en-directory-small': ('light', 'en', 'directory', True),
    'dark-zh-directory-small': ('dark', 'zh-CN', 'directory', True),
    'light-en-keys': ('light', 'en', 'keys', True),
    'dark-zh-keys-many': ('dark', 'zh-CN', 'keys-many', True),
    'light-en-settings': ('light', 'en', 'settings', True),
    'dark-en-settings': ('dark', 'en', 'settings', True),
    'dark-zh-settings': ('dark', 'zh-CN', 'settings', True),
    'system-en-settings': ('system', 'en', 'settings', True),
    'system-zh-settings': ('system', 'zh-CN', 'settings', True),
    'dark-en-error': ('dark', 'en', 'error', True),
    'light-zh-settings-error': ('light', 'zh-CN', 'settings-error', True),
    'dark-en-keys-error': ('dark', 'en', 'keys-error', True),
    'dark-zh-keys-usage-error': ('dark', 'zh-CN', 'keys-usage-error', True),
    'light-en-toast-many': ('light', 'en', 'toast-many', True),
    'light-en-preview': ('light', 'en', 'preview', False),
}
DOC_SCENES = {
    'light-en-gallery': 'gallery-light-en.png',
    'dark-en-done': 'completed-dark-en.png',
    'light-en-settings': 'settings-light-en.png',
    'dark-en-settings': 'settings-dark-en.png',
    'light-en-preview': 'preview-light-en.png',
}

def png(path, index):
    width, height = 420, 280
    palettes = [((205,228,237),(96,151,162),(29,97,119)), ((239,219,193),(193,145,108),(130,95,83)), ((215,222,239),(132,146,188),(69,84,131)), ((220,230,205),(142,168,124),(68,112,86)), ((228,219,230),(183,145,167),(124,81,113)), ((210,229,235),(139,182,196),(67,125,147))]
    sky, hill, foreground = palettes[index % len(palettes)]
    rows = []
    for y in range(height):
        row = bytearray(b'\0')
        for x in range(width):
            rgb = tuple(min(255, c + int(12 * y / height)) for c in sky)
            if (x - 305) ** 2 + (y - 60) ** 2 < 24 ** 2:
                rgb = (255,245,213)
            if y > 138 + 25 * math.sin(x / 65 + index):
                rgb = hill
            if y > 200 + 24 * math.sin(x / 88 + 1 + index):
                rgb = foreground
            row.extend(rgb)
        rows.append(row)
    def chunk(kind, data):
        return struct.pack('!I', len(data)) + kind + data + struct.pack('!I', zlib.crc32(kind + data))
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('!IIBBBBB', width,height,8,2,0,0,0)) + chunk(b'IDAT',zlib.compress(b''.join(rows))) + chunk(b'IEND',b''))

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--skip-build', action='store_true')
    parser.add_argument('--capture-ms', type=int, default=800)
    parser.add_argument('--scene', action='append', choices=SCENES)
    parser.add_argument('--docs', action='store_true', help='Render all public README screenshots with English UI and sample filenames')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.docs and args.scene:
        parser.error('--docs selects the public scenes; do not combine it with --scene')
    if not args.skip_build:
        cargo = shutil.which('cargo') or str(Path.home()/'.cargo/bin'/('cargo.exe' if platform.system() == 'Windows' else 'cargo'))
        subprocess.run([cargo,'build','--locked'],cwd=ROOT,check=True)
    binary = ROOT/'target/debug'/('tinyeasy.exe' if platform.system()=='Windows' else 'tinyeasy')
    output=(args.output or ROOT / 'docs' / ('images' if args.docs else 'ui-screenshots')).resolve();output.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='tinyeasy-render-ui-') as temporary:
        directory=Path(temporary);config=directory/'config';config.mkdir()
        images=directory/'images';images.mkdir();(directory/'Output 中文 images').mkdir()
        names=['Mountain morning.png','Desert light.png','Blue hour.png','中文 山谷.png','Long English filename for layout verification.png','Coastal evening.png']
        if args.docs:
            names=['Mountain morning.png','Desert light.png','Blue hour.png','Green valley.png','Forest afternoon.png','Coastal evening.png']
        for index,name in enumerate(names):png(images/name,index)
        for name in DOC_SCENES if args.docs else args.scene or SCENES:
            theme,language,scene,small=SCENES[name]
            (config/'settings.json').write_text(json.dumps({'version':1,'theme':theme,'language':language}))
            env=os.environ.copy()
            for key in list(env):
                if key.startswith('TINYEASY_QA_') or key in ('TINIFY_API_KEY','TINIFY_API_KEYS'):env.pop(key)
            screenshot=output/(DOC_SCENES[name] if args.docs else name+'.png')
            env.update(TINYEASY_QA_DIR=str(config),TINYEASY_QA_SCENE=scene,TINYEASY_QA_SCREENSHOT=str(screenshot),TINYEASY_QA_EXIT='1',TINYEASY_QA_CAPTURE_MS=str(args.capture_ms))
            if small:env['TINYEASY_QA_SMALL']='1'
            if scene in ('gallery','states','preview','directory','filter-empty','done'):env['TINYEASY_QA_IMPORT']=str(images)
            result=subprocess.run([str(binary)],cwd=ROOT,env=env,capture_output=True,text=True,timeout=30)
            if result.returncode or not screenshot.is_file():
                raise SystemExit(f'{name}: render failed (exit {result.returncode})\n{result.stderr}')
            print(name,'rendered')

if __name__=='__main__':
    main()
