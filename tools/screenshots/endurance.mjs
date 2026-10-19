#!/usr/bin/env node
import assert from 'node:assert/strict';
import { chromium } from 'playwright';

const args = process.argv.slice(2);
const valueAfter = (name, fallback) => {
  const index = args.indexOf(`--${name}`);
  return index >= 0 ? args[index + 1] : fallback;
};
const url = valueAfter(
  'url',
  'http://127.0.0.1:4173/?quality=low&qa_session_secs=30&qa_start_environment_time=99.8',
);

const browser = await chromium.launch();
// Keep the scene's 16:10 shape while reducing SwiftShader work on hosted CI.
const page = await browser.newPage({ viewport: { width: 640, height: 400 } });
const failures = [];
page.on('pageerror', (error) => failures.push(error.message));

try {
  await page.goto(url, { waitUntil: 'load', timeout: 120_000 });
  await page.waitForFunction(() => window.__FTD_READY__ === true, null, { timeout: 120_000 });
  await page.keyboard.press('Enter');
  await page.waitForFunction(
    () => window.__FTD_STATE__?.phase === 'running' && window.__FTD_STATE__?.audio_unlocked === true,
    null,
    { timeout: 20_000 },
  );
  await page.waitForFunction(() => window.__FTD_STATE__?.day_cycles >= 1, null, {
    timeout: 90_000,
  });

  const before = await page.evaluate(() => ({
    environment: window.__FTD_STATE__.environment_time,
    remaining: window.__FTD_STATE__.session_remaining,
  }));
  await page.waitForFunction(
    ({ environment, remaining }) =>
      window.__FTD_STATE__?.environment_time > environment + 0.15 &&
      window.__FTD_STATE__?.session_remaining < remaining - 0.15,
    before,
    { timeout: 15_000 },
  );
  await page.waitForFunction(() => window.__FTD_STATE__?.audio_music_players === 1, null, {
    // The fade is simulation-time bounded, but very slow software renderers
    // can need substantially more wall time to submit enough frames.
    timeout: 60_000,
  });

  const after = await page.evaluate(() => window.__FTD_STATE__);
  assert.equal(after.audio_music_track, 1, 'the second cycle must select soundtrack 1');
  assert.equal(after.audio_music_players, 1, 'the outgoing soundtrack must be despawned');
  assert.equal(failures.length, 0, `browser errors: ${failures.join('; ')}`);
  console.log(
    `browser endurance: OK (cycle=${after.day_cycles}, env=${after.environment_time.toFixed(2)}, players=${after.audio_music_players})`,
  );
} catch (error) {
  const state = await page.evaluate(() => window.__FTD_STATE__ ?? null).catch(() => null);
  console.error(`browser endurance state at failure: ${JSON.stringify(state)}`);
  throw error;
} finally {
  await browser.close();
}
