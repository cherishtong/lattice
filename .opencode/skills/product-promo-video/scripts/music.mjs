import { writeFileSync } from 'node:fs';

const duration = Number(process.argv[2] || 60);
const out = process.argv[3] || 'music.wav';
const sr = 48000;
const total = Math.floor(sr * duration);
const left = new Float32Array(total);
const right = new Float32Array(total);

const chords = [
  { root: 110.0, notes: [220.0, 261.63, 329.63, 392.0, 493.88] }, // Am9
  { root: 87.31, notes: [174.61, 220.0, 261.63, 329.63] },         // Fmaj7
  { root: 130.81, notes: [196.0, 246.94, 329.63, 392.0] },         // Cmaj7
  { root: 98.0, notes: [196.0, 246.94, 293.66, 329.63] },          // G6
];
const chordLength = 3.6;

function env(time, start, length, attack, release) {
  const t = time - start;
  if (t < 0 || t > length) return 0;
  return Math.min(1, t / attack, (length - t) / release);
}

function addTone(start, length, frequency, amplitude, pan, attack = 0.8, release = 1.4) {
  const from = Math.max(0, Math.floor(start * sr));
  const to = Math.min(total, Math.ceil((start + length) * sr));
  const lg = Math.cos((pan + 1) * Math.PI / 4);
  const rg = Math.sin((pan + 1) * Math.PI / 4);
  for (let i = from; i < to; i++) {
    const t = i / sr - start;
    const e = env(i / sr, start, length, attack, release);
    if (e <= 0) continue;
    const base = Math.sin(2 * Math.PI * frequency * t);
    const warm = 0.18 * Math.sin(2 * Math.PI * frequency * 2.001 * t + 0.6);
    const sub = 0.20 * Math.sin(2 * Math.PI * frequency * 0.5 * t);
    const drift = 1 + 0.0013 * Math.sin(2 * Math.PI * 0.09 * (i / sr) + frequency);
    const value = (base + warm + sub) * e * amplitude * drift;
    left[i] += value * lg;
    right[i] += value * rg;
  }
}

for (let index = 0; index * chordLength < duration; index++) {
  const chord = chords[index % chords.length];
  const start = index * chordLength;
  const length = Math.min(chordLength + 1.8, duration - start);
  addTone(start, length, chord.root, 0.070, 0.0, 1.1, 1.8);
  chord.notes.forEach((note, i) => {
    const pan = [-0.42, 0.38, -0.16, 0.22, 0.0][i % 5];
    addTone(start, length, note, 0.028, pan, 1.4, 2.0);
  });
}

for (let step = 0; step * 0.45 < duration - 1; step++) {
  if (step % 4 === 3) continue;
  const chord = chords[Math.floor((step * 0.45) / chordLength) % chords.length];
  const octave = step % 8 < 4 ? 2 : 4;
  const frequency = chord.notes[(step + Math.floor(step / 8)) % chord.notes.length] * octave;
  const start = step * 0.45;
  const from = Math.floor(start * sr);
  const to = Math.min(total, Math.floor((start + 1.5) * sr));
  const pan = step % 2 ? 0.28 : -0.30;
  const lg = Math.cos((pan + 1) * Math.PI / 4);
  const rg = Math.sin((pan + 1) * Math.PI / 4);
  for (let j = from; j < to; j++) {
    const t = j / sr - start;
    const e = Math.exp(-4.6 * t) * Math.min(1, t / 0.012);
    const value = Math.sin(2 * Math.PI * frequency * t + 0.4 * Math.exp(-4 * t) * Math.sin(2 * Math.PI * 2.7 * t)) * e * 0.022;
    left[j] += value * lg;
    right[j] += value * rg;
  }
}

for (let start = 1.2; start < duration - 3; start += 1.8) {
  const from = Math.floor(start * sr);
  const to = Math.min(total, Math.floor((start + 0.9) * sr));
  for (let j = from; j < to; j++) {
    const t = j / sr - start;
    const e = Math.exp(-7 * t) * Math.min(1, t / 0.02);
    const value = Math.sin(2 * Math.PI * 55 * t) * e * 0.020;
    left[j] += value;
    right[j] += value;
  }
}

let lowL = 0;
let lowR = 0;
for (let i = 0; i < total; i++) {
  const whiteL = Math.random() * 2 - 1;
  const whiteR = Math.random() * 2 - 1;
  lowL += (whiteL - lowL) * 0.010;
  lowR += (whiteR - lowR) * 0.010;
  const lfo = 0.5 + 0.5 * Math.sin(2 * Math.PI * 0.055 * (i / sr));
  const gain = 0.007 + lfo * 0.005;
  left[i] += lowL * gain * 2.0;
  right[i] += lowR * gain * 2.0;
}

let peak = 0;
for (let i = 0; i < total; i++) {
  const time = i / sr;
  const fade = Math.min(1, time / 1.8, (duration - time) / 2.8);
  left[i] = Math.tanh(left[i] * 1.25) * fade;
  right[i] = Math.tanh(right[i] * 1.25) * fade;
  peak = Math.max(peak, Math.abs(left[i]), Math.abs(right[i]));
}
const scale = peak > 0 ? 0.58 / peak : 1;
const buffer = Buffer.alloc(44 + total * 4);
buffer.write('RIFF', 0);
buffer.writeUInt32LE(36 + total * 4, 4);
buffer.write('WAVE', 8);
buffer.write('fmt ', 12);
buffer.writeUInt32LE(16, 16);
buffer.writeUInt16LE(1, 20);
buffer.writeUInt16LE(2, 22);
buffer.writeUInt32LE(sr, 24);
buffer.writeUInt32LE(sr * 4, 28);
buffer.writeUInt16LE(4, 32);
buffer.writeUInt16LE(16, 34);
buffer.write('data', 36);
buffer.writeUInt32LE(total * 4, 40);
for (let i = 0; i < total; i++) {
  buffer.writeInt16LE(Math.round(Math.max(-1, Math.min(1, left[i] * scale)) * 32767), 44 + i * 4);
  buffer.writeInt16LE(Math.round(Math.max(-1, Math.min(1, right[i] * scale)) * 32767), 46 + i * 4);
}
writeFileSync(out, buffer);
console.log(`wrote ${out} (${(buffer.length / 1024 / 1024).toFixed(1)} MiB)`);
