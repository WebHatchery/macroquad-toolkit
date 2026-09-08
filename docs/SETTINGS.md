# Shared game settings

The toolkit owns reusable preferences, persistence, editing and runtime adapters.
Games choose supported features, action names, defaults and gameplay behavior.
Existing `GameSettings`, camera and audio APIs remain available; struct literals
should use `..Default::default()` to accommodate added preferences.

## Foundation

`GameSettings::load_with_defaults(game, &defaults)` overlays stored fields onto
game defaults and sanitizes values. Missing or corrupt storage returns an error;
the host can explain it and use its defaults. `load` retains the old silent
fallback convenience. Older flat JSON settings remain readable. Settings are
separate from progress, under the existing per-game `settings` storage key.

`SettingsSession::new(settings, defaults)` keeps a draft, committed snapshot and
game defaults. Defaults edits the draft; Cancel restores the committed snapshot.
`commit` sanitizes and saves before advancing the committed snapshot. On failure,
show the error and allow retry; do not apply runtime settings yet.

`SettingsPanel::draw` uses opt-in `SettingsFeatures`. It provides paginated controls
with visible Previous, Next, Defaults, Cancel and Apply buttons. Pass a rectangle
at least 320x300 logical pixels and a `Pointer` in matching coordinates. Draw in
the UI camera, after the world. Keep this recovery panel in a stable coordinate
space while previewing UI scaling; game HUD layouts can use `VirtualUi::scaled`.
Do not send its pointer events to gameplay while it is open.

After a successful Apply, and once at startup:

```rust,no_run
use macroquad_toolkit::{audio::SoundManager, persistence::AutoSaveManager,
    settings::GameSettings};
let settings = GameSettings::default();
let mut audio = SoundManager::<u32>::new();
let mut autosave = AutoSaveManager::default();
settings.apply_display();
settings.apply_autosave(&mut autosave);
audio.apply_settings(&settings, true);
```

Use `DisplayPreview::begin(&committed, &draft)` before committing changes to
fullscreen or UI/text scale. Show visible Keep and Revert buttons, tick `update`
with real elapsed time even while simulation is paused, and cancel the draft on
timeout. The trial restores the previous display after 15 seconds. On Keep,
save first, then `confirm`; on save failure or menu exit call `revert`. Never
discard an active preview without reverting. The host retains ownership of the
window and event loop; the toolkit does not create a competing runtime.

## Audio and effects

Use `audio.play_group(id, AudioGroup::Music, gain, true)` for looping music and
the Sfx, Voice, Ui and Ambience groups for other sounds. `play_sfx` also participates.
`apply_settings` updates already-playing managed sounds. Feed it the host's actual
focus/visibility state whenever that changes; this release does not install focus
event hooks. Group gain is multiplied by master gain and optional focus muting.

Macroquad adjusts volume per loaded sound asset, not per playback instance. A
sound ID has one managed group/gain at a time. Load distinct IDs when independent
routing is required. `play_raw` deliberately opts that ID out of managed routing;
`stop_raw` removes its routing record. The historical `visible` flag still gates
new playback, and is not the focus-muting integration.

`apply_display` also applies effect preferences; headless callers can use
`apply_effects`. ScreenShake respects both shake-off and reduced motion. Reduced
motion hides cosmetic particle rendering, disables CRT flicker/moving scan bands
and stabilizes the runtime `math::pulse01` helper. Pure mathematical helpers stay
deterministic. Games must use readable state indicators rather than particle-only
gameplay cues and consult `reduced_motion_enabled` for their own animated effects.

`apply_autosave` updates both enablement and interval on `AutoSaveManager`. Games
still own save serialization and the safe-to-save predicate passed to `update`.

## Validation

This library has no `publish.ps1`; the prescribed publishing path cannot run here.
Use library/unit/doc tests and native/WASM compile checks as additional checks,
then validate each adopting game's own default `publish.ps1`. This is not evidence
of hardware controller, audio-device or browser interaction testing.
