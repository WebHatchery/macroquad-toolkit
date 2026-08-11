use super::*;

#[test]
fn test_autosave_manager_runs_after_interval() {
    let mut autosave = AutoSaveManager::new(1.0);
    let mut saves = 0;

    assert!(!autosave
        .update(0.5, true, || {
            saves += 1;
            Ok(())
        })
        .unwrap());
    assert_eq!(saves, 0);

    assert!(autosave
        .update(0.5, true, || {
            saves += 1;
            Ok(())
        })
        .unwrap());
    assert_eq!(saves, 1);
}
