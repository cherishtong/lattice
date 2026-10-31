# Lattice vertical reference

`lattice-vertical.html` is the full 10-scene reference implementation used for
the Lattice 2026 vertical promo. It is intentionally data-specific; copy the
structure, tokens, and motion helpers rather than the product copy.

- 1080 × 1920 stage
- `window.PROMO_DURATION` and `window.renderPromoFrame(t)`
- `?t=42.5` previews a single frame in Chrome
- themes and translation panel are real screenshots in `assets/`
- library and interface scenes are high-fidelity HTML mockups

To preview:

```powershell
node ../scripts/frames.mjs "file:///C:/path/to/lattice-vertical.html" frames 30
```

Or render one frame:

```powershell
chrome --headless=new --screenshot=preview.png --window-size=1080,1920 `
  "file:///C:/path/to/lattice-vertical.html?t=35"
```
