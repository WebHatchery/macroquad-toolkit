use super::*;

fn shape(offset: f32) -> Buffer {
    let mut buffer = Buffer::new(32, 32);
    buffer.circle(vec2(16.0 + offset, 16.0), 10.0, WHITE);
    buffer.tri(
        vec2(4.0, 28.0),
        vec2(28.0, 28.0),
        vec2(16.0 + offset, 6.0),
        Color::new(0.5, 0.2, 0.8, 0.6),
    );
    buffer
}

#[test]
fn the_same_drawing_fingerprints_the_same() {
    assert_eq!(shape(0.0).fingerprint(), shape(0.0).fingerprint());
}

#[test]
fn a_changed_drawing_fingerprints_differently() {
    // The whole point: a shape that moved by one pixel must not pass a
    // golden test that was recorded before it moved.
    assert_ne!(shape(0.0).fingerprint(), shape(1.0).fingerprint());
}

#[test]
fn an_empty_buffer_still_has_a_fingerprint() {
    // Rather than a special case a caller has to remember. Two blank
    // buffers of the same size agree; different sizes do not.
    assert_eq!(
        Buffer::new(8, 8).fingerprint(),
        Buffer::new(8, 8).fingerprint()
    );
    assert_ne!(
        Buffer::new(8, 8).fingerprint(),
        Buffer::new(9, 9).fingerprint()
    );
}

#[test]
fn monochrome_difference_is_zero_for_identical_art_and_grows_with_change() {
    let base = shape(0.0);
    assert_eq!(base.monochrome_difference(&shape(0.0)), 0.0);
    assert!(base.monochrome_difference(&shape(6.0)) > 0.0);
}

#[test]
fn silhouette_difference_ignores_colour() {
    // Two shapes of the same outline in different colours are the same
    // silhouette, which is what makes it the right tool for asking whether
    // shape alone carries a difference.
    let mut pale = Buffer::new(24, 24);
    let mut dark = Buffer::new(24, 24);
    pale.circle(vec2(12.0, 12.0), 8.0, WHITE);
    dark.circle(vec2(12.0, 12.0), 8.0, Color::new(0.3, 0.1, 0.1, 1.0));

    assert_eq!(pale.silhouette_difference(&dark), 0.0);
    assert!(pale.monochrome_difference(&dark) > 0.0);
}
