//! File management for upstream Skyline / ARCropolis. Never interprets or patches data.arc.
use crate::{ini::Ini, paths::Layout};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
pub static LOCK: Mutex<()> = Mutex::new(());
const PLUGINS: &str = "atmosphere/contents/01006A800016E000/romfs/skyline/plugins";
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn read_json(path: &Path) -> Result<Value, String> {
    match fs::read(path) {
        Ok(b) => serde_json::from_slice(&b).map_err(err),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(err(e)),
    }
}
fn absolute(l: &Layout, s: &str) -> PathBuf {
    let p = PathBuf::from(s);
    if p.is_absolute() {
        p
    } else {
        l.game_dir.join(p)
    }
}
fn configured(l: &Layout, name: &str) -> Option<String> {
    Ini::load(&l.launcher_ini())
        .get(Some("env"), name)
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var(name).ok().filter(|s| !s.is_empty()))
}
fn sd(l: &Layout) -> PathBuf {
    configured(l, "SSBU_SD")
        .map(|s| absolute(l, &s))
        .unwrap_or_else(|| l.root.join("sd"))
}
/// The manager and child must address the same SD. Per-run overrides cannot silently switch it.
pub fn configure(l: &Layout, env: &mut Map<String, Value>) -> Result<(), String> {
    for key in [
        "SSBU_RESOURCE_MODS",
        "SSBU_MOD_PROFILE",
        "SSBU_MOD_CACHE",
        "SSBU_MOD_REGION",
    ] {
        if env.keys().any(|k| k.eq_ignore_ascii_case(key)) || std::env::var_os(key).is_some() {
            return Err(format!(
                "旧模组设置 {key} 已停用；请移除它，使用模组页和 SSBU_SD 管理原生加载目录"
            ));
        }
    }
    let expected = sd(l);
    if let Some((_, v)) = env.iter().find(|(k, _)| k.eq_ignore_ascii_case("SSBU_SD")) {
        let supplied = absolute(l, v.as_str().ok_or("SSBU_SD 必须是路径")?);
        if normalized(&supplied) != normalized(&expected) {
            return Err(
                "本次 SD 路径与模组管理目录不同；请在启动设置中保存 SSBU_SD，再管理和启动模组"
                    .into(),
            );
        }
    }
    env.retain(|k, _| !k.eq_ignore_ascii_case("SSBU_SD"));
    env.insert("SSBU_SD".into(), json!(expected));
    Ok(())
}
fn normalized(p: &Path) -> String {
    let mut out = PathBuf::new();
    for c in p.components() {
        if c == std::path::Component::ParentDir {
            out.pop();
        } else if c != std::path::Component::CurDir {
            out.push(c.as_os_str());
        }
    }
    out.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\")
        .to_lowercase()
}
struct Session {
    sd: PathBuf,
    root: PathBuf,
    disabled: PathBuf,
    storage: PathBuf,
    metadata: PathBuf,
}
fn session(l: &Layout) -> Session {
    let sd = sd(l);
    let storage = sd.join(".console-mods");
    Session {
        root: sd.join("ultimate/mods"),
        disabled: storage.join("disabled"),
        storage,
        sd,
        metadata: l.config_dir().join("console-mods.json"),
    }
}
// Check every existing ancestor, including junctions, before creating or moving managed files.
fn safe_path(path: &Path) -> Result<(), String> {
    for p in path.ancestors() {
        if p.exists() {
            not_link(p)?;
        }
    }
    Ok(())
}
fn mkdir(path: &Path) -> Result<(), String> {
    safe_path(path)?;
    fs::create_dir_all(path).map_err(err)
}
fn valid_id(id: &str) -> bool {
    let stem = id.split('.').next().unwrap_or("").to_ascii_uppercase();
    !id.is_empty()
        && !id.starts_with('.')
        && !id.ends_with(['.', ' '])
        && !id
            .chars()
            .any(|c| c.is_control() || "/\\:<>\"|?*".contains(c))
        && ![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(&stem.as_str())
}
fn installed(s: &Session) -> Result<BTreeMap<String, (String, PathBuf, bool)>, String> {
    let mut out = BTreeMap::new();
    for (root, enabled) in [(&s.root, true), (&s.disabled, false)] {
        safe_path(root)?;
        if !root.exists() {
            continue;
        }
        for e in fs::read_dir(root).map_err(err)? {
            let e = e.map_err(err)?;
            let id = e.file_name().to_string_lossy().into_owned();
            if id.starts_with('.') {
                continue;
            }
            not_link(&e.path())?;
            if !e.file_type().map_err(err)?.is_dir() {
                continue;
            }
            if !valid_id(&id) {
                return Err(format!("模组文件夹名称无法管理：{id}"));
            }
            if out
                .insert(id.to_lowercase(), (id.clone(), e.path(), enabled))
                .is_some()
            {
                return Err(format!("启用和停用目录存在重名模组：{id}，请先手动整理"));
            }
        }
    }
    Ok(out)
}
fn locate(s: &Session, id: &str) -> Result<(PathBuf, bool), String> {
    if !valid_id(id) {
        return Err("模组名称无效".into());
    }
    let all = installed(s)?;
    let (actual, _, enabled) = all.get(&id.to_lowercase()).ok_or("找不到模组")?;
    Ok((
        package_path(if *enabled { &s.root } else { &s.disabled }, actual)?,
        *enabled,
    ))
}
fn metadata(s: &Session) -> Result<Value, String> {
    safe_path(&s.metadata)?;
    let v = read_json(&s.metadata)?;
    if v.get("schema_version").is_some_and(|n| n != 1) {
        return Err("模组管理配置版本较新，请先更新 Console".into());
    }
    if !v.is_object() || v.get("packages").is_some_and(|p| !p.is_object()) {
        return Err("模组管理配置损坏，请先恢复备份".into());
    }
    Ok(v)
}
fn save(s: &Session, v: &Value) -> Result<(), String> {
    let parent = s.metadata.parent().ok_or("配置路径无效")?;
    mkdir(parent)?;
    let temp = parent.join(format!("console-mods-{}.tmp", std::process::id()));
    safe_path(&temp)?;
    let mut value = v.clone();
    value["schema_version"] = json!(1);
    fs::write(&temp, serde_json::to_vec_pretty(&value).map_err(err)?).map_err(err)?;
    fs::rename(&temp, &s.metadata).map_err(|e| format!("管理配置未保存：{e}"))
}
fn inventory(l: &Layout, s: &Session) -> Result<Value, String> {
    let meta = metadata(s)?;
    let mut packages = vec![];
    for (_, (id, path, enabled)) in installed(s)? {
        let alias = meta["packages"][&id]["alias"].as_str().unwrap_or("");
        let plugins = if path.join("plugin.nro").is_file() {
            vec!["plugin.nro"]
        } else {
            vec![]
        };
        packages.push(json!({"id":id,"alias":alias,"enabled":enabled,"path":path,
            "tags":[],"fighters":[],"stages":[],"costumes":[],"paths":[],"plugins":plugins,"resource_count":0}));
    }
    // Presence is preparation evidence only; hook activation must be verified in a real run.
    let nro = s.sd.join(PLUGINS).join("libarcropolis.nro").is_file();
    let disabled = configured(l, "SSBU_SKYLINE").as_deref() == Some("0");
    let reason = if disabled {
        "启动设置 SSBU_SKYLINE=0 已关闭 Skyline；模组文件仍可管理，下次启动不会加载插件。"
    } else if nro {
        "已找到 ARCropolis 原插件；还需完成上游插件编译与挂钩验证，文件存在不代表已生效。"
    } else {
        "可管理模组；尚未找到 ARCropolis 原插件，资源加载尚未就绪。"
    };
    Ok(
        json!({"supported":true,"root":s.root,"trash":s.storage.join("trash"),"packages":packages,
        "reason":reason,"loader":{"arcropolis_nro":nro,"activation":"unverified","disabled":disabled},"diagnostics":[]}),
    )
}
pub fn list(l: &Layout) -> Result<Value, String> {
    inventory(l, &session(l))
}
pub fn update(l: &Layout, id: &str, patch: Value) -> Result<Value, String> {
    let s = session(l);
    let (source, enabled) = locate(&s, id)?;
    let patch = patch.as_object().ok_or("管理设置必须是对象")?;
    for (key, v) in patch {
        match key.as_str() {
            "enabled" if v.is_boolean() => (),
            "alias"
                if v.as_str().is_some_and(|a| {
                    a.chars().count() <= 120 && !a.chars().any(char::is_control)
                }) =>
            {
                ()
            }
            _ => return Err("这里只支持别名和启用开关；优先级和换槽交给原生加载方案".into()),
        }
    }
    let mut meta = metadata(&s)?;
    let actual = source
        .file_name()
        .ok_or("模组路径无效")?
        .to_string_lossy()
        .into_owned();
    if meta["packages"].is_null() {
        meta["packages"] = json!({});
    }
    if let Some(alias) = patch.get("alias") {
        meta["packages"][&actual] = json!({"alias":alias});
    }
    let wanted = patch
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(enabled);
    let moved = if wanted != enabled {
        let root = if wanted { &s.root } else { &s.disabled };
        mkdir(root)?;
        let target = root.join(&actual);
        if target.exists() {
            return Err("目标目录已有同名包".into());
        }
        fs::rename(&source, &target).map_err(|e| format!("启用状态未改变：{e}"))?;
        Some(target)
    } else {
        None
    };
    if let Err(e) = save(&s, &meta) {
        if let Some(target) = moved {
            fs::rename(target, &source)
                .map_err(|r| format!("{e}；恢复原位置失败：{r}，请检查 SD 目录"))?;
        }
        return Err(e);
    }
    inventory(l, &s)
}
pub fn delete(l: &Layout, id: &str) -> Result<Value, String> {
    let s = session(l);
    let (source, _) = locate(&s, id)?;
    let trash = s.storage.join("trash");
    mkdir(&trash)?;
    let target = trash.join(format!(
        "{}-{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S-%f"),
        id
    ));
    fs::rename(source, target).map_err(err)?;
    inventory(l, &s)
}
fn not_link(path: &Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    if fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .file_attributes()
        & 0x400
        != 0
    {
        return Err("不能操作链接或重解析目录，请使用真实文件夹".into());
    }
    Ok(())
}
fn package_path(root: &Path, id: &str) -> Result<PathBuf, String> {
    if !valid_id(id) {
        return Err("模组名称无效".into());
    }
    not_link(root)?;
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let p = root.join(id);
    not_link(&p)?;
    let real = fs::canonicalize(&p).map_err(|e| e.to_string())?;
    if real.parent() != Some(root.as_path()) || !real.is_dir() {
        return Err("模组路径超出安装目录".into());
    }
    Ok(real)
}
fn copy_tree(source: &Path, target: &Path) -> Result<(), String> {
    not_link(source)?;
    fs::create_dir_all(target).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        not_link(&path)?;
        let to = target.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &to)?;
        } else {
            fs::copy(path, to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn payload(path: &Path) -> bool {
    [
        "fighter",
        "sound",
        "ui",
        "stage",
        "effect",
        "camera",
        "stream;",
        "stream:",
        "config.json",
        "info.toml",
        "plugin.nro",
    ]
    .iter()
    .any(|s| path.join(s).exists())
        || fs::read_dir(path)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("0x"))
}
fn discover(path: &Path, depth: usize) -> Result<Vec<PathBuf>, String> {
    not_link(path)?;
    if depth > 7 {
        return Ok(vec![]);
    }
    if payload(path) {
        return Ok(vec![path.into()]);
    }
    let mut out = vec![];
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir()
            && !entry.file_name().to_string_lossy().starts_with('.')
        {
            out.extend(discover(&entry.path(), depth + 1)?);
        }
    }
    Ok(out)
}
fn extract(source: &Path, target: &Path) -> Result<(), String> {
    let file = fs::File::open(source).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("ZIP 无法读取：{e}"))?;
    if archive.len() > 100_000 {
        return Err("压缩包文件数量过多".into());
    }
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.enclosed_name().ok_or("压缩包包含越界路径")?;
        if name
            .components()
            .any(|c| c.as_os_str().to_string_lossy().contains([':', '\\']))
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err("压缩包包含链接或无效路径".into());
        }
        total = total.checked_add(entry.size()).ok_or("压缩包过大")?;
        if total > 16 * 1024 * 1024 * 1024 {
            return Err("解压内容超过 16 GB，请手动解压后导入文件夹".into());
        }
        let to = target.join(name);
        if entry.is_dir() {
            fs::create_dir_all(to).map_err(|e| e.to_string())?;
        } else {
            fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
            let mut output = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(to)
                .map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn import(l: &Layout, source: &Path) -> Result<Value, String> {
    let s = session(l);
    let existing = installed(&s)?;
    let mut meta = metadata(&s)?;
    if meta["packages"].is_null() {
        meta["packages"] = json!({});
    }
    // Preserve legacy settings and use compatible aliases / enabled choices only on explicit import.
    let legacy_path = l.config_dir().join("mods.json");
    let legacy = read_json(&legacy_path)?;
    if legacy_path.is_file() {
        let backup = l.config_dir().join("mods.before-skyline.json");
        safe_path(&backup)?;
        if !backup.exists() {
            fs::copy(&legacy_path, &backup).map_err(err)?;
        }
    }
    let staging = s.storage.join("staging");
    mkdir(&staging)?;
    let stage = staging.join(format!(
        "import-{}-{}",
        std::process::id(),
        chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    mkdir(&stage)?;
    let result = (|| -> Result<Vec<String>, String> {
        safe_path(source)?;
        if source.is_dir() {
            let from = normalized(&fs::canonicalize(source).map_err(err)?);
            let to = normalized(&fs::canonicalize(&stage).map_err(err)?);
            if to == from || to.starts_with(&(from + "\\")) {
                return Err("不能导入包含管理暂存区的文件夹；请选择其中的具体模组包".into());
            }
        }
        let payload_root = stage.join("payload");
        if source.is_dir() {
            copy_tree(source, &payload_root)?;
        } else {
            mkdir(&payload_root)?;
            extract(source, &payload_root)?;
        }
        let found = discover(&payload_root, 0)?;
        if found.is_empty() {
            return Err("未找到资源模组文件夹".into());
        }
        let mut planned = vec![];
        let mut ids = std::collections::BTreeSet::new();
        for path in found {
            let id = if path == payload_root {
                (if source.is_dir() {
                    source.file_name()
                } else {
                    source.file_stem()
                })
                .ok_or("源文件名称无效")?
                .to_string_lossy()
                .into_owned()
            } else {
                path.file_name().unwrap().to_string_lossy().into_owned()
            };
            if !valid_id(&id) {
                return Err(format!("模组文件夹名称无效：{id}"));
            }
            if existing.contains_key(&id.to_lowercase()) || !ids.insert(id.to_lowercase()) {
                return Err(format!("已有同名模组：{id}；请先整理后重试"));
            }
            let previous = &legacy["packages"][&id];
            let root = if previous["enabled"] == false {
                &s.disabled
            } else {
                &s.root
            };
            mkdir(root)?;
            if meta["packages"][&id].is_null() {
                if let Some(alias) = previous["alias"].as_str() {
                    meta["packages"][&id] = json!({"alias":alias});
                }
            }
            planned.push((path, root.join(&id), id));
        }
        let mut moved: Vec<(PathBuf, PathBuf)> = vec![];
        let applied = (|| -> Result<(), String> {
            for (from, to, _) in &planned {
                if to.exists() {
                    return Err("导入期间出现同名目录，操作已取消".into());
                }
                fs::rename(from, to).map_err(err)?;
                moved.push((from.clone(), to.clone()));
            }
            save(&s, &meta)
        })();
        if let Err(e) = applied {
            for (from, to) in moved.iter().rev() {
                fs::rename(to, from).map_err(|r| {
                    format!("{e}；恢复导入失败：{r}；文件保留在 {}", stage.display())
                })?;
            }
            return Err(e);
        }
        Ok(planned.into_iter().map(|(_, _, id)| id).collect())
    })();
    // Staging contains copied input only. Keep it on failure for diagnosis/recovery.
    if result.is_ok() {
        safe_path(&stage)?;
        fs::remove_dir_all(&stage).map_err(err)?;
    }
    let imported = result?;
    let mut out = inventory(l, &s)?;
    out["imported"] = json!(imported);
    Ok(out)
}
pub fn open(l: &Layout, id: Option<&str>) -> Result<Value, String> {
    let s = session(l);
    let path = if let Some(id) = id {
        locate(&s, id)?.0
    } else {
        mkdir(&s.root)?;
        s.root
    };
    crate::win::open_folder(&path)?;
    Ok(json!(true))
}
pub fn pick(kind: &str) -> Result<Value, String> {
    use std::os::windows::process::CommandExt;
    let script = match kind {
        "folder" => "Add-Type -AssemblyName System.Windows.Forms; $d=New-Object System.Windows.Forms.FolderBrowserDialog; $d.Description='选择已解压的模组文件夹'; if($d.ShowDialog() -eq 'OK'){[Console]::OutputEncoding=[Text.UTF8Encoding]::new(); [Console]::Write($d.SelectedPath)}",
        "zip" => "Add-Type -AssemblyName System.Windows.Forms; $d=New-Object System.Windows.Forms.OpenFileDialog; $d.Filter='ZIP (*.zip)|*.zip'; $d.Title='导入模组'; if($d.ShowDialog() -eq 'OK'){[Console]::OutputEncoding=[Text.UTF8Encoding]::new(); [Console]::Write($d.FileName)}",
        _ => return Err("导入类型无效".into()),
    };
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-STA", "-Command", script])
        .creation_flags(crate::game::CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("文件选择窗口无法打开".into());
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if path.is_empty() {
        Value::Null
    } else {
        json!(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(Layout);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }
    fn fixture(name: &str) -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "sky-console-{}-{}-{}",
            name,
            std::process::id(),
            chrono::Local::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir_all(root.join("config")).unwrap();
        Fixture(Layout {
            exe: root.join("game/ssbu_native.exe"),
            game_dir: root.join("game"),
            root,
            package: true,
        })
    }
    fn package(l: &Layout, name: &str) -> PathBuf {
        let p = l.root.join("original").join(name);
        fs::create_dir_all(p.join("fighter/mario/model/body/c00")).unwrap();
        fs::write(
            p.join("config.json"),
            b"{\"share-to-added\":true,\"untouched\":1}",
        )
        .unwrap();
        fs::write(
            p.join("fighter/mario/model/body/c00/model.nutexb"),
            b"unaltered texture bytes\x00\xff",
        )
        .unwrap();
        p
    }
    #[test]
    fn manager_needs_no_exe_backend_or_plugin_and_listing_is_read_only() {
        let f = fixture("list");
        let l = &f.0;
        let v = list(l).unwrap();
        assert_eq!(v["supported"], true);
        assert_eq!(v["loader"]["activation"], "unverified");
        assert!(!l.root.join("sd").exists());
        assert!(!l.config_dir().join("console-mods.json").exists());
    }
    #[test]
    fn lifecycle_moves_real_packages_and_preserves_original_bytes() {
        let f = fixture("lifecycle");
        let l = &f.0;
        let original = package(l, "Mario.v2");
        let config = fs::read(original.join("config.json")).unwrap();
        let resource = "fighter/mario/model/body/c00/model.nutexb";
        let bytes = fs::read(original.join(resource)).unwrap();
        assert_eq!(import(l, &original).unwrap()["imported"][0], "Mario.v2");
        let s = session(l);
        update(l, "Mario.v2", json!({"alias":"马力欧"})).unwrap();
        for enabled in [false, true, false, true] {
            let v = update(l, "Mario.v2", json!({"enabled":enabled})).unwrap();
            assert_eq!(v["packages"][0]["enabled"], enabled);
            assert_eq!(v["packages"][0]["alias"], "马力欧");
            assert_eq!(s.root.join("Mario.v2").exists(), enabled);
            assert_eq!(s.disabled.join("Mario.v2").exists(), !enabled);
            let installed = locate(&s, "Mario.v2").unwrap().0;
            assert_eq!(fs::read(installed.join("config.json")).unwrap(), config);
            assert_eq!(fs::read(installed.join(resource)).unwrap(), bytes);
        }
        delete(l, "Mario.v2").unwrap();
        assert_eq!(list(l).unwrap()["packages"].as_array().unwrap().len(), 0);
        assert_eq!(fs::read_dir(s.storage.join("trash")).unwrap().count(), 1);
        assert!(original.is_dir());
        assert_eq!(fs::read(original.join("config.json")).unwrap(), config);
    }
    #[test]
    fn invalid_patch_and_duplicate_import_leave_files_and_metadata_unchanged() {
        let f = fixture("atomic");
        let l = &f.0;
        let p = package(l, "Mario");
        import(l, &p).unwrap();
        let s = session(l);
        let meta = fs::read(&s.metadata).unwrap();
        assert!(update(l, "Mario", json!({"enabled":false,"slot_map":{}})).is_err());
        assert!(update(l, "Mario", json!({"alias":"bad\nname"})).is_err());
        assert!(import(l, &p).is_err());
        assert!(s.root.join("Mario").is_dir());
        assert!(!s.disabled.join("Mario").exists());
        assert_eq!(fs::read(&s.metadata).unwrap(), meta);
        for id in ["../Mario", "CON", "Mario.", "a/b", ""] {
            assert!(delete(l, id).is_err());
        }
    }
    #[test]
    fn metadata_failure_rolls_back_a_physical_toggle() {
        let f = fixture("rollback");
        let l = &f.0;
        let p = package(l, "Mario");
        import(l, &p).unwrap();
        let s = session(l);
        // An existing directory blocks the atomic metadata temp file, after the move has happened.
        fs::create_dir(
            l.config_dir()
                .join(format!("console-mods-{}.tmp", std::process::id())),
        )
        .unwrap();
        assert!(update(l, "Mario", json!({"enabled":false})).is_err());
        assert!(s.root.join("Mario").is_dir());
        assert!(!s.disabled.join("Mario").exists());
    }
    #[test]
    fn wrapper_import_preserves_legacy_settings_and_only_migrates_supported_fields() {
        let f = fixture("legacy");
        let l = &f.0;
        let wrapper = l.root.join("wrapper");
        let p = package(l, "Mario");
        fs::create_dir_all(wrapper.join("ultimate/mods")).unwrap();
        fs::rename(&p, wrapper.join("ultimate/mods/Mario")).unwrap();
        let legacy=b"{\"packages\":{\"Mario\":{\"enabled\":false,\"alias\":\"legacy\",\"priority\":10,\"slot_map\":{\"x\":8}}}}";
        fs::write(l.config_dir().join("mods.json"), legacy).unwrap();
        import(l, &wrapper).unwrap();
        let v = list(l).unwrap();
        assert_eq!(v["packages"][0]["enabled"], false);
        assert_eq!(v["packages"][0]["alias"], "legacy");
        assert_eq!(fs::read(l.config_dir().join("mods.json")).unwrap(), legacy);
        assert_eq!(
            fs::read(l.config_dir().join("mods.before-skyline.json")).unwrap(),
            legacy
        );
        assert!(!fs::read_to_string(session(l).metadata)
            .unwrap()
            .contains("slot_map"));
    }
    #[test]
    fn zip_import_is_byte_preserving_and_rejects_traversal() {
        use std::io::Write;
        let f = fixture("zip");
        let l = &f.0;
        let archive = l.root.join("mod.zip");
        let file = fs::File::create(&archive).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opt = zip::write::SimpleFileOptions::default();
        zip.start_file("ultimate/mods/ZipMod/config.json", opt)
            .unwrap();
        zip.write_all(b"unchanged original").unwrap();
        zip.finish().unwrap();
        import(l, &archive).unwrap();
        assert_eq!(
            fs::read(session(l).root.join("ZipMod/config.json")).unwrap(),
            b"unchanged original"
        );
        let file = fs::File::create(&archive).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("../outside", opt).unwrap();
        zip.write_all(b"escape").unwrap();
        zip.finish().unwrap();
        assert!(import(l, &archive).is_err());
        assert!(!l.root.join("outside").exists());
    }
    #[test]
    fn launch_path_matches_manager_and_blocks_per_run_sd_switch() {
        let f = fixture("env");
        let l = &f.0;
        let mut env = Map::new();
        configure(l, &mut env).unwrap();
        assert_eq!(env["SSBU_SD"], json!(session(l).sd));
        env.clear();
        env.insert("ssbu_sd".into(), json!(l.root.join("other")));
        assert!(configure(l, &mut env).is_err());
        fs::create_dir_all(l.root.join("launcher")).unwrap();
        fs::write(l.launcher_ini(), "[env]\nSSBU_SD=../custom-sd\n").unwrap();
        env.clear();
        configure(l, &mut env).unwrap();
        assert_eq!(env["SSBU_SD"], json!(l.game_dir.join("../custom-sd")));
        assert_eq!(
            session(l).root,
            l.game_dir.join("../custom-sd/ultimate/mods")
        );
    }
    #[test]
    fn importing_an_ancestor_of_staging_is_rejected() {
        let f = fixture("self-copy");
        let l = &f.0;
        assert!(import(l, &l.root).unwrap_err().contains("暂存区"));
        assert!(list(l).unwrap()["packages"].as_array().unwrap().is_empty());
    }
    #[test]
    fn legacy_env_is_rejected_and_skyline_opt_out_is_visible() {
        let f = fixture("legacy-env");
        let l = &f.0;
        let mut env = json!({"ssbu_resource_mods":"0"})
            .as_object()
            .unwrap()
            .clone();
        assert!(configure(l, &mut env).unwrap_err().contains("已停用"));
        fs::create_dir_all(l.root.join("launcher")).unwrap();
        fs::write(l.launcher_ini(), "[env]\nSSBU_SKYLINE=0\n").unwrap();
        assert_eq!(list(l).unwrap()["loader"]["disabled"], true);
    }
    #[test]
    fn directory_junction_is_rejected_without_touching_its_target() {
        let f = fixture("junction");
        let l = &f.0;
        let original = package(l, "Original");
        let root = session(l).root;
        mkdir(&root).unwrap();
        use std::os::windows::process::CommandExt;
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(root.join("Linked").to_string_lossy().replace('/', "\\"))
            .arg(original.to_string_lossy().replace('/', "\\"))
            .creation_flags(crate::game::CREATE_NO_WINDOW)
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{} {}",
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr)
        );
        assert!(list(l).is_err());
        assert!(delete(l, "Linked").is_err());
        assert!(original.join("config.json").is_file());
        fs::remove_dir(root.join("Linked")).unwrap();
    }
    #[test]
    #[ignore = "requires an unchanged original mod and an isolated evidence directory"]
    fn real_original_package_lifecycle() {
        use sha2::{Digest, Sha256};
        fn hashes(root: &Path, path: &Path, out: &mut BTreeMap<String, String>) {
            for e in fs::read_dir(path).unwrap() {
                let p = e.unwrap().path();
                not_link(&p).unwrap();
                if p.is_dir() {
                    hashes(root, &p, out);
                } else {
                    out.insert(
                        p.strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                        format!("{:x}", Sha256::digest(fs::read(&p).unwrap())),
                    );
                }
            }
        }
        let source =
            PathBuf::from(std::env::var("SSBU_TEST_MOD_ORIGINAL").expect("original package path"));
        let root = PathBuf::from(
            std::env::var("SSBU_TEST_MOD_EVIDENCE").expect("new isolated evidence path"),
        );
        assert!(!root.exists());
        mkdir(&root).unwrap();
        let l = Layout {
            game_dir: root.join("game"),
            exe: root.join("game/ssbu_native.exe"),
            root: root.clone(),
            package: true,
        };
        if let Ok(sd) = std::env::var("SSBU_TEST_MOD_SD") {
            let mut ini = Ini::load(&l.launcher_ini());
            ini.set(Some("env"), "SSBU_SD", &sd);
            ini.save(&l.launcher_ini()).unwrap();
        }
        let mut original = BTreeMap::new();
        hashes(&source, &source, &mut original);
        assert!(!original.is_empty());
        let imported = import(&l, &source).unwrap();
        let id = imported["imported"][0].as_str().unwrap();
        update(&l, id, json!({"alias":"原包验证"})).unwrap();
        for enabled in [false, true] {
            update(&l, id, json!({"enabled":enabled})).unwrap();
            let path = locate(&session(&l), id).unwrap().0;
            let mut installed = BTreeMap::new();
            hashes(&path, &path, &mut installed);
            assert_eq!(installed, original);
        }
        delete(&l, id).unwrap();
        assert!(!list(&l).unwrap()["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == id));
        let trash = fs::read_dir(session(&l).storage.join("trash"))
            .unwrap()
            .map(Result::unwrap)
            .find(|e| e.file_name().to_string_lossy().ends_with(&format!("-{id}")))
            .unwrap()
            .path();
        let mut deleted = BTreeMap::new();
        hashes(&trash, &trash, &mut deleted);
        assert_eq!(deleted, original);
        let mut after = BTreeMap::new();
        hashes(&source, &source, &mut after);
        assert_eq!(after, original);
        if std::env::var("SSBU_TEST_MOD_LEAVE_ENABLED").as_deref() == Ok("1") {
            import(&l, &source).unwrap();
        }
        fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"original":source,"files_sha256":original,"checks":["import","alias","disable","enable","soft_delete","source_unchanged","installed_and_trash_bytes_unchanged"]})).unwrap()).unwrap();
    }
    #[test]
    #[ignore = "requires an isolated prepared native game SD"]
    fn real_prepared_package_toggle() {
        let root = PathBuf::from(std::env::var("SSBU_TEST_MOD_EVIDENCE").unwrap());
        let l = Layout {
            game_dir: root.join("game"),
            exe: root.join("game/ssbu_native.exe"),
            root: root.clone(),
            package: true,
        };
        let id = std::env::var("SSBU_TEST_MOD_ID").unwrap();
        let enabled = std::env::var("SSBU_TEST_MOD_ENABLED").unwrap() == "1";
        update(&l, &id, json!({"enabled":enabled})).unwrap();
        let (path, actual) = locate(&session(&l), &id).unwrap();
        assert_eq!(enabled, actual);
        fs::write(
            root.join(format!("toggle-{}.json", enabled)),
            serde_json::to_vec_pretty(&json!({"id":id,"enabled":actual,"path":path})).unwrap(),
        )
        .unwrap();
    }
}
