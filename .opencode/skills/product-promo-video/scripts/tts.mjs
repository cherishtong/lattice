import { MsEdgeTTS, OUTPUT_FORMAT } from 'msedge-tts';
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';

const configPath = process.argv[2] || 'promo.config.json';
const config = JSON.parse(readFileSync(configPath, 'utf8').replace(/^\uFEFF/, ''));
const voice = config.voice || 'zh-CN-YunyangNeural';
const rate = config.rate || '-2%';
const pitch = config.pitch || '-1Hz';
const outDir = config.outDir || 'voice';
const segments = config.segments || [];

if (!segments.length) {
  throw new Error('promo.config.json needs a non-empty "segments" array');
}

rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });

const tts = new MsEdgeTTS();
await tts.setMetadata(voice, OUTPUT_FORMAT.AUDIO_24KHZ_96KBITRATE_MONO_MP3);

const manifest = [];
for (const segment of segments) {
  const id = String(Array.isArray(segment) ? segment[0] : segment.id).padStart(2, '0');
  const text = Array.isArray(segment) ? segment[1] : segment.text;
  const dir = `${outDir}/${id}`;
  mkdirSync(dir, { recursive: true });
  const { audioFilePath } = await tts.toFile(dir, text, { rate, pitch });
  const target = `${outDir}/${id}.mp3`;
  if (existsSync(target)) rmSync(target);
  renameSync(audioFilePath, target);
  manifest.push({ id, text, file: target });
  console.log(`voice ${id}`);
}
tts.close();
writeFileSync(`${outDir}/manifest.json`, JSON.stringify(manifest, null, 2));
