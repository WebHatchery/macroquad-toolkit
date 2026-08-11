use super::SpriteClip;

#[test]
fn looping_clip_wraps_at_its_last_frame() {
    let clip = SpriteClip::new("walk", 4, 3, 10.0);
    assert_eq!(clip.frame_at(0.0), 4);
    assert_eq!(clip.frame_at(0.25), 6);
    assert_eq!(clip.frame_at(0.3), 4);
}

#[test]
fn one_shot_clip_clamps_at_its_last_frame() {
    let clip = SpriteClip::new("death", 8, 3, 10.0).one_shot();
    assert_eq!(clip.frame_at(0.0), 8);
    assert_eq!(clip.frame_at(9.0), 10);
}
