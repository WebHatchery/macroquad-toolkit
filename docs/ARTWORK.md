# Toolkit artwork

The toolkit ships an optional neutral reference pack and an embedded UI font.
Its ordinary UI surfaces and effects are procedural and do not require bitmap
chrome. Consuming games own their cast, world art, content icons and title images.

## Bundled files

| Asset | Path | Use |
| --- | --- | --- |
| Rajdhani SemiBold | [font](../assets/fonts/Rajdhani-SemiBold.ttf), [OFL license](../assets/fonts/OFL-Rajdhani.txt) | Embedded by `ui::font`; preserve its license |
| Archivist portrait | [PNG](../assets/images/portraits/portrait_archivist_neutral.png) | Reference portrait, linear filter |
| Drone atlas | [PNG](../assets/images/sprites/drone_atlas.png) | 4x2 grid of 128px cells, nearest filter |
| Drone billboard | [PNG](../assets/images/sprites/drone_billboard.png) | Upright transparent frame, nearest filter |
| Semantic sheet | [PNG](../assets/images/ui/icons/semantic_icons.png) | 4x4 reference sheet, linear filter |
| Semantic glyphs | [icon directory](../assets/images/ui/icons) | 50 named 48x48 PNGs; exact names in `artwork::ICON_KEYS` |
| Missing texture | [PNG](../assets/images/missing_texture.png) | 64x64 crosshatch, nearest filter |

The [manifest](../assets/artwork_manifest.json) lists five reference textures.
`artwork::load_toolkit_artwork(&mut assets).await` reads that manifest and returns
`Result<usize, String>`. A valid manifest can still yield fewer than five loaded
textures, because individual texture failures are skipped. Check counts/required
keys. `load_toolkit_icons` separately loads the 50 glyphs and returns the number
loaded, registering bare names such as `close` and `save`. It uses the asset
manager's default filter. Set that filter before loading if needed.

Paths are runtime paths relative to the process working directory or browser
asset base, **not relative to the dependency's source directory**. A consuming
game that uses these helpers must package the manifest/images at those paths.
The font is embedded; it does not need the runtime copy. Install the missing
texture as a placeholder explicitly; loading the manifest alone does not do so.

The `*_source.png` files are source/reference images, not manifest entries.
[Concept art](../concept_art/README.md) is non-runtime visual direction and is
separate from the reference pack. No catalog thumbnail is required for this
library. There is no outstanding required artwork backlog represented here.

## Delivery requirements

Add toolkit assets when a public component or example needs them. A game keeps
its own concrete inventory of every rendered entity, expression, animation and
UI state; placeholder inventory rows are not an asset specification.

- Record the texture key, path, dimensions, crop/pivot, filtering, intended use,
  source/author and allowed uses. Use semantic lowercase snake_case names.
- Use RGBA PNG for transparency, icons and pixel art; JPEG is supported for opaque
  painted art. Export SVG source masters to a runtime bitmap format.
- Keep portrait framing, lighting, eye line and expression alignment consistent.
  Leave identifying features inside the supported crops and check the smallest
  display size. Use transparent backgrounds for dialogue overlays.
- Use uniform unrotated atlas cells with stable contact points. `SpriteAtlas`
  does not read packed-atlas metadata; its frame drawing exposes horizontal
  flipping. Test gutters/filtering to prevent adjacent-cell bleeding.
- Keep upright billboards bottom-centered with transparent clearance. Check
  recolor masks against protected details and alpha across multiple seeds.
- Keep icons recognizable around 20–24px while controls retain at least 44x44
  logical-pixel tap targets. Use tint/opacity for widget states where appropriate;
  include text for unfamiliar actions and never convey semantics by color alone.
- Validate alpha edges over light, dark and saturated backgrounds. Bound all
  labels, preserve touch-only recovery and check common browser sizes.
- Keep manifest path case/separators identical across native and browser builds.
  Check loading errors and required keys; make missing assets obvious.
- Validate consuming games through their no-argument `publish.ps1` and place
  verification captures directly in `docs/verification/`, replacing prior states.

Keep editable masters separate from runtime deliveries. Game asset inventories
should cover actual portraits, tiles, effects, resources, equipment, tutorials,
backgrounds and catalog art that the game displays. Do not add duplicate bitmap
panels or speculative character variants to satisfy a generic checklist.
