#!/usr/bin/env node
// shot.mjs — load the game headless, wait for readiness, capture PNGs (+console).
// Usage: node shot.mjs [--url http://localhost:8080] [--out ../../docs/screenshots/M4]
//                      [--frames 3] [--keys ArrowLeft:1200]
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const args = process.argv.slice(2);
const arg = (name, def) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : def;
};
const url = arg('url', 'http://localhost:8080');
const outDir = resolve(arg('out', '../../docs/screenshots/M4'));
const frames = parseInt(arg('frames', '3'), 10);
const keysArg = arg('keys', '');

mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1024, height: 640 } });
const consoleLines = [];
page.on('console', (m) => consoleLines.push(`[${m.type()}] ${m.text()}`));
page.on('pageerror', (e) => consoleLines.push(`[pageerror] ${e.message}`));

await page.goto(url, { waitUntil: 'load', timeout: 120_000 });

// wait for the app's readiness marker (index.html sets __FTD_READY__)
await page.waitForFunction(() => window.__FTD_READY__ === true, null, { timeout: 120_000 });
await page.waitForTimeout(800); // let a few frames render

// optional synthetic keyboard input: "ArrowLeft:1200" = hold ArrowLeft 1200ms
// a screenshot is captured WHILE the key is held (held-<Key>.png), then after release
if (keysArg) {
  for (const spec of keysArg.split(',')) {
    const [key, ms] = spec.split(':');
    await page.keyboard.down(key);
    await page.waitForTimeout(parseInt(ms || '500', 10));
    await page.screenshot({ path: `${outDir}/held-${key}.png` });
    await page.keyboard.up(key);
  }
}

for (let i = 1; i <= frames; i++) {
  await page.screenshot({ path: `${outDir}/shot-${String(i).padStart(3, '0')}.png` });
  await page.waitForTimeout(350);
}

const state = await page.evaluate(() => ({
  ready: window.__FTD_READY__ ?? null,
  canvas: (() => {
    const c = document.querySelector('canvas');
    return c ? { w: c.width, h: c.height } : null;
  })(),
  title: document.title,
}));
await browser.close();

console.log(JSON.stringify({ url, outDir, state, console: consoleLines.slice(0, 30) }, null, 2));
