# Future settings additions

The foundation and action/camera integration are implemented in
[SETTINGS.md](SETTINGS.md). The following are future additions, not currently
promised capabilities. Features should be opt-in and appear only when a game and
its target platform support them. Each addition needs persistence, runtime wiring,
touch-accessible UI, meaningful tests and adoption documentation.

## Next shared modules

| Addition | Toolkit responsibility | Game responsibility / acceptance criteria |
| --- | --- | --- |
| Localisation | Locale preference, translation lookup/fallback, supported language list, font coverage hooks | Translated strings/assets and correct layout; never offer untranslated languages |
| Subtitles and dialogue | Size, background, colour, speaker display, text speed, instant reveal, auto-advance timing | Text, speakers, audio timing, seen-dialogue state; visible advance/skip controls |
| Closed captions | Caption queue and visual presentation | Identify important sound events and descriptions; captions must preserve gameplay information |
| Accessible themes | Semantic palettes, high contrast, redundant shape/icon cues, optional font families | Map game meanings to colours and icons; validate readability without colour alone |
| Motion and flashing | Extend the existing reduced-motion policy to remaining toolkit transitions, custom effect hooks and flash limits | Audit authored effects; do not label a switch as guaranteeing photosensitivity safety |
| Notifications | Category/critical filters, duration, sound choice, history preferences | Categories, severity, targets and pause rules; critical recovery messages must remain accessible |
| Privacy | Analytics consent before client initialization/queueing, runtime disable and queue disposal, scoped data deletion, policy-link control | Policy URL and collection purpose; do not send queued events after opt-out |
| Crash reporting | Separate optional upload consent from current local crash logging | Service endpoint and report content; local diagnostics are not automatically an upload permission |

## Input and camera extensions

- Controller-only navigation and focus in settings/rebinding panels; preserve
  touch recovery controls and avoid binding capture consuming navigation events.
- Branded Xbox/PlayStation/Nintendo glyph packs and optional style selection.
  Current prompts use device-aware neutral names and always name a touch control.
- Multiple controller selection, per-player profiles, stick-axis remapping,
  response curves, separate per-stick dead zones, trigger thresholds and chords.
- Optional remap conflict resolution (swap/unbind another action). Current
  remapping rejects conflicts atomically and supports clearing bindings explicitly.
- Mash-to-hold/single-press and repetitive-input assistance. Games decide which
  actions can be simplified without breaking gameplay.
- Broader orbit/3D camera adapters, per-device inversion, independent smoothing
  and inertia preferences, shake strength, follow/event snapping, FOV and head bob.
  Current shared camera preferences cover 2D pan/zoom/edge scrolling/smoothing
  and isometric rotation; games still own target selection and collision.
- Portable focus/visibility events, held-input suppression after refocus/menu exit,
  configurable pause-on-focus-loss and background-audio integration. Audio currently
  accepts a host-supplied focus boolean; this is not automatic event detection.

## Interface and save extensions

- HUD opacity/visibility, optional element registry, safe-area padding, tooltip
  delay/detail, custom cursor sizing and platform-dependent confinement.
- Common confirmation policies, tutorial-reset hooks, remembered menu tabs,
  number/unit/date/time formatting preferences. Games supply the actual operations.
- Autosave rotation count, backup/recovery UI, save-on-exit policy, import/export
  UI and progress-deletion confirmation. Reuse existing persistence primitives;
  games own schemas and safe save points. Do not rely on browser exit events to
  guarantee saving. Autosave enablement/interval wiring is already implemented.
- Cloud saves through a separate optional service adapter, including conflict
  handling and offline behavior. A local settings checkbox cannot provide this.
- Screen-reader/native accessibility bridges, after assessing platform support;
  font choice alone does not provide screen-reader support.

## Platform and renderer work requiring investigation

| Feature | Required work before exposing it |
| --- | --- |
| Window size / resolution | Distinguish native window dimensions, browser canvas size and internal render resolution; enumerate valid choices and confirm/revert changes |
| Window modes | Verify separate borderless/exclusive modes by backend; existing fullscreen toggle does not imply three supported modes |
| VSync / frame cap | Check startup versus live configuration, pacing and browser scheduling; simulation must not depend on render rate |
| Monitor / refresh rate | Backend enumeration and supported mode application, with recovery |
| Render scale / anti-aliasing | Shared render-target pipeline, asset filtering policy, capability detection and restart requirements |
| Brightness / gamma / contrast | Defined rendering/colour-space policy and calibration controls |
| Mono / dynamic range / headphones | Actual mixing or processing support; gain sliders alone cannot implement these |
| Audio output device | Backend device enumeration and switching with error recovery |
| HDR and calibration | End-to-end HDR output support; do not represent an SDR colour adjustment as HDR |
| Upscaling / frame generation / ray tracing | A renderer and platform integration that really implement them; defer until a game justifies the complexity |

## Keep game-owned

Difficulty and its components, aim assist, AI aggression, resource abundance,
puzzle assistance, inventory automation, minimap semantics, quest tracking, gore,
cinematic skipping/seen state, combat/fast-forward speed, event-triggered pauses,
network regions, matchmaking, cross-play, friends/invites, voice/text chat and
moderation remain game or service responsibilities. A shared panel may host their
controls, but the toolkit must not pretend to implement the underlying behavior.

Graphics presets, particles, shadows, bloom, blur, depth of field, textures,
reflections, foliage and view distance belong to the renderer that implements
them. Extract reusable adapters when multiple games share such a renderer.
