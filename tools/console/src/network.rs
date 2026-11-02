//! Shared connection choice for Console and F1: online_mode in graphics.ini.
use crate::{ini::Ini, paths::Layout};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::io::Read;

pub fn value(path: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.trim_start_matches('\u{feff}').lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        k.trim().eq_ignore_ascii_case(key).then(|| v.split([';', '#']).next().unwrap_or("").trim().to_string())
    })
}
fn off(v: &str) -> bool { ["", "0", "off", "false", "no"].contains(&v.to_ascii_lowercase().as_str()) }
pub fn saved_mode(path: &Path) -> String {
    if let Some(v) = value(path, "online_mode") { return v.to_ascii_lowercase(); }
    match value(path, "online_server") {
        Some(v) if off(&v) => "off".into(),
        Some(_) => "lobby".into(),
        None => "yuzu".into(),
    }
}
/// The bridge may admit a server room before the native loader knows the game build.
pub fn game_version(l: &Layout) -> String {
    let state=std::fs::read(l.root.join("state.json")).ok().and_then(|b|serde_json::from_slice::<Value>(&b).ok());
    if state.as_ref().and_then(|v|v["game_version"].as_str())==Some("13.0.5") {"13.0.5".into()}else{"13.0.4".into()}
}
pub fn get(l: &Layout) -> Value {
    let path = l.graphics_ini();
    json!({"mode": saved_mode(&path), "server": value(&path,"online_server").unwrap_or_else(|| "opensmash-lobby.duckdns.org:39500".into()),
        "join": value(&path,"online_join").unwrap_or_default(), "rollback": value(&path,"rollback").is_some_and(|s| ["on","1","true","yes"].contains(&s.to_ascii_lowercase().as_str()))})
}
pub fn set(l: &Layout, values: &Map<String, Value>) -> Result<Value, String> {
    let mut entries = Vec::new();
    for (key, raw) in values {
        let s = raw.as_str().ok_or("连接设置必须是文字")?.trim();
        if s.chars().any(|c| c.is_control()) || s.contains([';', '#']) { return Err("连接设置不能含换行或注释字符".into()); }
        let valid = match key.as_str() {
            "online_mode" => ["off","yuzu","lan","host","join","lobby"].contains(&s),
            "online_server" => !s.is_empty() && s.len() < 128,
            "online_join" => s.len() < 64,
            "rollback" => ["on","off"].contains(&s),
            _ => false,
        };
        if !valid { return Err(format!("无效的连接设置：{}", key)); }
        entries.push((key.as_str(),s));
    }
    let mode = entries.iter().find(|(k,_)| *k=="online_mode").map(|(_,s)|s.to_string()).unwrap_or_else(||saved_mode(&l.graphics_ini()));
    if mode=="yuzu" && entries.iter().any(|(k,v)| *k=="rollback" && *v=="on") {
        return Err("Yuzu 联机暂不支持回滚；原有上游回滚设置会保留".into());
    }
    let mut ini = Ini::load(&l.graphics_ini());
    for (key, s) in entries { ini.set(None,key,s); }
    ini.save(&l.graphics_ini()).map_err(|e|format!("连接设置未保存：{}",e))?;
    Ok(get(l))
}
fn settings_path(l: &Layout, env: &Map<String, Value>) -> PathBuf {
    let p = if let Some(v)=env.get("SSBU_GRAPHICS_CONFIG").and_then(Value::as_str) { PathBuf::from(v) }
        else if let Some(v)=env.get("SSBU_CONFIG").and_then(Value::as_str) { Path::new(v).parent().unwrap_or(Path::new("")).join("graphics.ini") }
        else { return l.graphics_ini(); };
    if p.is_absolute() { p } else { l.game_dir.join(p) }
}
fn available(l: &Layout) -> Result<bool,String> {
    let path=l.exe.with_extension("build.json");
    let bytes=match std::fs::read(path) { Ok(b)=>b,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(false),Err(e)=>return Err(e.to_string()) };
    let f:Value=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
    if f["yuzu_network"]!=true { return Ok(false); }
    let mut file=std::fs::File::open(&l.exe).map_err(|e|e.to_string())?;
    let mut hash=Sha256::new();let mut buf=[0u8;65536];
    loop { let n=file.read(&mut buf).map_err(|e|e.to_string())?;if n==0 {break;}hash.update(&buf[..n]); }
    if f["network_abi"]!=1 || f["exe_sha256"]!=format!("{:x}",hash.finalize()) { return Err("程序的 Yuzu 构建记录不匹配，请重新构建或安装完整程序包".into()); }
    Ok(true)
}
/// Return the effective connection after all explicit per-run overrides.
pub fn configure(l: &Layout, env: &mut Map<String, Value>, smoke: bool) -> Result<String,String> {
    let old=std::mem::take(env);
    for (k,v) in old { env.insert(if k.to_ascii_uppercase().starts_with("SSBU_"){k.to_ascii_uppercase()}else{k},v); }
    // Resolve once so runtime and launch policy read the same explicit configuration.
    env.entry("SSBU_CONFIG").or_insert(json!(l.controllers_ini().to_string_lossy()));
    let path=settings_path(l,env);
    env.entry("SSBU_GRAPHICS_CONFIG").or_insert(json!(path.to_string_lossy()));
    if smoke { env.insert("SSBU_ONLINE".into(),json!("off"));env.insert("SSBU_RB_NET".into(),json!("0"));return Ok("off".into()); }
    let enabled=available(l)?;
    let choice=env.get("SSBU_ONLINE").and_then(Value::as_str).map(str::to_ascii_lowercase)
        .unwrap_or_else(|| if !enabled && value(&path,"online_mode").is_none() && value(&path,"online_server").is_none(){"lobby".into()}else{saved_mode(&path)});
    let mode=if off(&choice){"off"}else if choice=="yuzu" {
        if !enabled{return Err("此程序没有可核验的 Yuzu 联机扩展，请重新构建或安装完整程序包".into());}"yuzu"
    }else if ["lan","host","join","lobby","1","on","auto"].contains(&choice.as_str()) || choice.contains(':'){"upstream"}else{"off"};
    if mode!="upstream" {
        if !env.get("SSBU_RB_NET").and_then(Value::as_str).unwrap_or("").starts_with("loop") { env.insert("SSBU_RB_NET".into(),json!("0")); }
    }else if value(&path,"rollback").is_some_and(|s|["on","1","yes","true"].contains(&s.to_ascii_lowercase().as_str())) {
        env.entry("SSBU_RB_NET").or_insert(json!("net"));
    }
    Ok(mode.into())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(tag:&str)->Layout {
        let root=std::env::temp_dir().join(format!("ssbu_connection_{}_{}",tag,std::process::id()));
        std::fs::create_dir_all(root.join("game")).unwrap();std::fs::create_dir_all(root.join("config")).unwrap();
        let exe=root.join("game/ssbu_native.exe");std::fs::write(&exe,b"network-host").unwrap();
        let digest=format!("{:x}",Sha256::digest(b"network-host"));
        std::fs::write(exe.with_extension("build.json"),json!({"yuzu_network":true,"network_abi":1,"exe_sha256":digest}).to_string()).unwrap();
        Layout{game_dir:root.join("game"),exe,root,package:true}
    }
    #[test]
    fn room_admission_version_matches_selected_game_files() {
        let l=fixture("version");
        assert_eq!(game_version(&l),"13.0.4");
        std::fs::write(l.root.join("state.json"),r#"{"game_version":"13.0.5"}"#).unwrap();
        assert_eq!(game_version(&l),"13.0.5");
        std::fs::write(l.root.join("state.json"),b"invalid").unwrap();
        assert_eq!(game_version(&l),"13.0.4");
        std::fs::remove_dir_all(l.root).unwrap();
    }
    #[test]
    fn shared_settings_preserve_unknown_keys_and_upstream_rollback() {
        let l=fixture("settings");
        std::fs::write(l.graphics_ini(),"; user settings\nonline_mode = lobby\nrollback = on\ngpu = keep\nonline_direct = off\n").unwrap();
        set(&l,json!({"online_mode":"yuzu"}).as_object().unwrap()).unwrap();
        let got=get(&l);assert_eq!(got["mode"],"yuzu");assert_eq!(got["rollback"],true);
        let text=std::fs::read_to_string(l.graphics_ini()).unwrap();assert!(text.contains("gpu = keep")&&text.contains("online_direct = off")&&text.starts_with("; user settings"));
        assert!(set(&l,json!({"rollback":"on"}).as_object().unwrap()).is_err());
        assert_eq!(std::fs::read_to_string(l.graphics_ini()).unwrap(),text);
        set(&l,json!({"online_mode":"lobby"}).as_object().unwrap()).unwrap();assert_eq!(get(&l)["rollback"],true);
        std::fs::remove_dir_all(l.root).unwrap();
    }
    #[test]
    fn validation_is_atomic_and_rejects_injection() {
        let l=fixture("validation");std::fs::write(l.graphics_ini(),"online_mode=yuzu\n").unwrap();
        for v in [json!({"online_mode":"bad"}),json!({"online_mode":"off","online_server":"server\nrollback=on"}),json!({"renderer":"opengl"}),json!({"online_mode":"yuzu","rollback":"on"})] {
            assert!(set(&l,v.as_object().unwrap()).is_err());assert_eq!(std::fs::read_to_string(l.graphics_ini()).unwrap(),"online_mode=yuzu\n");
        }
        std::fs::remove_dir_all(l.root).unwrap();
    }
    #[test]
    fn effective_mode_gate_and_rollback_are_not_the_launch_tab() {
        let l=fixture("modes");std::fs::write(l.graphics_ini(),"online_mode=yuzu\nrollback=on\n").unwrap();
        let mut env=json!({"SSBU_RB_NET":"net"}).as_object().unwrap().clone();assert_eq!(configure(&l,&mut env,false).unwrap(),"yuzu");assert_eq!(env["SSBU_RB_NET"],"0");
        env=json!({"ssbu_online":"lobby"}).as_object().unwrap().clone();assert_eq!(configure(&l,&mut env,false).unwrap(),"upstream");assert_eq!(env["SSBU_RB_NET"],"net");
        env.insert("SSBU_RB_NET".into(),json!("0"));configure(&l,&mut env,false).unwrap();assert_eq!(env["SSBU_RB_NET"],"0");
        assert_eq!(configure(&l,&mut env,true).unwrap(),"off");assert_eq!(env["SSBU_ONLINE"],"off");assert_eq!(env["SSBU_RB_NET"],"0");
        std::fs::remove_dir_all(l.root).unwrap();
    }
    #[test]
    fn settings_paths_match_runtime_and_bom_is_read() {
        let l=fixture("paths");std::fs::write(l.game_dir.join("graphics.ini"),"\u{feff} online_mode=off # user\n").unwrap();
        let mut env=json!({"SSBU_CONFIG":"controllers.ini"}).as_object().unwrap().clone();assert_eq!(configure(&l,&mut env,false).unwrap(),"off");
        assert_eq!(PathBuf::from(env["SSBU_GRAPHICS_CONFIG"].as_str().unwrap()),l.game_dir.join("graphics.ini"));
        std::fs::write(l.graphics_ini(),"online_mode=lobby\n").unwrap();env.insert("SSBU_GRAPHICS_CONFIG".into(),json!(l.graphics_ini()));assert_eq!(configure(&l,&mut env,false).unwrap(),"upstream");
        std::fs::remove_dir_all(l.root).unwrap();
    }
    #[test]
    fn upstream_or_changed_binary_cannot_claim_yuzu() {
        let l=fixture("binary");std::fs::write(l.graphics_ini(),"online_mode=yuzu\n").unwrap();
        std::fs::write(&l.exe,b"replaced").unwrap();assert!(configure(&l,&mut Map::new(),false).is_err());
        std::fs::remove_file(l.exe.with_extension("build.json")).unwrap();assert!(configure(&l,&mut Map::new(),false).is_err());
        std::fs::write(l.graphics_ini(),"online_mode=off\n").unwrap();assert_eq!(configure(&l,&mut Map::new(),false).unwrap(),"off");
        std::fs::remove_dir_all(l.root).unwrap();
    }
}
