//! Marketplace discovery uses the same GitHub topics/manifests as the Spicetify store.
//! No upstream Marketplace application code is included.
use super::*;
use serde_json::Value;
use std::time::Duration;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoreEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub version: String,
    pub source: String,
    pub download_url: String,
    pub inline_css: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoreCache {
    pub fetched_at: u128,
    pub entries: Vec<StoreEntry>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prepared {
    pub package: Package,
    pub sha256: String,
    pub compatible: bool,
    pub reasons: Vec<String>,
    pub code: String,
}
fn client() -> Result<reqwest::blocking::Client> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    Ok(reqwest::blocking::Client::builder()
        .user_agent("Resonance/0.1 Marketplace")
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}
fn download(c: &reqwest::blocking::Client, url: &str, limit: u64) -> Result<String> {
    let parsed = reqwest::Url::parse(url)?;
    ensure!(
        parsed.scheme() == "https"
            && ["api.github.com", "raw.githubusercontent.com"]
                .contains(&parsed.host_str().unwrap_or_default()),
        "Only public GitHub HTTPS package sources are accepted"
    );
    let response = c.get(url).send()?.error_for_status()?;
    ensure!(
        response.content_length().unwrap_or(0) <= limit,
        "Remote file exceeds download limit"
    );
    let mut bytes = Vec::new();
    response.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "Remote file exceeds download limit"
    );
    Ok(String::from_utf8(bytes)?)
}
fn ident(source: &str) -> String {
    format!("store-{:x}", Sha256::digest(source.as_bytes()))[..38].into()
}
fn raw_url(repo: &str, branch: &str, file: &str) -> Result<String> {
    if file.starts_with("https://") {
        let u = reqwest::Url::parse(file)?;
        ensure!(
            u.host_str() == Some("raw.githubusercontent.com"),
            "External package URL is unsupported"
        );
        return Ok(file.into());
    }
    ensure!(
        !file.starts_with('/')
            && !file.contains("..")
            && !file.contains('\\')
            && !file.contains(':'),
        "Unsafe manifest path"
    );
    ensure!(!branch.contains(".."), "Unsafe branch");
    Ok(format!(
        "https://raw.githubusercontent.com/{repo}/{branch}/{file}"
    ))
}
pub fn convert_css(code: &str) -> String {
    let mut out = code.to_owned();
    for (old, new) in [
        (
            ".Root__now-playing-bar footer",
            "[data-testid=\"now-playing-bar\"]",
        ),
        (
            ".Root__now-playing-bar",
            "[data-testid=\"now-playing-bar\"]",
        ),
        (
            ".main-nowPlayingBar-container",
            "[data-testid=\"now-playing-bar\"]",
        ),
        (".main-navBar-navBar", "nav[aria-label=\"Main\"]"),
        (".main-view-container", "main"),
        (
            ".main-trackList-trackListRow",
            "[role=\"row\"][aria-rowindex]:has([role=\"gridcell\"])",
        ),
        (".cover-art-image", "[data-testid=\"cover-art-image\"]"),
        (".main-cardImage-image", "[data-testid=\"card-image\"]"),
    ] {
        out = out.replace(old, new);
    }
    out
}
pub fn assess(kind: &str, code: &str) -> Vec<String> {
    let mut reasons = vec![];
    if kind == "theme" {
        if let Err(e) = validate_css(code) {
            reasons.push(e.to_string());
        }
        let re = regex::Regex::new(r"\.(?:main|x|view|artist)-[A-Za-z0-9_-]+").unwrap();
        let unresolved: Vec<_> = re.find_iter(code).map(|m| m.as_str()).take(4).collect();
        if !unresolved.is_empty() {
            reasons.push(format!(
                "Unmapped Spicetify selectors: {}",
                unresolved.join(", ")
            ));
        }
    } else {
        let re = regex::Regex::new(r"Spicetify\s*\.\s*([A-Za-z_$][\w$]*)").unwrap();
        let mut unsupported = vec![];
        for cap in re.captures_iter(code) {
            if !["showNotification", "LocalStorage"].contains(&&cap[1])
                && !unsupported.contains(&cap[1].to_owned())
            {
                unsupported.push(cap[1].to_owned());
            }
        }
        if !unsupported.is_empty() {
            reasons.push(format!(
                "Unsupported Spicetify APIs: {}",
                unsupported.join(", ")
            ));
        }
        if code.contains("Spicetify[")
            || code.contains("Spicetify?.")
            || code.contains("import ")
            || code.contains("require(")
        {
            reasons
                .push("Dynamic API access or external module dependencies need conversion".into());
        }
    }
    reasons
}
impl Engine {
    pub fn store_cached(&self) -> StoreCache {
        read_json(&self.state.join("store-cache.json")).unwrap_or(StoreCache {
            fetched_at: 0,
            entries: vec![],
            warnings: vec![],
        })
    }
    pub fn store_refresh(&self) -> Result<StoreCache> {
        let _lock = self.lock()?;
        let c = client()?;
        let mut entries = vec![];
        let mut warnings = vec![];
        for (topic, kind) in [
            ("spicetify-themes", "theme"),
            ("spicetify-extensions", "extension"),
        ] {
            let url=format!("https://api.github.com/search/repositories?q=topic%3A{topic}&sort=stars&order=desc&per_page=15");
            let search: Value =
                match download(&c, &url, 1024 * 1024).and_then(|s| Ok(serde_json::from_str(&s)?)) {
                    Ok(v) => v,
                    Err(e) => {
                        warnings.push(format!("{topic}: {e}"));
                        continue;
                    }
                };
            let repos = search["items"].as_array().cloned().unwrap_or_default();
            for chunk in repos.chunks(4) {
                let handles:Vec<_>=chunk.iter().cloned().map(|repo|{let cl=c.clone();let kind=kind.to_owned();std::thread::spawn(move||->Result<Vec<StoreEntry>>{let name=repo["full_name"].as_str().context("Repository name missing")?;let branch=repo["default_branch"].as_str().unwrap_or("main");let raw=download(&cl,&format!("https://raw.githubusercontent.com/{name}/{branch}/manifest.json"),256*1024)?;let value:Value=serde_json::from_str(&raw)?;let manifests=if let Some(a)=value.as_array(){a.clone()}else{vec![value]};let mut found=vec![];for m in manifests{let Some(title)=m["name"].as_str()else{continue};let entry=m[if kind=="theme"{"usercss"}else{"main"}].as_str();let Some(entry)=entry else{continue};let url=match raw_url(name,m["branch"].as_str().unwrap_or(branch),entry){Ok(u)=>u,Err(_)=>continue};found.push(StoreEntry{id:ident(&url),name:title.chars().take(100).collect(),description:m["description"].as_str().unwrap_or_default().chars().take(2000).collect(),kind:kind.clone(),version:m["version"].as_str().unwrap_or("manifest / rolling").into(),source:format!("https://github.com/{name}"),download_url:url,inline_css:None});}Ok(found)})}).collect();
                for h in handles {
                    if let Ok(Ok(mut found)) = h.join() {
                        entries.append(&mut found);
                    }
                }
            }
        }
        let snippet_url =
            "https://raw.githubusercontent.com/spicetify/marketplace/main/resources/snippets.json";
        match download(&c, snippet_url, 1024 * 1024)
            .and_then(|s| Ok(serde_json::from_str::<Value>(&s)?))
        {
            Ok(v) => {
                for s in v.as_array().into_iter().flatten() {
                    if let (Some(title), Some(code)) = (s["title"].as_str(), s["code"].as_str()) {
                        entries.push(StoreEntry {
                            id: ident(&format!("{snippet_url}#{title}")),
                            name: title.chars().take(100).collect(),
                            description: s["description"]
                                .as_str()
                                .unwrap_or_default()
                                .chars()
                                .take(2000)
                                .collect(),
                            kind: "snippet".into(),
                            version: "marketplace / rolling".into(),
                            source: "https://github.com/spicetify/marketplace".into(),
                            download_url: snippet_url.into(),
                            inline_css: Some(code.into()),
                        });
                    }
                }
            }
            Err(e) => warnings.push(format!("Snippets: {e}")),
        };
        ensure!(
            !entries.is_empty(),
            "Store unavailable; cached catalog remains available. {}",
            warnings.join("; ")
        );
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries.dedup_by(|a, b| a.id == b.id);
        let cache = StoreCache {
            fetched_at: now(),
            entries,
            warnings,
        };
        json(&self.state.join("store-cache.json"), &cache)?;
        Ok(cache)
    }
    pub fn store_prepare(&self, id: &str) -> Result<Prepared> {
        let _lock = self.lock()?;
        ensure!(safe_id(id), "Invalid store ID");
        let e = self
            .store_cached()
            .entries
            .into_iter()
            .find(|p| p.id == id)
            .context("Package is not in cached store; refresh first")?;
        let mut code = match e.inline_css {
            Some(s) => s,
            None => download(&client()?, &e.download_url, 512 * 1024)?,
        };
        let kind = if e.kind == "extension" {
            "extension"
        } else {
            "theme"
        };
        if kind == "theme" {
            code = convert_css(&code);
        }
        let reasons = assess(kind, &code);
        let mut h = Sha256::new();
        h.update(code.as_bytes());
        let sha256 = format!("{:x}", h.finalize());
        let prepared = Prepared {
            package: Package {
                id: e.id,
                name: e.name,
                version: e.version,
                kind: kind.into(),
                description: e.description,
                source: format!("{} · {}", e.source, e.download_url),
                compatibility: "spotify-1.3.4-dom-v1".into(),
                entry: if kind == "theme" {
                    "theme.css"
                } else {
                    "extension.js"
                }
                .into(),
            },
            sha256,
            compatible: reasons.is_empty(),
            reasons,
            code,
        };
        json(
            &self.state.join("prepared").join(format!("{id}.json")),
            &prepared,
        )?;
        Ok(prepared)
    }
    pub fn store_install(&self, id: &str, sha256: &str) -> Result<Package> {
        let _lock = self.lock()?;
        ensure!(safe_id(id), "Invalid store ID");
        let p: Prepared = read_json(&self.state.join("prepared").join(format!("{id}.json")))?;
        ensure!(
            p.sha256 == sha256 && format!("{:x}", Sha256::digest(p.code.as_bytes())) == sha256,
            "Prepared download changed; review it again"
        );
        ensure!(
            p.compatible,
            "Package requires conversion: {}",
            p.reasons.join("; ")
        );
        let dest = self.state.join("packages").join(id);
        ensure!(!dest.exists(), "Package already installed");
        fs::create_dir_all(&dest)?;
        fs::write(dest.join(&p.package.entry), &p.code)?;
        json(&dest.join("manifest.json"), &p.package)?;
        let mut installed = self.installed()?;
        installed.push(id.into());
        json(&self.state.join("installed.json"), &installed)?;
        Ok(p.package)
    }
}
