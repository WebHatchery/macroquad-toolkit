# Shared game settings

The toolkit owns reusable preferences, persistence, editing and runtime adapters.
Games choose supported features, action names, defaults and gameplay behavior.
Existing `GameSettings`, camera and audio APIs remain available; struct literals
should use `..Default::default()` to accommodate added preferences.

## Foundation

`GameSettings::load_with_defaults(game, &defaults)` overlays stored fields onto
game defaults and sanitizes values. Missing or corrupt storage returns an error;
the host can explain it and use its defaults. `load` provides a silent
fallback convenience. Older flat JSON settings remain readable. Settings are
separate from progress, under the existing per-game `settings` storage key.

`SettingsSession::new(settings, defaults)` keeps a draft, committed snapshot and
game defaults. Defaults edits the draft; Cancel restores the committed snapshot.
`commit` sanitizes and saves before advancing the committed snapshot. On failure,
show the error and allow retry; do not apply runtime settings yet.

If an existing game stores additional settings in the same file, retain its typed
wrapper and use `commit_with` to save that complete payload. Do not replace a
game's composite settings file with the common settings alone. Unknown JSON
fields are tolerated on load but are not retained by `GameSettings` serialization.

`SettingsPanel::draw` uses opt-in `SettingsFeatures`. It provides paginated controls
with visible Previous, Next, Defaults, Cancel and Apply buttons. Pass a rectangle
at least 320x300 logical pixels and a `Pointer` in matching coordinates. Draw in
the UI camera, after the world. Keep this recovery panel in a stable coordinate
space while previewing UI scaling. Game HUDs can use `VirtualUi::responsive()`
and reflow against its logical bounds; `VirtualUi::scaled` additionally fits
minimum dimensions and can reduce the requested scale. UI scale supports
75–200%, independent of text scaling.
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
`stop_raw` removes its routing record. The `visible` flag still gates
new playback, and is not the focus-muting integration.

`apply_display` also applies effect preferences; headless callers can use
`apply_effects`. ScreenShake respects both shake-off and reduced motion. Reduced
motion hides cosmetic particle rendering, disables CRT flicker/moving scan bands
and stabilizes the runtime `math::pulse01` helper. Pure mathematical helpers stay
deterministic. Games must use readable state indicators rather than particle-only
gameplay cues and consult `reduced_motion_enabled` for their own animated effects.

`apply_autosave` updates both enablement and interval on `AutoSaveManager`. Games
still own save serialization and the safe-to-save predicate passed to `update`.

## Controls and camera integration

See [the compiling integration example](../examples/shared_settings.rs) for a
map with visible pan/zoom controls, touch gestures, settings editing, rebinding,
persistence and a display confirmation flow. It is a library adoption reference,
not a published game. The example deliberately does not expose audio or autosave
controls because its map has no sounds or progress to save.

1. Register game actions in `input::actions::ActionMap`. Each action has a stable
   ID, the exact visible touch-control label and default `Binding`s. For a direct
   gesture, use `with_touch_instruction("Drag the map")` instead of implying a
   button exists. Import
   `input::bindings::GamepadButton` without adding another dependency to the game.
2. Store preferences in `GameSettings.controls` and `.camera`. Missing nested
   fields inherit the game's defaults; empty binding arrays mean intentionally
   unbound. Keep legacy `key_bindings` only for compatibility. If a game actually
   used those labels as bindings, explicitly call `ActionMap::migrate_legacy`;
   this preserves controller defaults and existing real overrides.
3. Poll `ActionInput::capture(&settings.controls)` once per frame. It supplies
   physical button states, corrected mouse deltas, dead-zone-adjusted sticks and
   active-device activity. Do not simultaneously poll `GamepadInput`.
4. Add visible button action IDs to `ActionSnapshot.touch_pressed` for one-frame
   clicks, or `.touch_down` for held controls. Choose one convention per target;
   do not inject a second press on release of a held toggle control. Feed separate
   touches to multiple targets when the game needs simultaneous touch controls.
5. Call `ActionRuntime::update`, then consume `state(id).pressed`, `.down` or
   `.released`. `ActionMode::Toggle` is optional per action. Clear runtime state
   on menu/context changes, focus loss and applied remapping. Separate action maps
   represent mutually exclusive contexts; only update the active one.
6. `ActionMap::prompt` always names the visible touch control and supplements it
   with a binding for the active device. Controller labels currently use neutral
   physical names; branded controller glyph assets are not provided.

`RebindPanel::draw` edits the same session's controls draft. Give it at least
320x400 logical pixels. Navigation, device selection, clear, reset, Done and
capture cancellation all have tap targets. Capture waits up to ten seconds and
does not consume the click that opened it. A conflict keeps the previous binding
and displays its owner. The panel replaces bindings for one device at a time;
the lower-level `ActionMap::rebind` accepts multiple alternatives per device.
Use `ActionMap::conflicts` to diagnose conflicting defaults or edited saves.
Default and restored bindings should be conflict-free within each action map.
The settings panels use Pointer navigation; controller-only panel focus navigation
is not implemented yet. Gamepad input/remapping works for registered game actions.

Enable `SettingsFeatures.controls`, `.controller`, `.camera` and, only for cameras
supporting rotation, `.camera_rotation` to expose relevant preference controls.
Mouse X/Y sensitivity affects relative camera/input deltas, never the OS cursor.
Stick input has a radial dead zone rescaled to full range, independent X/Y
sensitivity and axis inversion. `ActionInput::rumble` applies vibration enablement
and strength; actual actuator support depends on the existing gamepads backend.
Android uses a no-controller fallback. Browser hosts still need the
existing gamepads JavaScript plugin; no new JS bridge is installed by this API.

`register_camera_actions` supplies pan, zoom and drag defaults, plus optional
rotation actions. Give every required action a visible touch control or explicit
gesture. Use `CameraFrame::from_actions` with the corrected left stick, add the
mouse drag delta only when the drag action is down, and add `TouchGesture` pan,
pinch scale and center. Do not feed synthesized mouse drag and touch pan together.

Call `CameraController::update_2d` **instead of** `Camera2D::update`.
It retains camera bounds and zoom limits, anchors zoom at the pointer, and applies
speed, edge scrolling and smoothing. Supply the viewport in the same coordinates
as the pointer. Set `captured` while any menu owns input: this stops all camera
input and clears residual smoothing. Edge scrolling requires a hovering pointer
inside the viewport, so a finger cannot accidentally activate it. Reduced motion
disables camera smoothing. `rotate_isometric` applies the shared rotation action
and speed to the toolkit's isometric camera. Other 3D movement/follow rules remain
game-owned until a shared adapter is needed.

The host still supplies focus signals to audio and clears gameplay input on focus
loss. Controller disconnect releases held physical actions on the next capture;
toggle actions intentionally retain their state until cleared or toggled again.

## Supported scope and validation

Expose only settings with a runtime implementation in the adopting game. The
shared panel does not implement controller-only focus navigation, automatic
focus/visibility detection, cloud saves, localization, accessibility bridges or
renderer/audio-device selection. Host-provided focus signals and visible
pointer/touch recovery controls remain required. Game-specific difficulty,
progress deletion and gameplay assistance belong to the game.

See [README validation](../README.md#validation) for package-scoped tests and
native/WASM checks. This library has no `publish.ps1`; adopting games use their
own no-argument publisher. Compile/unit checks do not establish actual controller,
audio-device or browser interaction behavior.
