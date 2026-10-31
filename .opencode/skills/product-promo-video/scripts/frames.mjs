import puppeteer from 'puppeteer-core';
import { existsSync, mkdirSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';

const url = process.argv[2];
const outDir = process.argv[3] || 'frames';
const fps = Number(process.argv[4] || 30);
const chrome =
  process.env.CHROME_PATH ||
  [
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    '/usr/bin/google-chrome',
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  ].find((path) => existsSync(path));

if (!url) throw new Error('usage: node frames.mjs <file-url> [outDir] [fps]');
if (!chrome) throw new Error('Chrome not found; set CHROME_PATH');

const frameDir = resolve(outDir);
rmSync(frameDir, { recursive: true, force: true });
mkdirSync(frameDir, { recursive: true });

const browser = await puppeteer.launch({
  executablePath: chrome,
  headless: 'new',
  args: [
    '--force-color-profile=srgb',
    '--disable-lcd-text',
    '--font-render-hinting=none',
    '--hide-scrollbars',
    '--allow-file-access-from-files',
    '--disable-gpu',
  ],
});
const page = await browser.newPage();
await page.setViewport({ width: 1080, height: 1920, deviceScaleFactor: 1 });
await page.goto(url, { waitUntil: 'load' });
await page.evaluate(async () => {
  await document.fonts.ready;
  await Promise.all([...document.images].map((image) => image.decode().catch(() => {})));
});

const duration = await page.evaluate(() => window.PROMO_DURATION);
if (!duration) throw new Error('page must expose window.PROMO_DURATION');
const total = Math.round(fps * duration);
console.log(`duration=${duration}s frames=${total}`);

for (let index = 0; index < total; index++) {
  await page.evaluate((t) => window.renderPromoFrame(t), index / fps);
  await page.screenshot({
    path: resolve(frameDir, `frame-${String(index).padStart(4, '0')}.jpg`),
    type: 'jpeg',
    quality: 93,
    optimizeForSpeed: true,
  });
  if (index % 200 === 0) console.log(`frame ${index}/${total}`);
}
await browser.close();
console.log('frames done');
