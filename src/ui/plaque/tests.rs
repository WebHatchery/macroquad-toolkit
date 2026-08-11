use super::*;

#[test]
fn palette_face_follows_state_priority() {
    let palette = PlaquePalette::default();
    assert_eq!(palette.face(PlaqueState::idle(false)).r, palette.disabled.r);
    assert_eq!(palette.face(PlaqueState::idle(true)).r, palette.normal.r);
    let pressed = PlaqueState {
        enabled: true,
        hovered: true,
        pressed: true,
        selected: false,
    };
    assert_eq!(palette.face(pressed).r, palette.pressed.r);
    let selected = PlaqueState {
        enabled: true,
        selected: true,
        ..PlaqueState::default()
    };
    assert_eq!(palette.face(selected).r, palette.hovered.r);
}

#[test]
fn border_color_prefers_selected_then_enabled() {
    let mut style = PlaqueStyle::default();
    let palette = PlaquePalette::default();
    let selected = PlaqueState {
        enabled: true,
        selected: true,
        ..PlaqueState::default()
    };

    assert_eq!(
        style.border_color(&palette, selected).r,
        palette.border.r,
        "no override falls back to palette border"
    );

    style.selected_border = Some(Color::new(0.74, 0.57, 0.30, 1.0));
    assert_eq!(style.border_color(&palette, selected).r, 0.74);
    assert_eq!(
        style.border_color(&palette, PlaqueState::idle(false)).r,
        style.disabled_border.r
    );
}

#[test]
fn label_size_fits_height_unless_fixed() {
    let mut style = PlaqueStyle::default();
    assert!((style.label_size(38.0) - 15.96).abs() < 0.01);
    assert_eq!(style.label_size(200.0), 17.0);
    assert_eq!(style.label_size(10.0), 10.0);
    style.font_size = Some(15.0);
    assert_eq!(style.label_size(38.0), 15.0);
}
