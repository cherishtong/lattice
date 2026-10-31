# Landscape adaptation

The same narration, audio, and ten-scene structure work at 1920 × 1080. Do not
crop or letterbox the vertical composition. Re-lay out each scene.

## Canvas

- 1920 × 1080, 16:9, 30 fps.
- Margins: 88 px left/right, 72 px top/bottom.
- Content band: roughly y=100 to y=950.
- Keep the bottom 120 px quiet for player controls.
- Headline 96 px; scene title 52–60 px; sub 23–24 px.

## Scene mapping

| Scene | Vertical | Landscape |
| --- | --- | --- |
| Brand | centered stack | text left, large wireframe logo + orbits right |
| Inventory | one column of 10 | 5 × 2 cards, 210 px tall |
| Library | title above mock | text + points left, mock right |
| Interface | full-width editor | same, 640 px tall, three columns |
| Themes | 2 × 2 grid | intro left, 2 × 2 grid right |
| Navigation | two stacked panels | references left, call tree right |
| Relations | stacked | code + type map left, relation rows right |
| Diff | stacked | diff left, summary + versions right |
| Translation | screenshot above pipeline | screenshot left, chips + pipeline right |
| Closing | centered | centered, slightly larger type |

## Specific fixes

- **Title wrapping**: a 60 px Chinese title needs ~620 px for 10 characters.
  Reduce to 52–54 px or widen the column; never let one character wrap alone.
- **Call tree**: the vertical tree is 460 × 860. For landscape, redraw it as a
  horizontal tree (780 × 560): root on the left, three children on the right,
  curved edges. Do not scale the portrait graph with `transform`; the aspect
  ratio does not fit.
- **Inventory**: five columns × two rows. Move the index to the top-right,
  stack name and version.
- **Themes**: keep 16:9 screenshots at native aspect. 2 × 2 on the right half
  gives ~540 px per card. Left column holds the title and a short note.
- **Relations**: code block 840 px, relation table 800 px. The type map stays
  under the code block; scale its SVG with `preserveAspectRatio="none"` only if
  the chip row still lines up.
- **Diff**: diff 1040 px wide, summary column 650 px. Put before/after version
  cards under the summary.
- **Closing**: enlarge the logo and stats slightly; centered negative space is
  fine.

## Motion and code

Keep `window.renderPromoFrame(t)`, the same `TIMELINE`, and the same audio. Only
the stage size and CSS layout change. The frame renderer viewport must match:

```js
await page.setViewport({ width: 1920, height: 1080, deviceScaleFactor: 1 });
```

## Common mistakes

- Copying vertical `top` values into 1080 px height; content falls off-screen.
- Stretching 16:9 screenshots into vertical frames, or vice versa.
- Scaling a portrait call tree instead of redrawing it.
- Letting a mono label sit below the safe area.
- Re-rendering audio: reuse the same mastered WAV if the script is unchanged.
