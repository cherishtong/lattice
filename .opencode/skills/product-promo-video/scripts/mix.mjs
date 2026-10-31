import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const configPath = process.argv[2] || 'promo.config.json';
const config = JSON.parse(readFileSync(configPath, 'utf8').replace(/^\uFEFF/, ''));
const voices = config.voices || [];
const starts = config.starts || [];
const duration = Number(config.duration || 60);
const music = config.music || 'music.wav';
const out = config.out || 'final-audio.wav';
const musicVolume = Number(config.musicVolume ?? 0.55);

if (!voices.length || voices.length !== starts.length) {
  throw new Error('config needs voices[] and starts[] with equal length');
}

const require = createRequire(import.meta.url);
let ffmpeg = process.env.FFMPEG;
if (!ffmpeg) {
  try {
    ffmpeg = require('@ffmpeg-installer/ffmpeg').path;
  } catch {
    ffmpeg = 'ffmpeg';
  }
}

const inputs = ['-i', music];
for (const voice of voices) inputs.push('-i', voice);

const filters = [];
filters.push(
  `[0:a]volume=${musicVolume},` +
    `afade=t=in:st=0:d=1.8,afade=t=out:st=${Math.max(0, duration - 3).toFixed(2)}:d=3,` +
    `aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo[mus]`,
);

voices.forEach((_, index) => {
  const delay = Math.round(Number(starts[index]) * 1000);
  filters.push(`[${index + 1}:a]aresample=48000,adelay=${delay}|${delay}[v${index}]`);
});

const mixInputs = voices.map((_, index) => `[v${index}]`).join('');
filters.push(
  `${mixInputs}amix=inputs=${voices.length},volume=${voices.length},` +
    `highpass=f=90,lowpass=f=12500,` +
    `equalizer=f=240:t=q:w=1:g=-2,equalizer=f=3000:t=q:w=1.2:g=2.5,` +
    `acompressor=threshold=0.12:ratio=3:attack=6:release=150:makeup=2,` +
    `alimiter=limit=0.95,` +
    `aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo[vox0]`,
);
filters.push('[vox0]asplit=2[voxA][voxB]');
filters.push('[mus][voxA]sidechaincompress=threshold=0.035:ratio=4:attack=20:release=420[duck]');
filters.push(
  `[duck][voxB]amix=inputs=2,loudnorm=I=-16:TP=-1.5:LRA=11,aresample=48000,apad=pad_len=60000[out]`,
);

const args = [
  '-y',
  '-hide_banner',
  '-loglevel',
  'warning',
  ...inputs,
  '-filter_complex',
  filters.join(';'),
  '-map',
  '[out]',
  '-c:a',
  'pcm_s16le',
  '-t',
  String(duration),
  out,
];

console.log(`mixing ${voices.length} voice tracks + music -> ${out}`);
const child = spawn(ffmpeg, args, { stdio: 'inherit' });
child.on('exit', (code) => {
  if (code !== 0) process.exit(code ?? 1);
  console.log(`wrote ${out}`);
});
