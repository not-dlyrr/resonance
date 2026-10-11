//! Original independent engine. Only Apps/xpui is replaced; original SPA and user data are never modified.
use anyhow::{bail, ensure, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use walkdir::WalkDir;

pub const RUNTIME: &str = include_str!("../../runtime/runtime.js");
pub const BASE_CSS: &str = include_str!("../../runtime/theme.css");
pub const CATALOG: &str = include_str!("../../packages/catalog.json");
pub mod store;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub theme: String,
    pub accent: String,
    pub font: String,
    pub density: String,
    pub radius: u32,
    pub hide_art: bool,
    pub css: String,
    pub extensions: Vec<String>,
    pub sleep_minutes: u32,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "My setup".into(),
            theme: "slate".into(),
            accent: "#78a9ff".into(),
            font: "SF Pro".into(),
            density: "comfortable".into(),
            radius: 12,
            hide_art: false,
            css: String::new(),
            extensions: vec![],
            sleep_minutes: 30,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub auto_updates: bool,
    pub update_url: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            auto_updates: true,
            update_url:
                "https://github.com/not-dlyrr/resonance/releases/latest/download/latest.json".into(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: String,
    pub description: String,
    pub source: String,
    pub compatibility: String,
    pub entry: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    pub version: String,
    pub executable_hash: String,
    pub archive_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Applied {
    identity: Identity,
    profile: Profile,
    files: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Journal {
    identity: Identity,
    previous: bool,
    operation: String,
    phase: String,
    previous_files: Option<BTreeMap<String, String>>,
    archive_primary_before: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub path: String,
    pub version: String,
    pub running: bool,
    pub supported: bool,
    pub message: String,
    pub existing: String,
    pub active: Option<Profile>,
    pub update_invalidated: bool,
    pub verified: bool,
    pub pending_recovery: bool,
    pub backup: bool,
}
#[derive(Clone)]
pub struct Engine {
    pub root: PathBuf,
    pub state: PathBuf,
}
fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
pub fn hash(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
#[cfg(windows)]
fn executable_version(path: &Path) -> Result<String> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "version")]
    extern "system" {
        fn GetFileVersionInfoSizeW(path: *const u16, handle: *mut u32) -> u32;
        fn GetFileVersionInfoW(
            path: *const u16,
            handle: u32,
            len: u32,
            data: *mut std::ffi::c_void,
        ) -> i32;
        fn VerQueryValueW(
            data: *const std::ffi::c_void,
            key: *const u16,
            out: *mut *mut std::ffi::c_void,
            len: *mut u32,
        ) -> i32;
    }
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut handle = 0;
    let n = unsafe { GetFileVersionInfoSizeW(name.as_ptr(), &mut handle) };
    ensure!(n > 0, "Spotify has no Windows version resource");
    let mut data = vec![0u8; n as usize];
    ensure!(
        unsafe { GetFileVersionInfoW(name.as_ptr(), 0, n, data.as_mut_ptr().cast()) } != 0,
        "Failed to read Spotify version"
    );
    let mut ptr = std::ptr::null_mut();
    let mut len = 0;
    let key = [92u16, 0];
    ensure!(
        unsafe { VerQueryValueW(data.as_ptr().cast(), key.as_ptr(), &mut ptr, &mut len) } != 0
            && len >= 52,
        "Invalid Spotify version resource"
    );
    let fields = unsafe { std::slice::from_raw_parts(ptr.cast::<u32>(), 13) };
    let (hi, lo) = (fields[4], fields[5]);
    Ok(format!(
        "{}.{}.{}.{}",
        hi >> 16,
        hi & 65535,
        lo >> 16,
        lo & 65535
    ))
}
fn json<T: Serialize>(p: &Path, v: &T) -> Result<()> {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = p.with_extension("json.new");
    let mut f = File::create(&tmp)?;
    f.write_all(&serde_json::to_vec_pretty(v)?)?;
    f.sync_all()?;
    replace_file(&tmp, p)
}
fn replace_file(src: &Path, dst: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let s: Vec<u16> = src.as_os_str().encode_wide().chain(Some(0)).collect();
        let d: Vec<u16> = dst.as_os_str().encode_wide().chain(Some(0)).collect();
        extern "system" {
            fn MoveFileExW(s: *const u16, d: *const u16, flags: u32) -> i32;
        }
        ensure!(
            unsafe { MoveFileExW(s.as_ptr(), d.as_ptr(), 0x1 | 0x8) } != 0,
            "Atomic replacement failed: {}",
            std::io::Error::last_os_error()
        );
    }
    #[cfg(not(windows))]
    fs::rename(src, dst)?;
    Ok(())
}
fn read_json<T: for<'a> Deserialize<'a>>(p: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
fn safe_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
fn tree_hash(p: &Path) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for e in WalkDir::new(p) {
        let e = e?;
        ensure!(!e.file_type().is_symlink(), "Symlink in managed tree");
        if e.file_type().is_file() {
            out.insert(
                e.path()
                    .strip_prefix(p)?
                    .to_string_lossy()
                    .replace('\\', "/"),
                hash(e.path())?,
            );
        }
    }
    Ok(out)
}
pub fn validate_css(css: &str) -> Result<()> {
    ensure!(css.len() <= 256 * 1024, "CSS exceeds 256 KB");
    let mut chars = css.chars().peekable();
    let mut stack = Vec::new();
    let mut quote = None;
    let mut comment = false;
    let mut line = 1;
    while let Some(c) = chars.next() {
        if c == '\n' {
            line += 1;
        }
        if comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                comment = false;
            }
            continue;
        }
        if let Some(q) = quote {
            if c == '\\' {
                chars.next();
            } else if c == q {
                quote = None;
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            comment = true;
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            continue;
        }
        match c {
            '{' | '(' | '[' => stack.push(c),
            '}' | ')' | ']' => {
                let expected = match c {
                    '}' => '{',
                    ')' => '(',
                    _ => '[',
                };
                ensure!(
                    stack.pop() == Some(expected),
                    "CSS line {line}: unmatched {c}"
                );
            }
            _ => {}
        }
    }
    ensure!(
        !comment && quote.is_none() && stack.is_empty(),
        "CSS line {line}: unfinished comment, string, or block"
    );
    lightningcss::stylesheet::StyleSheet::parse(css, Default::default())
        .map_err(|e| anyhow::anyhow!("CSS: {e}"))?;
    let lower = css.to_lowercase();
    ensure!(
        !lower.contains("@import") && !lower.contains("url("),
        "External resources (@import and url) are not supported; keep CSS self-contained"
    );
    Ok(())
}
pub fn validate_profile(p: &Profile) -> Result<()> {
    ensure!(
        !p.name.trim().is_empty() && p.name.len() <= 80,
        "Profile name must have 1–80 characters"
    );
    ensure!(
        p.accent.len() == 7
            && p.accent.starts_with('#')
            && p.accent[1..].bytes().all(|b| b.is_ascii_hexdigit()),
        "Accent must be #RRGGBB"
    );
    ensure!(
        [
            "SF Pro",
            "SF Pro Text",
            "SF Pro Display",
            "SF Mono",
            "Segoe UI",
            "Arial",
            "Georgia",
            "Consolas"
        ]
        .contains(&p.font.as_str()),
        "Unsupported font"
    );
    ensure!(
        ["compact", "comfortable", "roomy"].contains(&p.density.as_str()),
        "Unsupported density"
    );
    ensure!(
        p.radius <= 24 && p.sleep_minutes >= 1 && p.sleep_minutes <= 240,
        "Radius or timer outside allowed range"
    );
    ensure!(p.extensions.len() <= 16, "Too many extensions");
    for id in &p.extensions {
        ensure!(safe_id(id), "Invalid extension identifier");
    }
    validate_css(&p.css)
}
pub fn extract(archive: &Path, dest: &Path) -> Result<()> {
    let mut z = zip::ZipArchive::new(File::open(archive)?)?;
    ensure!(z.len() < 10000, "Archive has too many entries");
    let mut total = 0;
    for i in 0..z.len() {
        let mut f = z.by_index(i)?;
        if f.name() == "/" {
            continue;
        }
        ensure!(
            !f.name().contains('\\') && !f.name().contains(':'),
            "Unsafe archive path"
        );
        let rel = f.enclosed_name().context("Archive path traversal")?;
        ensure!(
            !rel.components()
                .any(|c| matches!(c, std::path::Component::ParentDir)),
            "Unsafe archive path"
        );
        ensure!(
            f.unix_mode().unwrap_or(0) & 0o170000 != 0o120000,
            "Archive symlink unsupported"
        );
        total += f.size();
        ensure!(
            total < 256 * 1024 * 1024 && f.size() < 64 * 1024 * 1024,
            "Archive too large"
        );
        let p = dest.join(rel);
        if f.is_dir() {
            fs::create_dir_all(p)?;
        } else {
            fs::create_dir_all(p.parent().unwrap())?;
            let mut out = File::create(p)?;
            std::io::copy(&mut f, &mut out)?;
            out.sync_all()?;
        }
    }
    Ok(())
}
impl Engine {
    pub fn default_paths() -> Self {
        Self {
            root: PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default()).join("Spotify"),
            state: PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default())
                .join("Resonance"),
        }
    }
    fn apps(&self) -> PathBuf {
        self.root.join("Apps")
    }
    fn archive(&self) -> PathBuf {
        let p = self.apps().join("xpui.spa");
        if p.exists() {
            p
        } else {
            self.apps().join(".resonance-original-xpui")
        }
    }
    fn lock(&self) -> Result<File> {
        fs::create_dir_all(&self.state)?;
        let f = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.state.join("operation.lock"))?;
        f.try_lock_exclusive()
            .context("Another operation is running")?;
        Ok(f)
    }
    pub fn identity(&self) -> Result<Identity> {
        ensure!(self.root.join("Spotify.exe").is_file(),"Spotify desktop installation not found. Install the standalone Windows desktop client.");
        let version = if self.root.join("fixture-version").exists() {
            fs::read_to_string(self.root.join("fixture-version"))?
                .trim()
                .into()
        } else {
            #[cfg(windows)]
            {
                executable_version(&self.root.join("Spotify.exe"))?
            }
            #[cfg(not(windows))]
            {
                bail!("This release supports Windows only")
            }
        };
        Ok(Identity {
            version,
            executable_hash: hash(&self.root.join("Spotify.exe"))?,
            archive_hash: hash(&self.archive())?,
        })
    }
    pub fn running(&self) -> bool {
        if self.root.join("fixture-version").exists() {
            return false;
        }
        let s =
            sysinfo::System::new_with_specifics(sysinfo::RefreshKind::nothing().with_processes(
                sysinfo::ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::Always),
            ));
        s.processes().values().any(|p| {
            p.exe().is_some_and(|e| {
                e.to_string_lossy()
                    .eq_ignore_ascii_case(&self.root.join("Spotify.exe").to_string_lossy())
            })
        })
    }
    fn supported(&self, id: &Identity) -> bool {
        id.version.starts_with("1.3.4.")
    }
    pub fn status(&self) -> Result<Status> {
        let id = self.identity()?;
        let applied: Option<Applied> = read_json(&self.state.join("applied.json")).ok();
        let updated = applied.as_ref().is_some_and(|a| a.identity != id);
        let existing = fs::read_to_string(self.apps().join("xpui/index.html")).unwrap_or_default();
        let verified = applied.as_ref().is_some_and(|a| {
            !updated && tree_hash(&self.apps().join("xpui")).ok().as_ref() == Some(&a.files)
        });
        let supported = self.supported(&id);
        Ok(Status{path:self.root.display().to_string(),version:id.version.clone(),running:self.running(),supported,message:if !supported{"This version has no verified adapter. Restore/recovery remains version-safe; applying is blocked."}else if updated{"Spotify changed. Your profiles are preserved. Reapply from the new clean archive."}else if applied.is_some()&&!verified{"Applied files differ. Use Repair or Restore before launching."}else{"Windows desktop adapter available"}.into(),existing:if existing.contains("resonance-runtime"){"Resonance"}else if existing.to_lowercase().contains("spicetify"){"Spicetify (preserved on first apply)"}else if !existing.is_empty(){"Unmanaged extracted interface"}else{"Original Spotify"}.into(),active:applied.map(|a|a.profile),update_invalidated:updated,verified,pending_recovery:self.apps().join(".resonance-journal.json").exists(),backup:self.backup_dir(&id).join("identity.json").exists()})
    }
    fn backup_dir(&self, id: &Identity) -> PathBuf {
        self.state
            .join("backups")
            .join(format!("{}-{}", id.version, &id.archive_hash[..16]))
    }
    fn clean_archive(&self) -> Result<()> {
        let mut z = zip::ZipArchive::new(File::open(self.archive())?)?;
        let mut html = String::new();
        z.by_name("index.html")?.read_to_string(&mut html)?;
        ensure!(
            html.contains("xpui-snapshot.js")
                && !html.to_lowercase().contains("spicetify")
                && !html.contains("resonance-runtime"),
            "xpui.spa is modified or unsupported. A clean archive is required."
        );
        Ok(())
    }
    fn backup_locked(&self, id: &Identity) -> Result<()> {
        self.clean_archive()?;
        let dir = self.backup_dir(id);
        if dir.join("identity.json").exists() {
            let stored: Identity = read_json(&dir.join("identity.json"))?;
            ensure!(
                &stored == id && hash(&dir.join("xpui.spa"))? == id.archive_hash,
                "Backup failed integrity check; no files changed"
            );
            return Ok(());
        }
        fs::create_dir_all(&dir)?;
        fs::copy(self.archive(), dir.join("xpui.spa"))?;
        ensure!(
            hash(&dir.join("xpui.spa"))? == id.archive_hash,
            "Backup verification failed"
        );
        json(&dir.join("identity.json"), id)
    }
    pub fn backup(&self) -> Result<String> {
        let _lock = self.lock()?;
        let id = self.identity()?;
        self.backup_locked(&id)?;
        Ok(self.backup_dir(&id).display().to_string())
    }
    pub fn recover(&self) -> Result<String> {
        let _lock = self.lock()?;
        ensure!(!self.running(), "Close Spotify before recovery");
        self.recover_locked()?;
        Ok("Interrupted operation rolled back and verified".into())
    }
    fn recover_locked(&self) -> Result<()> {
        let p = self.apps().join(".resonance-journal.json");
        if !p.exists() {
            return Ok(());
        }
        let j: Journal = read_json(&p)?;
        ensure!(j.identity==self.identity()?,"Spotify changed during interrupted operation. Manual diagnosis required; stale files were not restored.");
        let target = self.apps().join("xpui");
        let prior = self.apps().join(".resonance-prior");
        let primary = self.apps().join("xpui.spa");
        let parked = self.apps().join(".resonance-original-xpui");
        if j.phase == "committed" {
            if j.operation == "apply" {
                let a: Applied = read_json(&self.state.join("applied.json"))?;
                ensure!(
                    a.identity == j.identity && tree_hash(&target)? == a.files,
                    "Committed setup failed verification; recovery stopped"
                );
                ensure!(
                    !primary.exists() && hash(&parked)? == j.identity.archive_hash,
                    "Archive relocation verification failed"
                );
            } else {
                ensure!(!target.exists(), "Unexpected interface after restore");
                ensure!(
                    primary.exists() && hash(&primary)? == j.identity.archive_hash,
                    "Restored archive verification failed"
                );
            }
            if prior.exists() {
                fs::rename(
                    &prior,
                    self.apps().join(format!(".resonance-preserved-{}", now())),
                )?;
            }
            fs::remove_file(p)?;
            return Ok(());
        }
        if prior.exists() {
            if target.exists() {
                let quarantine = self
                    .apps()
                    .join(format!(".resonance-interrupted-{}", now()));
                fs::rename(&target, quarantine)?;
            }
            fs::rename(prior, &target)?;
        } else if !j.previous && target.exists() {
            fs::rename(
                &target,
                self.apps()
                    .join(format!(".resonance-interrupted-{}", now())),
            )?;
        }
        if let Some(files) = &j.previous_files {
            ensure!(
                tree_hash(&target)? == *files,
                "Rollback files failed hash verification"
            );
        }
        if j.archive_primary_before && !primary.exists() {
            fs::rename(&parked, &primary)?;
        } else if !j.archive_primary_before && primary.exists() {
            ensure!(!parked.exists(), "Conflicting recovery archives");
            fs::rename(&primary, &parked)?;
        }
        if let Ok(old) = fs::read(self.state.join("applied.before.json")) {
            replace_bytes(&self.state.join("applied.json"), &old)?;
        } else if self.state.join("applied.json").exists() {
            fs::remove_file(self.state.join("applied.json"))?;
        }
        fs::remove_file(p)?;
        Ok(())
    }
    pub fn apply(&self, p: &Profile, recovery: bool) -> Result<String> {
        self.apply_with_failure(p, recovery, None)
    }
    pub fn apply_with_failure(
        &self,
        p: &Profile,
        recovery: bool,
        fail: Option<&str>,
    ) -> Result<String> {
        let _lock = self.lock()?;
        ensure!(
            !self.running(),
            "Close Spotify before applying. Playback will remain stopped until you reopen it."
        );
        self.recover_locked()?;
        validate_profile(p)?;
        let id = self.identity()?;
        ensure!(
            self.supported(&id),
            "Untested Spotify version {}; applying is blocked",
            id.version
        );
        self.backup_locked(&id)?;
        let stage = self.apps().join(".resonance-stage");
        if stage.exists() {
            fs::remove_dir_all(&stage)?;
        }
        fs::create_dir(&stage)?;
        extract(&self.backup_dir(&id).join("xpui.spa"), &stage)?;
        let html = fs::read_to_string(stage.join("index.html"))?;
        ensure!(
            html.matches("</head>").count() == 1,
            "Unsupported HTML entrypoint"
        );
        let injection="<link id=\"resonance-theme\" rel=\"stylesheet\" href=\"resonance/theme.css\"><script id=\"resonance-runtime\" defer src=\"resonance/runtime.js\"></script>";
        fs::write(
            stage.join("index.html"),
            html.replace("</head>", &format!("{injection}</head>")),
        )?;
        let r = stage.join("resonance");
        fs::create_dir(&r)?;
        let mut config = p.clone();
        if recovery {
            config.extensions.clear();
        }
        let (bg, surface, text, muted) = match p.theme.as_str() {
            "slate" => ("#17191d", "#22252b", "#f4f5f7", "#acb1ba"),
            "ink" => ("#080a0f", "#121723", "#f1f4fc", "#a3acc0"),
            "paper" => ("#e4e3de", "#f2f1ed", "#202225", "#565b65"),
            _ => {
                let package = self
                    .catalog()?
                    .into_iter()
                    .find(|a| a.id == p.theme && a.kind == "theme")
                    .context("Theme is not installed")?;
                ensure!(
                    self.state.join("packages").join(&package.id).exists(),
                    "Theme is not installed"
                );
                ("#17191d", "#22252b", "#f4f5f7", "#acb1ba")
            }
        };
        let row = match p.density.as_str() {
            "compact" => 44,
            "roomy" => 72,
            _ => 56,
        };
        let vars=format!(":root, .encore-dark-theme, .encore-light-theme {{ --res-bg:{bg};--res-surface:{surface};--res-text:{text};--res-muted:{muted};--res-accent:{};--res-font:'{}';--res-radius:{}px;--res-row:{row}px; --background-base:{bg};--background-highlight:{surface};--background-elevated-base:{surface};--text-base:{text};--text-subdued:{muted};--text-bright-accent:{};--essential-bright-accent:{};--spice-main:{bg};--spice-sidebar:{surface};--spice-text:{text};--spice-subtext:{muted};--spice-button:{};--spice-player:{surface};--spice-card:{surface};}}",p.accent,p.font,p.radius,p.accent,p.accent,p.accent);
        let mut css = format!("{vars}\n{BASE_CSS}\n{}", p.css);
        if let Some((format, bytes)) = self.local_font()? {
            let file = format!("sf-local.{format}");
            fs::write(r.join(&file), bytes)?;
            let font_format = match format.as_str() {
                "ttf" => "truetype",
                "otf" => "opentype",
                _ => "woff2",
            };
            css=format!("@font-face{{font-family:'SF Pro';src:url('{file}') format('{font_format}');font-weight:100 900;font-display:swap;}}\n{css}");
        }
        if p.hide_art {
            css.push_str("\n[data-testid='cover-art-image'],[data-testid='card-image'] { visibility:hidden !important; }");
        }
        if !["slate", "ink", "paper"].contains(&p.theme.as_str()) {
            let package = self
                .catalog()?
                .into_iter()
                .find(|a| a.id == p.theme)
                .unwrap();
            css.push_str(&fs::read_to_string(
                self.state
                    .join("packages")
                    .join(package.id)
                    .join(package.entry),
            )?);
        }
        fs::write(r.join("theme.css"), css)?;
        let mut script = format!(
            "window.__RESONANCE_CONFIG__={};\n{}",
            serde_json::to_string(&config)?.replace('<', "\\u003c"),
            RUNTIME
        );
        for ext in &config.extensions {
            ensure!(
                self.installed()?.contains(ext),
                "Extension {ext} is not installed; deliberately install it first"
            );
            let code = match ext.as_str() {
                "keyboard" => include_str!("../../packages/keyboard.js").into(),
                "sleep-timer" => include_str!("../../packages/sleep-timer.js").into(),
                _ => {
                    let pack = self
                        .catalog()?
                        .into_iter()
                        .find(|a| a.id == *ext && a.kind == "extension")
                        .context("Extension manifest missing")?;
                    fs::read_to_string(self.state.join("packages").join(ext).join(pack.entry))?
                }
            };
            script.push_str(&format!(
                "\ntry{{{}\n}}catch(e){{window.Resonance.report({},e);}}",
                code,
                serde_json::to_string(ext)?
            ));
        }
        fs::write(r.join("runtime.js"), script)?;
        let files = tree_hash(&stage)?;
        ensure!(files.len() > 2, "Stage is incomplete");
        ensure!(
            self.identity()? == id,
            "Spotify updated during preparation; no files committed"
        );
        let target = self.apps().join("xpui");
        let prior = self.apps().join(".resonance-prior");
        ensure!(!prior.exists(), "Previous transaction exists");
        let primary = self.apps().join("xpui.spa");
        let parked = self.apps().join(".resonance-original-xpui");
        if primary.exists() && parked.exists() {
            fs::rename(
                &parked,
                self.apps().join(format!(".resonance-archive-{}", now())),
            )?;
        }
        let mut j = Journal {
            identity: id.clone(),
            previous: target.exists(),
            operation: "apply".into(),
            phase: "prepared".into(),
            archive_primary_before: primary.exists(),
            previous_files: if target.exists() {
                Some(tree_hash(&target)?)
            } else {
                None
            },
        };
        let old = self.state.join("applied.json");
        let before = self.state.join("applied.before.json");
        if old.exists() {
            fs::copy(&old, &before)?;
        } else if before.exists() {
            fs::remove_file(&before)?;
        }
        json(&self.apps().join(".resonance-journal.json"), &j)?;
        if primary.exists() {
            fs::rename(&primary, &parked)?;
        }
        if fail == Some("after-archive") {
            bail!("Simulated interruption after archive relocation");
        }
        if target.exists() {
            fs::rename(&target, &prior)?;
        }
        if fail == Some("after-rename") {
            bail!("Simulated interruption after rename");
        }
        j.phase = "swapping".into();
        json(&self.apps().join(".resonance-journal.json"), &j)?;
        fs::rename(&stage, &target)?;
        if fail == Some("after-commit") {
            bail!("Simulated interruption after commit");
        }
        ensure!(
            tree_hash(&target)? == files,
            "Committed files failed verification"
        );
        json(
            &old,
            &Applied {
                identity: id,
                profile: config,
                files,
            },
        )?;
        j.phase = "committed".into();
        json(&self.apps().join(".resonance-journal.json"), &j)?;
        if fail == Some("after-finalize") {
            bail!("Simulated interruption after metadata commit");
        }
        if prior.exists() {
            fs::rename(
                &prior,
                self.apps().join(format!(".resonance-preserved-{}", now())),
            )?;
        }
        fs::remove_file(self.apps().join(".resonance-journal.json"))?;
        Ok("Applied files verified. Open Spotify to use your setup.".into())
    }
    pub fn restore(&self) -> Result<String> {
        let _lock = self.lock()?;
        ensure!(!self.running(), "Close Spotify before restoring");
        self.recover_locked()?;
        let id = self.identity()?;
        let a: Applied =
            read_json(&self.state.join("applied.json")).context("No applied setup to restore")?;
        ensure!(a.identity==id,"Spotify has updated. Refusing to restore stale files; reapply from current archive instead.");
        self.backup_locked(&id)?;
        let target = self.apps().join("xpui");
        let prior = self.apps().join(".resonance-prior");
        ensure!(!prior.exists(), "Previous transaction exists");
        let primary = self.apps().join("xpui.spa");
        let parked = self.apps().join(".resonance-original-xpui");
        fs::copy(
            self.state.join("applied.json"),
            self.state.join("applied.before.json"),
        )?;
        let mut j = Journal {
            identity: id.clone(),
            previous: target.exists(),
            operation: "restore".into(),
            phase: "prepared".into(),
            archive_primary_before: primary.exists(),
            previous_files: if target.exists() {
                Some(tree_hash(&target)?)
            } else {
                None
            },
        };
        json(&self.apps().join(".resonance-journal.json"), &j)?;
        if target.exists() {
            fs::rename(&target, &prior)?;
        }
        if !primary.exists() {
            fs::rename(&parked, &primary)?;
        }
        fs::remove_file(self.state.join("applied.json"))?;
        ensure!(
            hash(&self.apps().join("xpui.spa"))? == id.archive_hash,
            "Original archive verification failed"
        );
        j.phase = "committed".into();
        json(&self.apps().join(".resonance-journal.json"), &j)?;
        if prior.exists() {
            fs::rename(
                prior,
                self.apps().join(format!(".resonance-removed-{}", now())),
            )?;
        }
        fs::remove_file(self.apps().join(".resonance-journal.json"))?;
        Ok("Original Spotify restored; archive SHA-256 verified. Previous customizations remain preserved.".into())
    }
    pub fn profiles(&self) -> Result<Vec<Profile>> {
        let dir = self.state.join("profiles");
        fs::create_dir_all(&dir)?;
        let mut out: Vec<Profile> = vec![];
        for e in fs::read_dir(dir)? {
            let p = e?.path();
            if p.extension().is_some_and(|e| e == "json") {
                out.push(read_json(&p)?);
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
    pub fn save_profile(&self, p: &Profile) -> Result<()> {
        let _lock = self.lock()?;
        validate_profile(p)?;
        let mut h = Sha256::new();
        h.update(&p.name);
        json(
            &self
                .state
                .join("profiles")
                .join(format!("{:x}.json", h.finalize())),
            p,
        )
    }
    pub fn delete_profile(&self, name: &str) -> Result<()> {
        let _lock = self.lock()?;
        let mut h = Sha256::new();
        h.update(name);
        fs::remove_file(
            self.state
                .join("profiles")
                .join(format!("{:x}.json", h.finalize())),
        )?;
        Ok(())
    }
    pub fn preferences(&self) -> Preferences {
        read_json(&self.state.join("preferences.json")).unwrap_or_default()
    }
    pub fn save_preferences(&self, p: &Preferences) -> Result<()> {
        let _lock = self.lock()?;
        ensure!(
            p.update_url.is_empty() || p.update_url.starts_with("https://"),
            "Update feed must use HTTPS"
        );
        json(&self.state.join("preferences.json"), p)
    }
    pub fn catalog(&self) -> Result<Vec<Package>> {
        let mut out: Vec<Package> = serde_json::from_str(CATALOG)?;
        let dir = self.state.join("packages");
        if dir.exists() {
            for e in fs::read_dir(dir)? {
                let p = e?.path().join("manifest.json");
                if p.exists() {
                    out.push(read_json(&p)?);
                }
            }
        }
        Ok(out)
    }
    pub fn installed(&self) -> Result<Vec<String>> {
        Ok(read_json(&self.state.join("installed.json")).unwrap_or_default())
    }
    pub fn install(&self, id: &str) -> Result<()> {
        let _lock = self.lock()?;
        ensure!(
            self.catalog()?.iter().any(|p| p.id == id),
            "Unknown package"
        );
        let mut ids = self.installed()?;
        if !ids.iter().any(|s| s == id) {
            ids.push(id.into());
        }
        json(&self.state.join("installed.json"), &ids)
    }
    pub fn uninstall(&self, id: &str) -> Result<()> {
        let _lock = self.lock()?;
        ensure!(safe_id(id), "Invalid identifier");
        let mut ids = self.installed()?;
        ids.retain(|s| s != id);
        json(&self.state.join("installed.json"), &ids)?;
        let path = self.state.join("packages").join(id);
        if path.exists() {
            fs::rename(
                path,
                self.state.join(format!("removed-package-{id}-{}", now())),
            )?;
        }
        Ok(())
    }
    pub fn import_package(&self, path: &Path) -> Result<Package> {
        let _lock = self.lock()?;
        ensure!(
            path.extension().is_some_and(|e| e == "zip"),
            "Import a .zip containing manifest.json and CSS or JS"
        );
        let stage = self.state.join(format!("import-{}", now()));
        extract(path, &stage)?;
        let p: Package = read_json(&stage.join("manifest.json"))?;
        ensure!(
            safe_id(&p.id)
                && !["keyboard", "sleep-timer", "slate", "ink", "paper"].contains(&p.id.as_str()),
            "Reserved or invalid package ID"
        );
        ensure!(
            p.version.len() < 40 && p.name.len() < 100 && p.description.len() < 2000,
            "Manifest fields too long"
        );
        ensure!(
            ["theme", "extension"].contains(&p.kind.as_str()),
            "Unsupported package kind"
        );
        ensure!(
            p.compatibility == "spotify-1.3.4-dom-v1",
            "Package requires a different adapter"
        );
        ensure!(
            !p.entry.contains('/')
                && !p.entry.contains('\\')
                && !p.entry.contains(':')
                && !p.entry.starts_with('.'),
            "Entry must be a filename"
        );
        let content = fs::read_to_string(stage.join(&p.entry))?;
        ensure!(content.len() < 512 * 1024, "Package entry exceeds 512 KB");
        if p.kind == "theme" {
            ensure!(p.entry.ends_with(".css"), "Theme must use CSS");
            validate_css(&content)?;
        } else {
            ensure!(p.entry.ends_with(".js"), "Extension must use JS");
        }
        let dest = self.state.join("packages").join(&p.id);
        fs::create_dir_all(dest.parent().unwrap())?;
        ensure!(!dest.exists(), "Package already exists; remove it first");
        fs::rename(stage, dest)?;
        let mut ids = self.installed()?;
        ids.push(p.id.clone());
        json(&self.state.join("installed.json"), &ids)?;
        Ok(p)
    }
    pub fn import_spicetify_css(&self, path: &Path) -> Result<Profile> {
        let content = fs::read_to_string(path)?;
        validate_css(&content)?;
        Ok(Profile {
            name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            css: content,
            ..Default::default()
        })
    }
    pub fn launch(&self) -> Result<()> {
        ensure!(
            !self.apps().join(".resonance-journal.json").exists(),
            "Recover interrupted operation before launching"
        );
        let mut c = Command::new(self.root.join("Spotify.exe"));
        c.current_dir(&self.root);
        c.spawn()?;
        Ok(())
    }
    pub fn import_sf_font(&self, path: &Path) -> Result<String> {
        let _lock = self.lock()?;
        let meta = fs::metadata(path)?;
        ensure!(meta.len() <= 8 * 1024 * 1024, "Font exceeds 8 MB");
        let bytes = fs::read(path)?;
        let format = if bytes.starts_with(b"wOF2") {
            "woff2"
        } else if bytes.starts_with(b"OTTO") {
            "otf"
        } else if bytes.starts_with(&[0, 1, 0, 0]) {
            "ttf"
        } else {
            bail!("Choose a WOFF2, OTF, or TTF font; this file is not supported")
        };
        let dir = self.state.join("fonts");
        fs::create_dir_all(&dir)?;
        let tmp = dir.join("sf-font.new");
        fs::write(&tmp, bytes)?;
        replace_file(&tmp, &dir.join("sf-font"))?;
        json(&dir.join("format.json"), &format)?;
        Ok(
            "Local SF Pro font imported. The manager uses it now; apply to use it inside Spotify."
                .into(),
        )
    }
    fn local_font(&self) -> Result<Option<(String, Vec<u8>)>> {
        let p = self.state.join("fonts/sf-font");
        if !p.exists() {
            return Ok(None);
        }
        let format: String = read_json(&self.state.join("fonts/format.json"))?;
        ensure!(
            ["woff2", "otf", "ttf"].contains(&format.as_str()),
            "Unsupported font format"
        );
        Ok(Some((format, fs::read(p)?)))
    }
    pub fn font_css(&self) -> Result<String> {
        use base64::Engine as _;
        if let Some((format, bytes)) = self.local_font()? {
            let (mime, fmt) = match format.as_str() {
                "ttf" => ("font/ttf", "truetype"),
                "otf" => ("font/otf", "opentype"),
                _ => ("font/woff2", "woff2"),
            };
            Ok(format!("@font-face{{font-family:'SF Pro';src:url(data:{mime};base64,{}) format('{fmt}');font-weight:100 900;font-display:swap;}}",base64::engine::general_purpose::STANDARD.encode(bytes)))
        } else {
            Ok(String::new())
        }
    }
}
fn replace_bytes(p: &Path, b: &[u8]) -> Result<()> {
    let tmp = p.with_extension("json.new");
    fs::write(&tmp, b)?;
    replace_file(&tmp, p)
}
