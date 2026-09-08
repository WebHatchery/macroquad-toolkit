//! Integration reference: a touch-controlled map, saved settings, rebinding,
//! camera preferences and timed display confirmation. No external assets.
use macroquad::prelude::*;
use macroquad_toolkit::{
    audio::SoundManager,
    camera::{register_camera_actions, Camera2D as MapCamera, CameraController, CameraFrame},
    input::{
        actions::{ActionDefinition, ActionMap, ActionRuntime},
        bindings::{Binding, GamepadButton},
        remap::RebindPanel,
        ActionInput, TouchGesture,
    },
    persistence::AutoSaveManager,
    settings::{
        panel_button, DisplayPreview, GameSettings, SettingsFeatures, SettingsPanel,
        SettingsPanelAction, SettingsSession,
    },
    ui::{draw_text_centered_in_box, Pointer},
};

const GAME: &str = "toolkit_settings_example";

fn conf() -> Conf {
    Conf {
        window_title: "Shared toolkit settings".into(),
        window_width: 960,
        window_height: 720,
        ..Default::default()
    }
}

fn apply(settings: &GameSettings, audio: &mut SoundManager<u8>, autosave: &mut AutoSaveManager) {
    settings.apply_display();
    settings.apply_autosave(autosave);
    audio.apply_settings(settings, true);
}

#[macroquad::main(conf)]
async fn main() {
    let defaults = GameSettings::default();
    let (mut settings, mut message) = match GameSettings::load_with_defaults(GAME, &defaults) {
        Ok(settings) => (settings, String::new()),
        Err(error) => (defaults.clone(), format!("Using defaults: {error}")),
    };
    let mut map = ActionMap::default();
    register_camera_actions(&mut map, false).expect("unique camera actions");
    map.register(ActionDefinition::new(
        "settings",
        "SETTINGS",
        vec![
            Binding::key(KeyCode::Escape),
            Binding::gamepad(GamepadButton::RightCenterCluster),
        ],
    ))
    .expect("unique settings action");
    let mut input = ActionInput::new();
    let mut actions = ActionRuntime::default();
    let mut camera = MapCamera::default();
    let mut camera_input = CameraController::default();
    let mut gestures = TouchGesture::new();
    let mut panel = SettingsPanel::default();
    let mut rebind = RebindPanel::default();
    let mut session = SettingsSession::new(settings.clone(), defaults.clone());
    let mut audio = SoundManager::<u8>::new();
    let mut autosave = AutoSaveManager::default();
    apply(&settings, &mut audio, &mut autosave);
    let mut open = false;
    let mut binding_page = false;
    let mut preview: Option<DisplayPreview> = None;
    loop {
        let dt = get_frame_time();
        let screen = Rect::new(0.0, 0.0, screen_width(), screen_height());
        let pointer = Pointer::read(|point| point);
        let mut snapshot = input.capture(&settings.controls);
        let control_height = (48.0 * macroquad_toolkit::ui::ui_scale()).max(44.0);
        let controls_top = screen.h - 2.0 * (control_height + 6.0) - 4.0;
        let gesture = gestures.update();
        set_default_camera();
        clear_background(Color::new(0.06, 0.08, 0.12, 1.0));

        // Explicit touch controls feed the same actions as keys and gamepads.
        if !open {
            for (index, id) in [
                "camera_left",
                "camera_right",
                "camera_up",
                "camera_down",
                "camera_zoom_in",
                "camera_zoom_out",
            ]
            .into_iter()
            .enumerate()
            {
                let width = (screen.w - 32.0) / 3.0;
                let rect = Rect::new(
                    12.0 + (index % 3) as f32 * (width + 4.0),
                    controls_top + (index / 3) as f32 * (control_height + 6.0),
                    width,
                    control_height,
                );
                // Drawn again after world rendering below, without polling twice.
                if pointer.down && rect.contains(pointer.position) {
                    snapshot.touch_down.insert(id.into());
                }
            }
        }
        actions.update(&map, &settings.controls, &snapshot);
        if !open
            && (actions.state("settings").pressed
                || (pointer.released
                    && Rect::new(12.0, 12.0, 160.0, 48.0).contains(pointer.position)))
        {
            open = true;
            session = SettingsSession::new(settings.clone(), defaults.clone());
            actions.clear();
        }
        let on_controls = pointer.position.y > controls_top - 8.0 || pointer.position.y < 72.0;
        let mut frame = CameraFrame::from_actions(&actions, snapshot.left_stick);
        frame.captured = open;
        if !on_controls {
            frame.pointer = Some(if gesture.active {
                gesture.center
            } else {
                pointer.position
            });
            frame.hovering = pointer.hovering;
            frame.wheel = mouse_wheel().1;
            frame.drag = if gesture.active {
                gesture.pan
            } else if actions.state("camera_drag").down {
                snapshot.mouse_delta
            } else {
                Vec2::ZERO
            };
            frame.pinch_scale = gesture.scale;
        }
        camera_input.update_2d(&mut camera, &settings.camera, frame, screen, dt);
        camera.begin();
        for x in -10..=10 {
            for y in -10..=10 {
                draw_rectangle_lines(x as f32 * 80.0, y as f32 * 80.0, 76.0, 76.0, 2.0, DARKGRAY);
            }
        }
        draw_circle(0.0, 0.0, 24.0, SKYBLUE);
        set_default_camera();
        if !open {
            panel_button(Rect::new(12.0, 12.0, 160.0, 48.0), "SETTINGS", pointer);
            for (index, label) in [
                "PAN LEFT",
                "PAN RIGHT",
                "PAN UP",
                "PAN DOWN",
                "ZOOM IN",
                "ZOOM OUT",
            ]
            .into_iter()
            .enumerate()
            {
                let width = (screen.w - 32.0) / 3.0;
                panel_button(
                    Rect::new(
                        12.0 + (index % 3) as f32 * (width + 4.0),
                        controls_top + (index / 3) as f32 * (control_height + 6.0),
                        width,
                        control_height,
                    ),
                    label,
                    pointer,
                );
            }
            draw_text_centered_in_box(
                &map.prompt("camera_drag", &settings.controls, actions.active_device),
                180.0,
                12.0,
                screen.w - 192.0,
                48.0,
                18.0,
                WHITE,
            );
        } else {
            draw_rectangle(
                0.0,
                0.0,
                screen.w,
                screen.h,
                Color::new(0.06, 0.08, 0.12, 1.0),
            );
            let area = Rect::new(16.0, 76.0, screen.w - 32.0, screen.h - 148.0);
            if let Some(trial) = preview.as_mut() {
                if trial.update(dt) {
                    preview = None;
                    session.cancel();
                    message = "Display restored".into();
                } else {
                    draw_text_centered_in_box(
                        &format!("Keep this display? {:.0} seconds", trial.remaining()),
                        area.x,
                        area.y,
                        area.w,
                        80.0,
                        22.0,
                        WHITE,
                    );
                    if panel_button(
                        Rect::new(area.x, area.y + 88.0, area.w, 48.0),
                        "Keep display",
                        pointer,
                    ) {
                        match session.commit(GAME) {
                            Ok(()) => {
                                trial.confirm();
                                settings = session.draft.clone();
                                apply(&settings, &mut audio, &mut autosave);
                                message = "Settings saved".into();
                            }
                            Err(error) => {
                                trial.revert();
                                message = error;
                            }
                        }
                        preview = None;
                    } else if panel_button(
                        Rect::new(area.x, area.y + 144.0, area.w, 48.0),
                        "Revert display",
                        pointer,
                    ) {
                        trial.revert();
                        preview = None;
                        session.cancel();
                    }
                }
            } else if binding_page {
                if rebind.draw(
                    area,
                    pointer,
                    &map,
                    &mut session.draft.controls,
                    &snapshot,
                    dt,
                ) {
                    binding_page = false;
                }
            } else {
                if panel_button(
                    Rect::new(16.0, 16.0, screen.w - 32.0, 48.0),
                    "Edit action bindings",
                    pointer,
                ) {
                    binding_page = true;
                }
                match panel.draw(
                    area,
                    pointer,
                    &mut session,
                    SettingsFeatures {
                        fullscreen: true,
                        ui_scale: true,
                        text_scale: true,
                        effects: false,
                        controls: true,
                        controller: true,
                        camera: true,
                        ..Default::default()
                    },
                ) {
                    SettingsPanelAction::Apply => {
                        if session.draft.fullscreen != settings.fullscreen
                            || session.draft.ui_scale != settings.ui_scale
                            || session.draft.ui_text_scale != settings.ui_text_scale
                        {
                            preview = Some(DisplayPreview::begin(&settings, &session.draft));
                        } else {
                            match session.commit(GAME) {
                                Ok(()) => {
                                    settings = session.draft.clone();
                                    apply(&settings, &mut audio, &mut autosave);
                                    actions.clear();
                                    message = "Settings saved".into();
                                }
                                Err(error) => message = error,
                            }
                        }
                    }
                    SettingsPanelAction::Cancel => {
                        open = false;
                        actions.clear();
                    }
                    SettingsPanelAction::None => {}
                }
            }
            draw_text_centered_in_box(
                &message,
                16.0,
                screen.h - 64.0,
                screen.w - 32.0,
                56.0,
                16.0,
                WHITE,
            );
        }
        next_frame().await;
    }
}
