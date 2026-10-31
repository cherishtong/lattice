---
name: Product Promo Video
description: Produce a polished vertical product promo video with an HTML/Chrome frame renderer, Edge TTS narration, original ambient music, and ffmpeg mixing. Use for 9:16 product intros, launch teasers, feature tours, or restyling an existing promo.
---

# Product Promo Video

A repeatable pipeline for a 9:16 product film that feels calm, technical, and
premium. The look is warm near-black, coral accent, dense mono labels, heavy
Chinese sans headlines, real UI inside dark frames, and a lot of intentional
negative space. The sound is a low, steady male Chinese voice over sparse
original ambient synth music, with the music ducked under speech.

## When to use

Use this skill when the user asks for any of:

- a vertical product promo / launch video / feature tour
- a 30–90 second social video for a software product
- a re-style of an existing promo video
- a narrated walkthrough built from UI screenshots, HTML mockups, or code

Do not use it for live-action footage, fast meme edits, or heavily kinetic
motion-graphics sequences. This style is deliberately slow.

## Output contract

- 1080 × 1920, 9:16, 30 fps
- 45–90 seconds; 75–90 seconds when showing UI and themes
- H.264 High, CRF 18, yuv420p, AAC 192 kbps / 48 kHz
- Final loudness about -16 LUFS, true peak ≤ -1.5 dBFS
- Chinese narration + original music; no third-party audio
- Every scene readable with sound off

## Workflow

1. **Collect real product facts.** Read the product docs, screenshots, pack or
   data inventory, and actual UI copy. Never invent counts, versions, or code
   diffs. Label mockups as mockups if they are not real captures.
2. **Write the narration first.** One compact paragraph per scene, 5–8 seconds
   each. Use short sentences, concrete nouns, no marketing adjectives stacked
   together. Prefer “十套固定版本的源码包” over “海量生态”。
3. **Generate TTS and measure.** Generate one MP3 per scene, measure each
   duration with ffmpeg, then set the scene duration to `voice + 0.9–1.2s`.
   The timeline is derived from the voice, not guessed.
4. **Build the HTML stage.** Fixed 1080×1920, deterministic
   `window.renderPromoFrame(t)`, no CSS time-based animation. Each scene is
   absolutely positioned and driven by a timeline array.
5. **Render frames with Chrome.** Use `puppeteer-core` + installed Chrome or
   Chromium. Screenshot JPEG frames at 30 fps. No Playwright download needed.
6. **Mix audio and encode.** Generate original music, duck it under the voice,
   normalize, then encode frames + WAV with ffmpeg.
7. **QA.** Check first frame, each scene midpoint, last frame, audio peaks,
   duration, resolution, and that every screenshot actually loaded.

## Visual language

Read `references/visual-language.md` for tokens and component recipes.

Core rules:

- Near-black warm canvas, one coral accent, muted gold / green / azure for
  semantics.
- Headline: very large Chinese sans, tight tracking, one accent phrase.
- Eyebrow: small mono uppercase with wide tracking.
- Background: soft radial glow + 72 px grid + grain + vignette. Never pure
  flat black.
- Content sits in the upper ~75%; leave the lower area quiet for social UI.
- Reveal with 0.5–0.8 s opacity + 18–30 px translate. Stagger by 0.1–0.2 s.
- Use real screenshots inside dark rounded frames when available; use
  high-fidelity HTML mockups only when fresh captures are unavailable.
- Avoid stock photos, 3D blobs, glossy gradients, emoji, and fast zooms.

## Scene grammar

A 75–90 second product tour usually uses 10 scenes:

1. Brand opening: logo, product name, one-line positioning, 3 proof chips.
2. Inventory: versions / packs / models, one row per item, new items marked.
3. Library or management: import, validation, versions, uninstall.
4. Interface: full three-pane UI with callouts for tree, tabs, code, panels.
5. Themes or modes: 2 × 2 real screenshots with labels and a scan highlight.
6. Navigation: definitions, references, call tree, dependency.
7. Relations / data model: inheritance, implementation, lineage.
8. Diff / comparison: one real before/after line + symbol summary.
9. Local / offline capability: real panel screenshot + worker pipeline.
10. Closing: read-only promise, stats, repository URL.

## Narration rules

- Voice: `zh-CN-YunyangNeural` for calm technical tours; `zh-CN-YunxiNeural`
  for a younger, faster teaser; `zh-CN-XiaoxiaoNeural` for a warmer tone.
- Start with `rate: "-2%"`, `pitch: "-1Hz"`. Keep the rate between -5% and 0%.
- Write for ~5 Chinese characters per second. A 7-second line is ~35 characters.
- Say the product name once in scene 1 and once in the closing. Do not repeat it
  every scene.
- End each scene with a noun or a short claim, not a trailing clause.
- Read English product names naturally; do not translate them.

## Audio recipe

Read `references/audio-recipe.md` for the full chain.

- Original ambient bed: Am9 → Fmaj7 → Cmaj7 → G6, 3.6 s per chord, sine pad,
  slow arpeggio, low pulse, filtered noise air.
- Duck music under voice with `sidechaincompress`, not a static low volume.
- Voice chain: high-pass 90 Hz, low-pass 12.5 kHz, -2 dB at 240 Hz,
  +2.5 dB at 3 kHz, 3:1 compression, limiter.
- Target -16 LUFS integrated, -1.5 dBFS true peak.
- Never use trending audio or copyrighted music.

## Commands

Install once per machine:

```powershell
npm install puppeteer-core msedge-tts @ffmpeg-installer/ffmpeg
```

Generate voice and music:

```powershell
node scripts/tts.mjs
node scripts/music.mjs 86.6 music.wav
```

Render frames:

```powershell
node scripts/frames.mjs file:///C:/project/promo.html frames 30
```

Mix and encode:

```powershell
node scripts/mix.mjs promo.config.json
ffmpeg -framerate 30 -i frames/frame-%04d.jpg -i final-audio.wav `
  -c:v libx264 -preset slow -crf 18 -profile:v high -level 4.1 `
  -pix_fmt yuv420p -c:a aac -b:a 192k -shortest -movflags +faststart `
  promo-vertical.mp4
```

## QA checklist

- [ ] `ffprobe` reports 1080×1920, 30 fps, expected duration
- [ ] Audio integrated loudness -15 to -17 LUFS, peak ≤ -1.5 dBFS
- [ ] Every scene has a readable title and no clipped code/text
- [ ] All screenshots loaded (no blank white cards)
- [ ] Voice does not overlap scene changes
- [ ] No fake counts, fake code diffs, or outdated screenshots
- [ ] Last frame holds long enough to read the URL
- [ ] File size reasonable for sharing (5–15 MB at 45–90 s)

## Reuse

Copy this whole directory into another project as
`.opencode/skills/product-promo-video/`, or add its parent directory to the
`skills` array in `opencode.json`. `examples/lattice-vertical.html` is a complete
10-scene reference implementation with the exact visual system.
