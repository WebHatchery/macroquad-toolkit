use super::*;

#[test]
fn key_removal_preserves_foreground_and_existing_alpha() {
    let mut image = Image {
        width: 4,
        height: 1,
        bytes: vec![
            255, 0, 255, 255, 250, 5, 250, 255, 180, 80, 170, 200, 20, 120, 50, 128,
        ],
    };
    remove_key(&mut image, [255, 0, 255], 10, 20);
    assert_eq!(&image.bytes[..8], &[0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(&image.bytes[8..], &[180, 80, 170, 200, 20, 120, 50, 128]);
}

#[test]
fn feather_has_a_bounded_alpha_ramp_and_zero_feather_is_safe() {
    let mut image = Image {
        width: 1,
        height: 1,
        bytes: vec![235, 0, 255, 200],
    };
    remove_key(&mut image, [255, 0, 255], 10, 20);
    assert_eq!(image.bytes[3], 100);
    remove_key(&mut image, [255, 0, 255], 10, 0);
    assert_eq!(image.bytes[3], 100);
}
