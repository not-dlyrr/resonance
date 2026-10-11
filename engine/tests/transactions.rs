use resonance_engine::*;
use std::{fs, io::Write};
fn fixture() -> (tempfile::TempDir, Engine) {
    let t = tempfile::tempdir().unwrap();
    let e = Engine {
        root: t.path().join("Spotify"),
        state: t.path().join("State"),
    };
    fs::create_dir_all(e.root.join("Apps")).unwrap();
    fs::write(e.root.join("Spotify.exe"), b"fixture-executable").unwrap();
    fs::write(e.root.join("fixture-version"), "1.3.4.258").unwrap();
    fs::write(e.root.join("prefs"), "private-user-settings").unwrap();
    let mut z = zip::ZipWriter::new(fs::File::create(e.root.join("Apps/xpui.spa")).unwrap());
    let o = zip::write::SimpleFileOptions::default();
    z.start_file("index.html", o).unwrap();
    z.write_all(
        b"<html><head></head><body><script src='/xpui-snapshot.js'></script></body></html>",
    )
    .unwrap();
    z.start_file("xpui-snapshot.js", o).unwrap();
    z.write_all(b"console.log('fixture')").unwrap();
    z.finish().unwrap();
    (t, e)
}
#[test]
fn apply_switch_restore_preserves_archive_and_user_data() {
    let (_t, e) = fixture();
    let original = e.identity().unwrap();
    let mut p = Profile::default();
    e.apply(&p, false).unwrap();
    assert!(e.status().unwrap().verified);
    p.theme = "ink".into();
    e.apply(&p, false).unwrap();
    assert_eq!(e.status().unwrap().active.unwrap().theme, "ink");
    e.restore().unwrap();
    assert!(!e.root.join("Apps/xpui").exists());
    assert_eq!(e.identity().unwrap(), original);
    assert_eq!(
        fs::read_to_string(e.root.join("prefs")).unwrap(),
        "private-user-settings"
    );
}
#[test]
fn repeated_apply_has_one_injection() {
    let (_t, e) = fixture();
    for _ in 0..3 {
        e.apply(&Profile::default(), false).unwrap();
    }
    assert_eq!(
        fs::read_to_string(e.root.join("Apps/xpui/index.html"))
            .unwrap()
            .matches("id=\"resonance-runtime\"")
            .count(),
        1
    );
}
#[test]
fn interrupted_rename_recovers_existing_customization() {
    let (_t, e) = fixture();
    fs::create_dir(e.root.join("Apps/xpui")).unwrap();
    fs::write(e.root.join("Apps/xpui/index.html"), "old-spicetify").unwrap();
    assert!(e
        .apply_with_failure(&Profile::default(), false, Some("after-rename"))
        .is_err());
    e.recover().unwrap();
    assert_eq!(
        fs::read_to_string(e.root.join("Apps/xpui/index.html")).unwrap(),
        "old-spicetify"
    );
}
#[test]
fn interrupted_commit_rolls_back_active_profile() {
    let (_t, e) = fixture();
    e.apply(&Profile::default(), false).unwrap();
    let mut p = Profile::default();
    p.theme = "paper".into();
    assert!(e
        .apply_with_failure(&p, false, Some("after-commit"))
        .is_err());
    e.recover().unwrap();
    assert_eq!(e.status().unwrap().active.unwrap().theme, "slate");
    assert!(e.status().unwrap().verified);
}
#[test]
fn interrupted_first_apply_returns_stock() {
    let (_t, e) = fixture();
    assert!(e
        .apply_with_failure(&Profile::default(), false, Some("after-commit"))
        .is_err());
    e.recover().unwrap();
    assert!(!e.root.join("Apps/xpui").exists());
}
#[test]
fn corrupted_backup_blocks_apply() {
    let (_t, e) = fixture();
    let b = e.backup().unwrap();
    fs::write(std::path::Path::new(&b).join("xpui.spa"), b"corrupt").unwrap();
    assert!(e
        .apply(&Profile::default(), false)
        .unwrap_err()
        .to_string()
        .contains("integrity"));
    assert!(!e.root.join("Apps/xpui").exists());
}
#[test]
fn stale_restore_is_blocked() {
    let (_t, e) = fixture();
    e.apply(&Profile::default(), false).unwrap();
    fs::write(e.root.join("Spotify.exe"), "updated").unwrap();
    assert!(e.status().unwrap().update_invalidated);
    assert!(e.restore().is_err());
    assert!(e.root.join("Apps/xpui").exists());
}
#[test]
fn unsupported_version_blocks_apply() {
    let (_t, e) = fixture();
    fs::write(e.root.join("fixture-version"), "1.4.0.1").unwrap();
    assert!(!e.status().unwrap().supported);
    assert!(e.apply(&Profile::default(), false).is_err());
}
#[test]
fn recovery_disables_extensions() {
    let (_t, e) = fixture();
    e.install("keyboard").unwrap();
    let mut p = Profile::default();
    p.extensions.push("keyboard".into());
    e.apply(&p, true).unwrap();
    let js = fs::read_to_string(e.root.join("Apps/xpui/resonance/runtime.js")).unwrap();
    assert!(!js.contains("Alt"));
    assert!(e.status().unwrap().active.unwrap().extensions.is_empty());
}
#[test]
fn extensions_require_deliberate_install() {
    let (_t, e) = fixture();
    let mut p = Profile::default();
    p.extensions.push("keyboard".into());
    assert!(e.apply(&p, false).is_err());
    assert!(!e.root.join("Apps/xpui").exists());
}
#[test]
fn profiles_persist() {
    let (_t, e) = fixture();
    let p = Profile::default();
    e.save_profile(&p).unwrap();
    assert_eq!(e.profiles().unwrap(), vec![p.clone()]);
    e.delete_profile(&p.name).unwrap();
    assert!(e.profiles().unwrap().is_empty());
}
#[test]
fn css_validation_and_injection_inputs() {
    assert!(validate_css("body {color: red;").is_err());
    assert!(validate_css("@import 'https://evil.com';").is_err());
    assert!(validate_css("a{background:url(file:///secret)}").is_err());
    assert!(validate_css("body {color: #fff}").is_ok());
    let mut p = Profile::default();
    p.accent = "red;}body{".into();
    assert!(validate_profile(&p).is_err());
}
#[test]
fn zip_traversal_is_rejected() {
    let (_t, e) = fixture();
    let path = e.root.join("bad.zip");
    let mut z = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    z.start_file("../escape", zip::write::SimpleFileOptions::default())
        .unwrap();
    z.write_all(b"bad").unwrap();
    z.finish().unwrap();
    assert!(extract(&path, &e.state).is_err());
}
#[test]
fn lock_blocks_concurrent_operation() {
    use fs2::FileExt;
    let (_t, e) = fixture();
    fs::create_dir(&e.state).unwrap();
    let f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(e.state.join("operation.lock"))
        .unwrap();
    f.lock_exclusive().unwrap();
    assert!(e.backup().is_err());
}
#[test]
fn modified_archive_not_mistaken_for_stock() {
    let (_t, e) = fixture();
    let mut z = zip::ZipWriter::new(fs::File::create(e.root.join("Apps/xpui.spa")).unwrap());
    z.start_file("index.html", zip::write::SimpleFileOptions::default())
        .unwrap();
    z.write_all(b"<html>spicetify xpui-snapshot.js</html>")
        .unwrap();
    z.finish().unwrap();
    assert!(e.backup().is_err());
}
#[test]
fn interrupted_finalization_finishes_verified_commit() {
    let (_t, e) = fixture();
    e.apply(&Profile::default(), false).unwrap();
    let mut p = Profile::default();
    p.theme = "ink".into();
    assert!(e
        .apply_with_failure(&p, false, Some("after-finalize"))
        .is_err());
    e.recover().unwrap();
    assert!(e.status().unwrap().verified);
    assert_eq!(e.status().unwrap().active.unwrap().theme, "ink");
}
#[test]
fn archive_relocation_interruption_restores_stock_archive(){let(_t,e)=fixture();let original=e.identity().unwrap();assert!(e.apply_with_failure(&Profile::default(),false,Some("after-archive")).is_err());assert!(!e.root.join("Apps/xpui.spa").exists());e.recover().unwrap();assert!(e.root.join("Apps/xpui.spa").exists());assert_eq!(original,e.identity().unwrap());}
#[test]
fn marketplace_subset_converts_known_css_and_blocks_unknown_api(){use resonance_engine::store::*;let css=convert_css(".Root__now-playing-bar {border-radius:16px}");assert!(css.contains("data-testid"));assert!(assess("theme",&css).is_empty());assert!(!assess("extension","Spicetify.Player.togglePlay();").is_empty());assert!(assess("extension","Spicetify.showNotification('ok');").is_empty());}
