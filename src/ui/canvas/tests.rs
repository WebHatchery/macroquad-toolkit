use super::*;

#[test]
fn ninety_five_percent_preserves_content_and_centers_it() {
    let ui = UiCanvas::from_screen_size(
        vec2(1280.0, 720.0),
        vec2(1280.0, 720.0),
        0.95,
        Vec2::splat(0.5),
    );
    assert_eq!(ui.content, vec2(1280.0, 720.0));
    assert_eq!(ui.view, Rect::new(32.0, 18.0, 1216.0, 684.0));
    assert!(ui.horizontal.is_none());
    assert_eq!(ui.origin, Vec2::ZERO);
}

#[test]
fn enlarged_canvas_can_reach_every_edge_with_aligned_input() {
    for zoom in [1.05, 1.5] {
        for pan in [Vec2::ZERO, Vec2::splat(0.5), Vec2::ONE] {
            let ui =
                UiCanvas::from_screen_size(vec2(1280.0, 720.0), vec2(1280.0, 720.0), zoom, pan);
            let point = vec2(640.0, 360.0);
            assert!(ui.screen_to_ui(ui.ui_to_screen(point)).distance(point) < 0.001);
            if pan == Vec2::ONE {
                assert!(
                    ui.screen_to_ui(vec2(ui.view.right(), ui.view.bottom()))
                        .distance(ui.content)
                        < 0.001
                );
            }
            let mut moved = Vec2::ZERO;
            let track = ui.horizontal.unwrap();
            let pointer = Pointer {
                position: vec2(track.right() - 1.0, track.y + 22.0),
                down: true,
                released: true,
                ..Default::default()
            };
            ui.navigate(pointer, &mut moved);
            assert_eq!(moved.x, 1.0);
            assert!(!ui.pointer(pointer).released);
        }
    }
}
