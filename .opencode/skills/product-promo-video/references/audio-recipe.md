# Audio recipe

## Voice

| Use | Voice | Rate | Pitch |
| --- | --- | --- | --- |
| Calm technical tour | `zh-CN-YunyangNeural` | `-2%` | `-1Hz` |
| Younger teaser | `zh-CN-YunxiNeural` | `-4%` | `-2Hz` |
| Warmer, softer | `zh-CN-XiaoxiaoNeural` | `-5%` | `-2Hz` |

Use Edge TTS via `msedge-tts`. One MP3 per scene:

```js
const tts = new MsEdgeTTS();
await tts.setMetadata(voice, OUTPUT_FORMAT.AUDIO_24KHZ_96KBITRATE_MONO_MP3);
await tts.toFile(dir, text, { rate, pitch });
```

Write for about five Chinese characters per second. A 7-second line is roughly
35 characters. One idea per scene. Avoid three clauses joined by commas.

## Timing

1. Generate all voice segments.
2. Measure each one:
   `ffmpeg -i voice/01.mp3 -f null -`
3. Scene duration = voice duration + 0.9–1.2 s.
4. Voice start = scene start + 0.4 s.
5. Total video = sum of scene durations.
6. Encode with `-shortest`; pad the mix by ~1 s.

Never place voice exactly at the scene cut. The 0.4 s lead-in lets the first
title appear and avoids a hard cut on the first syllable.

## Music

Original ambient bed, generated in Node. No samples, no third-party rights.

- 48 kHz stereo, duration = video duration.
- Progression: Am9 → Fmaj7 → Cmaj7 → G6.
- Chord length 3.6 s.
- Layers:
  - low root sine, amplitude ~0.07
  - chord tones, amplitude ~0.028, wide pan
  - slow arpeggio every 0.45 s, exponential decay, amplitude ~0.022
  - low 55 Hz pulse every 1.8 s, amplitude ~0.020
  - filtered noise air, very low level
- Soft saturation `tanh(x * 1.25)`, fade in 1.8 s, fade out 2.8 s.
- Normalize the music file to about 0.58 peak so the mix has headroom.

Keep the bed sparse. It should be felt, not listened to.

## Mix

Voice chain:

```text
highpass=f=90
lowpass=f=12500
equalizer=f=240:t=q:w=1:g=-2
equalizer=f=3000:t=q:w=1.2:g=2.5
acompressor=threshold=0.12:ratio=3:attack=6:release=150:makeup=2
alimiter=limit=0.95
aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo
```

Music chain:

```text
volume=0.55
afade=t=in:st=0:d=1.8
afade=t=out:st=<duration-3>:d=3
aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo
```

Ducking:

```text
[mus][voxA]sidechaincompress=threshold=0.035:ratio=4:attack=20:release=420[duck]
[duck][voxB]amix=inputs=2,loudnorm=I=-16:TP=-1.5:LRA=11,aresample=48000
```

The voice output must be split before it is used twice:

```text
[vox0]asplit=2[voxA][voxB]
```

Pad the final audio so it is slightly longer than the video:

```text
apad=pad_len=60000
```

Targets:

- integrated loudness: -15 to -17 LUFS
- true peak: ≤ -1.5 dBFS
- stereo, 48 kHz, AAC 192 kbps in the MP4

## Common mistakes

- Static music volume: speech gets buried in the loud parts.
- Mixing `amix` without accounting for its normalization; either use a modern
  ffmpeg with `normalize=0` or compensate with `volume=N`.
- Forgetting `asplit` and reusing a filter label twice; ffmpeg treats it as an
  error.
- Low-passing at 12.5 kHz before resampling a 24 kHz TTS file; always
  `aresample=48000` first.
- Normalizing the voice only. Normalize the final mix.
- Using a music loop longer than the video; generate to the exact duration.
