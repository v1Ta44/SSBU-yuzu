use serde_json::json;
use ssbu_updater::*;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("ssbu-update-test-{}", unique()));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        if self.0.starts_with(std::env::temp_dir())
            && self
                .0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("ssbu-update-test-")
        {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
struct Fixture {
    temp: Temp,
    root: PathBuf,
    key: Vec<u8>,
    trust: Trust,
}
fn bytes(p: &Path) -> Vec<u8> {
    fs::read(p).unwrap()
}
fn write(p: &Path, b: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, b).unwrap();
}
fn features() -> Features {
    Features {
        resource_mods: true,
        latency_slider: true,
    }
}
fn version(sequence: u64) -> Version {
    Version {
        schema: 1,
        channel: "yuzu-test".into(),
        platform: "win-x64".into(),
        sequence,
        version: format!("test.{sequence}"),
        build_id: format!("build-{sequence}"),
        game_version: "13.0.4".into(),
        features: features(),
        minimum_updater: 1,
    }
}
impl Fixture {
    fn new() -> Self {
        let temp = Temp::new();
        let root = temp.0.join("包 with spaces");
        fs::create_dir_all(&root).unwrap();
        let (key, public) = generate_key().unwrap();
        let trust = Trust {
            keys: BTreeMap::from([("test".into(), public)]),
        };
        let files = [
            ("SSBU Console.exe", b"old-console".to_vec()),
            ("game/ssbu_native.exe", b"old-game".to_vec()),
            (
                "game/ssbu_native.build.json",
                serde_json::to_vec(&features()).unwrap(),
            ),
        ];
        let mut map = serde_json::Map::new();
        for (n, b) in files {
            write(&root.join(n), &b);
            map.insert(n.into(), json!({"size":b.len(),"sha256":hash_bytes(&b)}));
        }
        write(
            &root.join("manifest.json"),
            &serde_json::to_vec(
                &json!({"build_id":"old-build","name":"old","romfs":{"size":17},"files":map}),
            )
            .unwrap(),
        );
        for p in [
            "userdata/save/user.bin",
            "mods/custom.bin",
            "config/graphics.ini",
            "cache/vk/custom.bin",
            "logs/log.txt",
            "reports/report.txt",
            "romfs/data.arc",
            "launcher/launcher.ini",
            "game/net_delay/webview-data/Preferences",
            "game/personal.dll",
        ] {
            write(&root.join(p), b"user-owned");
        }
        Self {
            temp,
            root,
            key,
            trust,
        }
    }
    fn contents(&self, sequence: u64) -> (Package, BTreeMap<String, Vec<u8>>) {
        let v = version(sequence);
        let current = installed(&self.root, &self.trust).unwrap();
        let mut payload: BTreeMap<String, Vec<u8>> = BTreeMap::from([
            (
                "SSBU Console.exe".into(),
                format!("new-console-{sequence}").into_bytes(),
            ),
            ("game/new.dll".into(), b"new-managed-file".to_vec()),
            ("version.json".into(), serde_json::to_vec(&v).unwrap()),
        ]);
        let mut manifest: serde_json::Value = read_json(&self.root.join("manifest.json")).unwrap();
        // Full updates supply all target files, regardless of the old bytes.
        for name in manifest["files"].as_object().unwrap().keys() {
            if managed(name) && name != "manifest.json" && name != "version.json" {
                payload
                    .entry(name.clone())
                    .or_insert_with(|| bytes(&self.root.join(name)));
            }
        }
        manifest["build_id"] = json!(v.build_id);
        for (n, b) in &payload {
            manifest["files"][n] = json!({"size":b.len(),"sha256":hash_bytes(b)});
        }
        payload.insert(
            "manifest.json".into(),
            serde_json::to_vec(&manifest).unwrap(),
        );
        let changes = payload
            .iter()
            .map(|(path, b)| Change {
                path: path.clone(),
                size: b.len() as u64,
                sha256: hash_bytes(b),
                before: BTreeMap::from([(
                    current.build_id.clone(),
                    if self.root.join(path).exists() {
                        Some(hash_file(&self.root.join(path)).unwrap())
                    } else {
                        None
                    },
                )]),
            })
            .collect();
        (
            Package {
                version: v,
                kind: "full".into(),
                base_build_ids: vec![],
                notes: "fix".into(),
                files: changes,
                remove: vec![],
            },
            payload,
        )
    }
    fn archive(
        &self,
        p: &Package,
        mut payload: BTreeMap<String, Vec<u8>>,
        extra: Option<(&str, &[u8])>,
    ) -> PathBuf {
        let zip = self.temp.0.join(format!("{}.zip", unique()));
        let mut w = zip::ZipWriter::new(fs::File::create(&zip).unwrap());
        let opt = zip::write::SimpleFileOptions::default();
        let envelope = sign(&serde_json::to_vec(p).unwrap(), &self.key, "test").unwrap();
        w.start_file("update.json", opt).unwrap();
        w.write_all(&serde_json::to_vec(&envelope).unwrap())
            .unwrap();
        for (name, b) in &mut payload {
            w.start_file(format!("payload/{name}"), opt).unwrap();
            w.write_all(b).unwrap();
        }
        if let Some((name, b)) = extra {
            w.start_file(name, opt).unwrap();
            w.write_all(b).unwrap();
        }
        w.finish().unwrap();
        zip
    }
    fn prepare(&self, sequence: u64) -> PathBuf {
        let (p, b) = self.contents(sequence);
        stage_archive(
            &self.root,
            &self.archive(&p, b, None),
            &self.trust,
            |_, _| {},
        )
        .unwrap()
    }
}
#[test]
fn full_install_preserves_user_data_and_rollback_restores_exact_bytes() {
    let f = Fixture::new();
    let old = bytes(&f.root.join("manifest.json"));
    let link = f.temp.0.join("hardlink.exe");
    fs::hard_link(f.root.join("SSBU Console.exe"), &link).unwrap();
    let s = f.prepare(1);
    install(&f.root, &s, &f.trust).unwrap();
    assert_eq!(installed(&f.root, &f.trust).unwrap().sequence, 1);
    assert_eq!(bytes(&link), b"old-console");
    for p in [
        "userdata/save/user.bin",
        "mods/custom.bin",
        "config/graphics.ini",
        "cache/vk/custom.bin",
        "logs/log.txt",
        "reports/report.txt",
        "romfs/data.arc",
        "launcher/launcher.ini",
        "game/net_delay/webview-data/Preferences",
        "game/personal.dll",
    ] {
        assert_eq!(bytes(&f.root.join(p)), b"user-owned");
    }
    // Reinstalling this same signed version repairs drift without a downgrade.
    assert!(preflight(&f.root, &s, &f.trust, false).is_ok());
    rollback(&f.root).unwrap();
    assert_eq!(bytes(&f.root.join("manifest.json")), old);
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"old-console");
    assert!(!f.root.join("game/new.dll").exists());
    assert!(!f.root.join("version.json").exists());
    assert_eq!(installed(&f.root, &f.trust).unwrap().sequence, 0);
}
#[test]
fn partial_failure_automatically_rolls_back() {
    let f = Fixture::new();
    let s = f.prepare(1);
    assert!(install_with_hook(&f.root, &s, &f.trust, |i| if i == 1 {
        Err("disk error".into())
    } else {
        Ok(())
    })
    .unwrap_err()
    .contains("已恢复"));
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"old-console");
    assert!(!pending(&f.root));
}
#[test]
fn interrupted_replacement_is_recovered_next_run() {
    let f = Fixture::new();
    let s = f.prepare(1);
    let result = std::panic::catch_unwind(|| {
        let _ = install_with_hook(&f.root, &s, &f.trust, |i| {
            if i == 1 {
                panic!("simulated process interruption");
            }
            Ok(())
        });
    });
    assert!(result.is_err());
    assert!(pending(&f.root));
    recover(&f.root).unwrap();
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"old-console");
    assert!(!f.root.join("game/new.dll").exists());
    assert!(!pending(&f.root));
}
#[test]
fn bad_signature_and_unknown_publisher_are_rejected() {
    let f = Fixture::new();
    let (p, b) = f.contents(1);
    let archive = f.archive(&p, b, None);
    let other = Trust {
        keys: BTreeMap::new(),
    };
    assert!(stage_archive(&f.root, &archive, &other, |_, _| {}).is_err());
    let mut e = sign(b"{}", &f.key, "test").unwrap();
    e.payload = "e30g".into();
    assert!(verify::<Package>(&e, &f.trust).is_err());
}
#[test]
fn corrupt_payload_and_extra_entries_are_rejected() {
    let f = Fixture::new();
    let (p, mut b) = f.contents(1);
    b.insert("SSBU Console.exe".into(), b"bad-console-1".to_vec());
    let a = f.archive(&p, b, None);
    assert!(stage_archive(&f.root, &a, &f.trust, |_, _| {}).is_err());
    let (p, b) = f.contents(1);
    for extra in [
        "../escape.dll",
        "C:/escape.dll",
        "payload/../config/graphics.ini",
        "payload/game/extra.dll",
    ] {
        assert!(stage_archive(
            &f.root,
            &f.archive(&p, b.clone(), Some((extra, b"bad"))),
            &f.trust,
            |_, _| {}
        )
        .is_err());
    }
}
#[test]
fn full_overwrites_modified_target_but_rejects_inconsistent_new_features() {
    let f = Fixture::new();
    let s = f.prepare(1);
    write(&f.root.join("SSBU Console.exe"), b"locally-modified");
    install(&f.root, &s, &f.trust).unwrap();
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"new-console-1");
    rollback(&f.root).unwrap();
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"locally-modified");
    let (p, mut b) = f.contents(1);
    let mut wrong = p.clone();
    wrong.version.features.resource_mods = false;
    b.insert(
        "version.json".into(),
        serde_json::to_vec(&wrong.version).unwrap(),
    );
    assert!(stage_archive(&f.root, &f.archive(&wrong, b, None), &f.trust, |_, _| {}).is_err());
}
#[test]
fn full_replaces_unchanged_target_file_after_local_drift() {
    let f = Fixture::new();
    let s = f.prepare(1);
    write(&f.root.join("game/ssbu_native.exe"), b"modified");
    install(&f.root, &s, &f.trust).unwrap();
    assert_eq!(bytes(&f.root.join("game/ssbu_native.exe")), b"old-game");
    rollback(&f.root).unwrap();
    assert_eq!(bytes(&f.root.join("game/ssbu_native.exe")), b"modified");
}
#[test]
fn signed_target_path_is_overwritten_and_actual_bytes_backed_up() {
    let f = Fixture::new();
    let (mut p, b) = f.contents(1);
    p.kind = "full".into();
    p.base_build_ids.clear();
    let s = stage_archive(&f.root, &f.archive(&p, b, None), &f.trust, |_, _| {}).unwrap();
    write(&f.root.join("game/new.dll"), b"personal-file");
    install(&f.root, &s, &f.trust).unwrap();
    assert_eq!(bytes(&f.root.join("game/new.dll")), b"new-managed-file");
    rollback(&f.root).unwrap();
    assert_eq!(bytes(&f.root.join("game/new.dll")), b"personal-file");
}
#[test]
fn corrupted_backup_and_post_install_edits_refuse_rollback() {
    let f = Fixture::new();
    let s = f.prepare(1);
    install(&f.root, &s, &f.trust).unwrap();
    write(&f.root.join("SSBU Console.exe"), b"post-install-edit");
    assert!(rollback(&f.root).is_err());
    assert_eq!(
        bytes(&f.root.join("SSBU Console.exe")),
        b"post-install-edit"
    );
    write(&f.root.join("SSBU Console.exe"), b"new-console-1");
    let j = journal(&f.root).unwrap().unwrap();
    write(
        &f.root
            .join(format!("updates/backup/{}/SSBU Console.exe", j.id)),
        b"damaged",
    );
    assert!(rollback(&f.root).is_err());
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"new-console-1");
}
#[test]
fn repeated_updates_and_rollback_restore_previous_signed_version() {
    let f = Fixture::new();
    let s = f.prepare(1);
    install(&f.root, &s, &f.trust).unwrap();
    let next = f.prepare(2);
    install(&f.root, &next, &f.trust).unwrap();
    assert_eq!(installed(&f.root, &f.trust).unwrap().sequence, 2);
    rollback(&f.root).unwrap();
    assert_eq!(installed(&f.root, &f.trust).unwrap().sequence, 1);
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"new-console-1");
}
#[test]
fn paths_and_personal_directories_are_protected() {
    for p in [
        "../evil",
        "/evil",
        "C:/evil",
        "a\\b",
        "a:stream",
        "a/../b",
        "game/con.dll",
        "game/NUL",
        "game/a.",
        "game/a ",
        "game/a//b",
        "game/com1.exe",
    ] {
        assert!(relative(p).is_err(), "{p}");
    }
    for p in [
        "romfs/data.arc",
        "RomFS/Data.Arc",
        "userdata/save/a",
        "config/graphics.ini",
        "mods/custom.dll",
        "cache/vk/a",
        "logs/a",
        "reports/a",
        "launcher/launcher.ini",
        "game/net_delay/webview-data/Default/Preferences",
    ] {
        assert!(!managed(p), "{p}");
    }
    assert!(managed("romfs/.nrr/cross2.nrr"));
}
#[test]
fn signed_protected_path_and_oversized_package_are_rejected() {
    let f = Fixture::new();
    let (mut p, b) = f.contents(1);
    p.files[0].path = "config/graphics.ini".into();
    assert!(stage_archive(&f.root, &f.archive(&p, b, None), &f.trust, |_, _| {}).is_err());
    let (mut p, b) = f.contents(1);
    p.files[0].size = MAX_EXTRACTED + 1;
    assert!(stage_archive(&f.root, &f.archive(&p, b, None), &f.trust, |_, _| {}).is_err());
}
#[cfg(windows)]
#[test]
fn console_and_locked_executable_block_installation() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    let lock = RootLock::acquire(&f.root).unwrap();
    assert!(RootLock::acquire(&f.root).is_err());
    drop(lock);
    let s = f.prepare(1);
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(f.root.join("SSBU Console.exe"))
        .unwrap();
    assert!(install(&f.root, &s, &f.trust).is_err());
    drop(held);
    install(&f.root, &s, &f.trust).unwrap();
}

#[test]
fn concurrent_staging_identifiers_are_unique() {
    let workers: Vec<_> = (0..8)
        .map(|_| std::thread::spawn(|| (0..1000).map(|_| unique()).collect::<Vec<_>>()))
        .collect();
    let names: std::collections::HashSet<_> = workers
        .into_iter()
        .flat_map(|w| w.join().unwrap())
        .collect();
    assert_eq!(names.len(), 8000);
}

#[test]
fn full_repairs_missing_modified_and_already_target_files_and_exactly_rolls_back() {
    let f = Fixture::new();
    let (p, payload) = f.contents(1);
    let target = payload.clone();
    let archive = f.archive(&p, payload, None);
    fs::remove_file(f.root.join("game/ssbu_native.exe")).unwrap();
    write(
        &f.root.join("game/ssbu_native.build.json"),
        b"{broken record",
    );
    write(
        &f.root.join("SSBU Console.exe"),
        &target["SSBU Console.exe"],
    );
    let stage = stage_archive(&f.root, &archive, &f.trust, |_, _| {}).unwrap();
    install(&f.root, &stage, &f.trust).unwrap();
    for (path, bytes) in target {
        assert_eq!(fs::read(f.root.join(path)).unwrap(), bytes);
    }
    rollback(&f.root).unwrap();
    assert!(!f.root.join("game/ssbu_native.exe").exists());
    assert_eq!(
        bytes(&f.root.join("game/ssbu_native.build.json")),
        b"{broken record"
    );
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"new-console-1");
}
#[test]
fn full_crosses_old_feature_flags_and_ignores_local_build_identity() {
    let f = Fixture::new();
    let (p, payload) = f.contents(1);
    let archive = f.archive(&p, payload, None);
    write(
        &f.root.join("game/ssbu_native.build.json"),
        br#"{"resource_mods":false,"latency_slider":false}"#,
    );
    write(
        &f.root.join("manifest.json"),
        br#"{"build_id":"locally-changed"}"#,
    );
    let stage = stage_archive(&f.root, &archive, &f.trust, |_, _| {}).unwrap();
    install(&f.root, &stage, &f.trust).unwrap();
    assert_eq!(
        installed(&f.root, &f.trust).unwrap().features,
        p.version.features
    );
}
#[test]
fn same_signed_version_repairs_edited_version_records() {
    let f = Fixture::new();
    let stage = f.prepare(1);
    install(&f.root, &stage, &f.trust).unwrap();
    write(&f.root.join("version.json"), b"{changed");
    write(&f.root.join("manifest.json"), b"{changed");
    write(&f.root.join("game/ssbu_native.exe"), b"changed runtime");
    assert_eq!(installed(&f.root, &f.trust).unwrap().sequence, 1);
    install(&f.root, &stage, &f.trust).unwrap();
    assert_eq!(bytes(&f.root.join("game/ssbu_native.exe")), b"old-game");
    assert_eq!(installed(&f.root, &f.trust).unwrap().sequence, 1);
}
#[test]
fn damaged_local_installed_record_can_be_repaired_and_restored_verbatim() {
    let f = Fixture::new();
    let stage = f.prepare(1);
    let damaged = b"malformed previous installed record\r\n";
    write(&f.root.join("updates/installed.json"), damaged);
    install(&f.root, &stage, &f.trust).unwrap();
    assert_eq!(installed(&f.root, &f.trust).unwrap().sequence, 1);
    rollback(&f.root).unwrap();
    assert_eq!(bytes(&f.root.join("updates/installed.json")), damaged);
}
#[test]
fn incomplete_full_and_legacy_patch_are_rejected_before_replacement() {
    let f = Fixture::new();
    let (mut p, mut payload) = f.contents(1);
    p.files.retain(|x| x.path != "game/ssbu_native.exe");
    payload.remove("game/ssbu_native.exe");
    assert!(
        stage_archive(&f.root, &f.archive(&p, payload, None), &f.trust, |_, _| {})
            .unwrap_err()
            .contains("完整")
    );
    let (mut p, payload) = f.contents(1);
    p.kind = "patch".into();
    p.base_build_ids = vec!["old-build".into()];
    assert!(
        stage_archive(&f.root, &f.archive(&p, payload, None), &f.trust, |_, _| {})
            .unwrap_err()
            .contains("完整更新包")
    );
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"old-console");
}
#[test]
fn signed_removal_ignores_old_hash_and_tolerates_already_missing_path() {
    for missing in [false, true] {
        let f = Fixture::new();
        let (mut p, payload) = f.contents(1);
        p.remove.push(Removal {
            path: "game/retired.dll".into(),
            before: BTreeMap::from([("old-build".into(), hash_bytes(b"original"))]),
        });
        if !missing {
            write(&f.root.join("game/retired.dll"), b"local edit");
        }
        let stage =
            stage_archive(&f.root, &f.archive(&p, payload, None), &f.trust, |_, _| {}).unwrap();
        install(&f.root, &stage, &f.trust).unwrap();
        assert!(!f.root.join("game/retired.dll").exists());
    assert!(!f.root.join("game/ssbu_game_eh_o1.dll").exists());
        rollback(&f.root).unwrap();
        if !missing {
            assert_eq!(bytes(&f.root.join("game/retired.dll")), b"local edit");
        }
    }
}
#[test]
fn full_failure_restores_modified_files_and_raw_installed_record() {
    let f = Fixture::new();
    let stage = f.prepare(1);
    write(&f.root.join("SSBU Console.exe"), b"locally modified");
    write(&f.root.join("updates/installed.json"), b"broken");
    assert!(install_with_hook(&f.root, &stage, &f.trust, |i| if i == 2 {
        Err("write failure".into())
    } else {
        Ok(())
    })
    .is_err());
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"locally modified");
    assert_eq!(bytes(&f.root.join("updates/installed.json")), b"broken");
    assert!(!pending(&f.root));
}
#[test]
fn valid_signed_local_version_still_blocks_downgrade() {
    let f = Fixture::new();
    let old = f.prepare(1);
    let new = f.prepare(2);
    install(&f.root, &new, &f.trust).unwrap();
    assert!(preflight(&f.root, &old, &f.trust, false)
        .unwrap_err()
        .contains("旧"));
}

#[test]
fn release_generator_full_package_repairs_without_opening_any_base_installation() {
    use std::process::Command;
    let f = Fixture::new();
    let package = f.temp.0.join("source");
    let output = f.temp.0.join("release");
    let helper = PathBuf::from(env!("CARGO_BIN_EXE_ssbu-updater"));
    let executable = bytes(&helper);
    let host = json!({"schema_version":2,"resource_mods":false,"latency_slider":false,"skyline_loader":true,"exe_sha256":hash_bytes(&executable)});
    let files = BTreeMap::from([
        ("game/plugin_bin/sky_arcropolis.dll", b"compiled-arc".to_vec()),
        ("game/plugin_bin/plugins.txt", b"portable-registry".to_vec()),
        ("sd/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libarcropolis.nro", b"original-arc".to_vec()),
        ("game/plugin_bin/arcropolis.json", serde_json::to_vec(&json!({"dll_sha256":hash_bytes(b"compiled-arc"),"nro_sha256":hash_bytes(b"original-arc")})).unwrap()),
        ("SSBU Console.exe", executable.clone()),
        ("game/ssbu_native.exe", executable),
        (
            "game/ssbu_native.build.json",
            serde_json::to_vec(&host).unwrap(),
        ),
    ]);
    let mut records = serde_json::Map::new();
    for (path, data) in &files {
        write(&package.join(path), data);
        records.insert(
            (*path).into(),
            json!({"size":data.len(),"sha256":hash_bytes(data)}),
        );
    }
    write(
        &package.join("manifest.json"),
        &serde_json::to_vec(&json!({"build_id":"source","files":records})).unwrap(),
    );
    let key = f.temp.0.join("diagnostic-key.pk8");
    write(&key, &f.key);
    let notes = f.temp.0.join("notes.txt");
    write(&notes, b"private regression only");
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/release_update.py");
    let run = Command::new("python")
        .arg("-X")
        .arg("utf8")
        .arg(script)
        .arg("--package")
        .arg(&package)
        .arg("--out")
        .arg(&output)
        .arg("--helper")
        .arg(&helper)
        .arg("--key")
        .arg(&key)
        .args([
            "--key-id",
            "test",
            "--version",
            "diagnostic",
            "--build-id",
            "replacement",
            "--sequence",
            "1",
            "--kind",
            "full",
            "--remove",
            "game/retired.dll",
        ])
        .arg("--base")
        .arg(f.temp.0.join("does-not-exist"))
        .arg("--notes")
        .arg(&notes)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let archive = output.join("yuzu-test-diagnostic-full.zip");
    write(&f.root.join("game/ssbu_game_eh_o1.dll"), b"retired large game library");
    write(
        &f.root.join("game/ssbu_native.build.json"),
        b"invalid local JSON",
    );
    fs::remove_file(f.root.join("SSBU Console.exe")).unwrap();
    write(&f.root.join("game/retired.dll"), b"any old contents");
    let stage = stage_archive(&f.root, &archive, &f.trust, |_, _| {}).unwrap();
    let (_, plan) = stage_package(&stage, &f.trust).unwrap();
    assert_eq!(plan.version.minimum_updater, 2);
    assert_eq!(plan.kind, "full");
    install(&f.root, &stage, &f.trust).unwrap();
    for (path, data) in files {
        assert_eq!(bytes(&f.root.join(path)), data);
    }
    assert!(!f.root.join("game/retired.dll").exists());
    assert_eq!(bytes(&f.root.join("config/graphics.ini")), b"user-owned");
    rollback(&f.root).unwrap();
    assert_eq!(bytes(&f.root.join("game/retired.dll")), b"any old contents");
    assert_eq!(bytes(&f.root.join("game/ssbu_game_eh_o1.dll")), b"retired large game library");
    assert_eq!(
        bytes(&f.root.join("game/ssbu_native.build.json")),
        b"invalid local JSON"
    );
    assert!(!f.root.join("SSBU Console.exe").exists());
}

#[test]
fn cleanup_after_success_removes_transients_keeps_only_latest_rollback_and_personal_data() {
    let f = Fixture::new();
    let first = f.prepare(1);
    install(&f.root, &first, &f.trust).unwrap();
    let old = journal(&f.root).unwrap().unwrap();
    let second = f.prepare(2);
    install(&f.root, &second, &f.trust).unwrap();
    let latest = journal(&f.root).unwrap().unwrap();
    let download = f
        .root
        .join(format!("updates/downloads/{}.download", unique()));
    write(&download, b"downloaded zip");
    let failed = f
        .root
        .join(format!("updates/staging/{}/payload/failed.dll", unique()));
    write(&failed, b"failed extraction");
    let helper = f
        .root
        .join(format!("updates/helpers/{}/SSBU Updater.exe", unique()));
    write(&helper, b"old helper");
    let manual = f.root.join("updates/downloads/My saved update.zip");
    write(&manual, b"manual archive");
    let import = f.root.join("offline-update.zip");
    write(&import, b"user import");
    let lock = RootLock::acquire(&f.root).unwrap();
    let report = cleanup::after_success(&f.root, &lock).unwrap();
    drop(lock);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(report.removed_files > 0);
    assert!(!download.exists() && !failed.exists() && !helper.exists());
    assert!(!first.exists() && !second.exists());
    assert!(!f.root.join(format!("updates/backup/{}", old.id)).exists());
    assert!(f
        .root
        .join(format!("updates/backup/{}", latest.id))
        .exists());
    assert!(!f
        .root
        .join(format!("updates/history/{}.json", old.id))
        .exists());
    assert_eq!(bytes(&manual), b"manual archive");
    assert_eq!(bytes(&import), b"user import");
    for path in [
        "config/graphics.ini",
        "mods/custom.bin",
        "userdata/save/user.bin",
        "romfs/data.arc",
        "game/personal.dll",
    ] {
        assert_eq!(bytes(&f.root.join(path)), b"user-owned");
    }
    rollback(&f.root).unwrap();
    assert_eq!(installed(&f.root, &f.trust).unwrap().sequence, 1);
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"new-console-1");
}
#[test]
fn cleanup_refuses_unfinished_transaction_and_preserves_all_recovery_material() {
    let f = Fixture::new();
    let stage = f.prepare(1);
    install(&f.root, &stage, &f.trust).unwrap();
    let mut j = journal(&f.root).unwrap().unwrap();
    j.phase = "applying".into();
    save_json(&f.root.join("updates/transaction.json"), &j).unwrap();
    let archive = f
        .root
        .join(format!("updates/downloads/{}.download", unique()));
    write(&archive, b"keep");
    let lock = RootLock::acquire(&f.root).unwrap();
    assert!(cleanup::after_success(&f.root, &lock).is_err());
    assert!(stage.exists() && archive.exists());
    assert!(f.root.join(format!("updates/backup/{}", j.id)).exists());
}
#[test]
fn cleanup_never_discards_older_backup_when_current_backup_or_target_is_damaged() {
    for damage_backup in [false, true] {
        let f = Fixture::new();
        let first = f.prepare(1);
        install(&f.root, &first, &f.trust).unwrap();
        let old = journal(&f.root).unwrap().unwrap();
        let second = f.prepare(2);
        install(&f.root, &second, &f.trust).unwrap();
        let current = journal(&f.root).unwrap().unwrap();
        let path = if damage_backup {
            f.root
                .join(format!("updates/backup/{}/SSBU Console.exe", current.id))
        } else {
            f.root.join("SSBU Console.exe")
        };
        write(&path, b"unexpected damage");
        let lock = RootLock::acquire(&f.root).unwrap();
        assert!(cleanup::after_success(&f.root, &lock).is_err());
        assert!(first.exists() && second.exists());
        assert!(f.root.join(format!("updates/backup/{}", old.id)).exists());
    }
}
#[cfg(windows)]
#[test]
fn cleanup_locked_temporary_file_is_nonfatal_and_retains_rollback() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    let stage = f.prepare(1);
    install(&f.root, &stage, &f.trust).unwrap();
    let archive = f
        .root
        .join(format!("updates/downloads/{}.download", unique()));
    write(&archive, b"locked zip");
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&archive)
        .unwrap();
    let lock = RootLock::acquire(&f.root).unwrap();
    let report = cleanup::after_success(&f.root, &lock).unwrap();
    assert!(!report.warnings.is_empty());
    assert!(archive.exists());
    drop(held);
    drop(lock);
    assert_eq!(installed(&f.root, &f.trust).unwrap().sequence, 1);
    rollback(&f.root).unwrap();
}
#[cfg(windows)]
#[test]
fn cleanup_does_not_follow_directory_junction_or_remove_its_target() {
    use std::process::Command;
    let f = Fixture::new();
    let stage = f.prepare(1);
    install(&f.root, &stage, &f.trust).unwrap();
    let outside = f.temp.0.join("outside");
    write(&outside.join("precious.bin"), b"preserve");
    let candidate = f.root.join(format!("updates/staging/{}", unique()));
    fs::create_dir_all(&candidate).unwrap();
    let junction = candidate.join("linked");
    let q = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside)
        .output()
        .unwrap();
    assert!(q.status.success());
    let lock = RootLock::acquire(&f.root).unwrap();
    let report = cleanup::after_success(&f.root, &lock).unwrap();
    drop(lock);
    assert!(!report.warnings.is_empty());
    assert!(candidate.exists());
    assert_eq!(bytes(&outside.join("precious.bin")), b"preserve");
    // Remove only the junction itself so fixture teardown cannot traverse it.
    fs::remove_dir(&junction).unwrap();
}
#[test]
fn cleanup_rejects_lock_from_another_installation() {
    let f = Fixture::new();
    let stage = f.prepare(1);
    install(&f.root, &stage, &f.trust).unwrap();
    let other = Fixture::new();
    let lock = RootLock::acquire(&other.root).unwrap();
    assert!(cleanup::after_success(&f.root, &lock).is_err());
    assert!(stage.exists());
}

#[test]
fn cleanup_running_helper_worker() {
    let Some(root) = std::env::var_os("SSBU_TEST_CLEANUP_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    assert!(root.starts_with(std::env::temp_dir()));
    assert!(root
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("ssbu-update-test-"));
    let lock = RootLock::acquire(&root).unwrap();
    let report = cleanup::after_success(&root, &lock).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}
#[test]
fn cleanup_skips_current_helper_process_but_removes_obsolete_helper() {
    let f = Fixture::new();
    let stage = f.prepare(1);
    install(&f.root, &stage, &f.trust).unwrap();
    let current = f
        .root
        .join(format!("updates/helpers/{}/SSBU Updater.exe", unique()));
    fs::create_dir_all(current.parent().unwrap()).unwrap();
    fs::copy(std::env::current_exe().unwrap(), &current).unwrap();
    let obsolete = f
        .root
        .join(format!("updates/helpers/{}/SSBU Updater.exe", unique()));
    write(&obsolete, b"old helper");
    let run = std::process::Command::new(&current)
        .args(["--exact", "cleanup_running_helper_worker", "--nocapture"])
        .env("SSBU_TEST_CLEANUP_ROOT", &f.root)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(current.exists());
    assert!(!obsolete.exists());
    assert!(!stage.exists());
    rollback(&f.root).unwrap();
    assert_eq!(bytes(&f.root.join("SSBU Console.exe")), b"old-console");
}
#[test]
fn skyline_update_paths_are_narrow() {
    for path in ["game/plugin_bin/sky_arcropolis.dll", "game/plugin_bin/plugins.txt", "game/plugin_bin/arcropolis.json", "sd/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libarcropolis.nro"] {
        assert!(managed(path), "{path}");
    }
    for path in ["sd/ultimate/mods/mine/config.json", "sd/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/other.nro", "game/plugin_bin/other.dll"] {
        assert!(!managed(path), "{path}");
    }
}

#[test]
fn skyline_host_full_update_requires_original_and_compiled_loader() {
    let f = Fixture::new();
    let (mut plan, mut payload) = f.contents(2);
    let mut stamp: serde_json::Value = serde_json::from_slice(&payload["game/ssbu_native.build.json"]).unwrap();
    stamp["skyline_loader"] = json!(true);
    let stamp_bytes = serde_json::to_vec(&stamp).unwrap();
    payload.insert("game/ssbu_native.build.json".into(), stamp_bytes.clone());
    let mut manifest: serde_json::Value = serde_json::from_slice(&payload["manifest.json"]).unwrap();
    manifest["files"]["game/ssbu_native.build.json"] = json!({"size":stamp_bytes.len(), "sha256":hash_bytes(&stamp_bytes)});
    payload.insert("manifest.json".into(),serde_json::to_vec(&manifest).unwrap());
    for record in &mut plan.files {
        let data = &payload[&record.path];
        record.size=data.len() as u64; record.sha256=hash_bytes(data);
    }
    let error=stage_archive(&f.root,&f.archive(&plan,payload,None),&f.trust,|_,_|{}).unwrap_err();
    assert!(error.contains("Skyline / ARCropolis"), "{error}");
}
