# Visual language

## Canvas

- 1080 × 1920, 9:16, 30 fps.
- Safe margins: 64 px left/right, 84 px top, 72 px bottom.
- Content usually lives between y=80 and y=1500. The lower ~400 px stays quiet
  for platform UI (captions, buttons, comments).
- Never put essential text below y=1700.

## Tokens

```css
--bg: #0b0a08;
--panel: #14110d;
--line: rgba(242, 230, 212, .12);
--line-soft: rgba(242, 230, 212, .06);
--text: #f4ede2;
--soft: #cabfb0;
--sub: #9b8f7e;
--muted: #6d6355;
--accent: #e86a58;
--accent-soft: rgba(232, 106, 88, .13);
--gold: #d9a441;
--green: #86b57d;
--azure: #7fa6c9;
```

Background:

```css
background:
  radial-gradient(760px 900px at 16% 4%, rgba(196, 68, 52, .22), transparent 62%),
  radial-gradient(820px 900px at 90% 88%, rgba(150, 104, 36, .12), transparent 68%),
  linear-gradient(168deg, #0e0c09, #0a0907 58%, #0c0a08);
```

Add a 72 px grid at 4.5% white, a 4–5% grain layer, and a radial vignette.
Never use flat `#000`.

## Type

- Sans: `"Microsoft YaHei UI", "Microsoft YaHei", "Segoe UI", sans-serif`
- Mono: `"Cascadia Code", "Cascadia Mono", Consolas, monospace`
- H1: 104–112 px, weight 700, tracking -0.035em, line-height 1.1.
- H2: 64 px, weight 650, tracking -0.025em.
- Sub: 27 px, line-height 1.6, color `--sub`.
- Eyebrow: 15 px mono, tracking .28em, uppercase, color `--accent`.
- Body labels: 13–17 px; tiny metadata 9–11 px mono with wide tracking.
- Accent phrase: one line or one clause, never the whole headline.

## Scene recipes

### Brand opening

Logo 120–132 px, product name tracked +0.3em, H1 two lines, one accent line,
three chips, three stats. Reveal logo → name → H1 → chips → stats over 4 s, then
hold. Do not animate a spinning logo.

### Inventory list

One row per item, 96 px tall, 12 px gap, 10 rows fit. Left: zero-padded index in
mono. Center: name. Right: version in mono. Highlight new items with a coral
border, tinted fill, and a tiny `NEW` flag. Scan a coral outline down the rows
every ~0.5 s. Do not use a dense grid of logos.

### Interface showcase

Use a three-column window: file tree 196 px, code 1fr, outline 248 px. Include a
tab bar, status bar, and at least one active row. Code font 14–15 px, line-height
1.9. Keep lines short enough to avoid clipping. Add 2–3 floating callout chips
outside the window, not on top of code.

### Theme gallery

2 × 2 real screenshots, each in a dark card with a label row
(`无为 · 暗` / `WUWEI DARK`). Highlight one card at a time with a coral border.
Do not tint the screenshots; their own palettes are the point.

### Diff

One real changed line, red `-` then green `+`, with context lines around it.
Below: three symbol-summary cards (added / removed / changed) and a
before → after version pair. Never invent a diff; use a real version string or
a known change.

### Closing

Centered logo, two-line claim, three stats, repository URL, release metadata.
Hold the final frame for at least 2.5 s.

## Motion

- Scene in/out: 0.5 s opacity crossfade + 18 px translate.
- Child reveal: 0.65 s, cubic ease-out, 18–30 px upward.
- Stagger: 0.1–0.2 s per item.
- Scan highlight: step every 0.5 s, not a smooth sweep.
- No spring physics, no bounce, no zoom punches, no spinning.
- Everything is deterministic: `renderPromoFrame(t)` sets styles from `t`.
  Do not rely on CSS animations or `setTimeout`.

```js
const clamp = (v) => Math.min(1, Math.max(0, v));
const ease = (v) => 1 - Math.pow(1 - v, 3);

function reveal(el, local, at, dur = 0.65) {
  const p = ease(clamp((local - at) / dur));
  el.style.opacity = String(p);
  el.style.transform = `translateY(${(1 - p) * 24}px)`;
}
```

## Screenshots vs mockups

- Prefer fresh captures for anything that shows counts, paths, versions, or
  file lists. Old screenshots contradict new data.
- Use HTML mockups when the live UI is hard to capture; make them match real
  spacing and copy.
- If a screenshot is from an older build, crop away counts and paths, or label
  the scene as a concept.
- Never stretch a 16:9 screenshot into 9:16. Put it in a frame and use negative
  space.

## Pitfalls

- Overfilling the frame. Empty space is part of the style.
- Too many accent colors. Coral is the only primary accent.
- Centered body text. Left-align everything except the closing.
- Code lines clipped on the right. Shorten the snippet, do not shrink below
  13 px.
- Fake data: invented symbol counts, fabricated code diffs, outdated pack lists.
- Tiny text. If it is not readable at 50% scale, remove it.
