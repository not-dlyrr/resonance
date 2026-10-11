use anyhow::{Context, Result};
use resonance_engine::{Engine, Profile};
use std::path::PathBuf;
fn main() {
    if let Err(e) = run() {
        eprintln!("{e:#}");
        std::process::exit(1)
    }
}
fn run() -> Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut e = Engine::default_paths();
    let mut args = vec![];
    let mut i = 0;
    while i < a.len() {
        match a[i].as_str() {
            "--root" => {
                i += 1;
                e.root = PathBuf::from(a.get(i).context("--root requires a path")?);
            }
            "--state" => {
                i += 1;
                e.state = PathBuf::from(a.get(i).context("--state requires a path")?);
            }
            _ => args.push(a[i].clone()),
        }
        i += 1;
    }
    let cmd = args.first().map(String::as_str).unwrap_or("status");
    match cmd {
        "store-refresh" => { println!("{}", serde_json::to_string_pretty(&e.store_refresh()?)?); return Ok(()); },
        "store-prepare" => { println!("{}", serde_json::to_string_pretty(&e.store_prepare(args.get(1).context("store-prepare requires ID")?)?)?); return Ok(()); },
        "store-install" => { println!("{}", serde_json::to_string_pretty(&e.store_install(args.get(1).context("store-install requires ID")?,args.get(2).context("store-install requires reviewed SHA256")?)?)?); return Ok(()); },
        _ => {}
    }
    match cmd{"status"=>println!("{}",serde_json::to_string_pretty(&e.status()?)?),"backup"=>println!("{}",e.backup()?),"restore"=>println!("{}",e.restore()?),"recover"=>println!("{}",e.recover()?),"launch"=>e.launch()?,"apply"|"repair"|"safe"=>{let p=if let Some(file)=args.get(1){serde_json::from_slice(&std::fs::read(file)?)?}else{Profile::default()};println!("{}",e.apply(&p,cmd=="safe")?);},"profiles"=>println!("{}",serde_json::to_string_pretty(&e.profiles()?)?),"save"=>{e.save_profile(&serde_json::from_slice(&std::fs::read(args.get(1).context("save requires profile.json")?)?)?)?;},"install"=>e.install(args.get(1).context("install requires package ID")?)?,"catalog"=>println!("{}",serde_json::to_string_pretty(&e.catalog()?)?),_=>anyhow::bail!("Commands: status, backup, apply [profile.json], safe [profile.json], repair [profile.json], restore, recover, launch, profiles, save profile.json, install ID, catalog. Optional --root PATH --state PATH")};
    Ok(())
}
