#!/usr/bin/env node
// shot.mjs — load the game headless, wait for readiness, capture PNGs (+console),
// drive synthetic input, and record a __FTD_STATE__ timeline for assertions.
//
// Usage: node shot.mjs [--url http://localhost:8080] [--out ../../docs/screenshots/M5]
//                      [--frames 3] [--keys ArrowLeft:1200,KeyR:150] [--wait 500]
//
// Key spec: "<Key>:<hold-ms>" — key held for the duration, state snapshotted
// during the hold, "held-<Key>.png" captured, then released.
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const args = process.argv.slice(2);
const arg = (name, def) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : def;
};
const url = arg('url', 'http://localhost:8080');
const outDir = resolve(arg('out', '../../docs/screenshots/M5'));
const frames = parseInt(arg('frames', '2'), 10);
const keysArg = arg('keys', '');
const quick = args.includes('--quickkeys');
const waitMs = parseInt(arg('wait', '0'), 10);

mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1024, height: 640 } });
const consoleLines = [];
page.on('console', (m) => consoleLines.push(`[${m.type()}] ${m.text()}`));
page.on('pageerror', (e) => consoleLines.push(`[pageerror] ${e.message}`));

await page.goto(url, { waitUntil: 'load', timeout: 120_000 });
await page.waitForFunction(() => window.__FTD_READY__ === true, null, { timeout: 120_000 });
await page.waitForTimeout(800);

const timeline = [];
const snap = async (label) => {
  // MUST be awaited: on a busy rAF page the evaluate can otherwise run much
  // later and mislabel the snapshot
  const state = await page.evaluate(() => window.__FTD_STATE__ ?? null);
  timeline.push({ at: label, state });
};
await snap('boot');

if (keysArg) {
  for (const spec of keysArg.split(',')) {
    const [key, ms] = spec.split(':');
    await page.keyboard.down(key);
    await page.waitForTimeout(parseInt(ms || '500', 10));
    if (!quick) {
      // screenshot FIRST: it is slow under software GL, and the state snapshot
      // must describe the same moment the PNG captures
      await page.screenshot({ path: `${outDir}/held-${key}.png` });
      await snap(`during-${key}`);
    }
    await page.keyboard.up(key);
    if (quick) await page.waitForTimeout(180); // release gap between taps
    await page.waitForTimeout(120);
  }
}
if (waitMs) await page.waitForTimeout(waitMs);
await snap('end');

for (let i = 1; i <= frames; i++) {
  await page.screenshot({ path: `${outDir}/shot-${String(i).padStart(3, '0')}.png` });
  await page.waitForTimeout(350);
}

const meta = await page.evaluate(() => ({
  ready: window.__FTD_READY__ ?? null,
  canvas: (() => {
    const c = document.querySelector('canvas');
    return c ? { w: c.width, h: c.height } : null;
  })(),
  title: document.title,
}));
await browser.close();

console.log(
  JSON.stringify({ url, outDir, meta, timeline, console: consoleLines.slice(0, 30) }, null, 2),
);
