//! Icon & style resolver for Archflow diagrams.
//!
//! Resolve chain per provider:
//!
//! ```text
//! use aws                        → local cache → official registry
//! use aws from ./my-icons        → local path only (no fallback)
//! use aws from github:org/repo   → local cache → HTTP fetch → cache result
//! use aws from https://cdn.com   → local cache → HTTP fetch → cache result
//! ```
//!
//! Principles:
//! - `from` present → that source only (no fallback to other sources)
//! - `from` absent → local cache → official registry
//! - HTTP results are cached to ~/.archflow/cache/

use crate::model::DiagramIR;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Default official icon registry on GitHub.
const DEFAULT_REGISTRY: &str = "https://raw.githubusercontent.com/soulee-dev/archflow-icons/main";

// ─── Trait ───

/// Trait for fetching icon data and manifests.
pub trait IconSource: std::fmt::Debug {
    fn fetch_svg(&self, path: &str) -> Option<String>;
    fn fetch_manifest(&self, path: &str) -> Option<ProviderManifest>;
}

// ─── Manifest types ───

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProviderManifest {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub node_render_mode: Option<String>,
    #[serde(default)]
    pub cluster_styles: HashMap<String, ClusterStyleDef>,
    #[serde(default)]
    pub nodes: Vec<String>,
    #[serde(default)]
    pub clusters: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ClusterStyleDef {
    pub stroke: Option<String>,
    pub fill: Option<String>,
    pub stroke_dasharray: Option<String>,
    pub corner_radius: Option<f64>,
}

// ─── Source implementations ───

/// Reads icons from a local directory.
#[derive(Debug)]
pub struct LocalSource {
    base: PathBuf,
}

impl LocalSource {
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    /// ~/.archflow/icons/
    pub fn default_cache() -> Option<Self> {
        let path = home_dir()?.join(".archflow").join("icons");
        if path.is_dir() {
            Some(Self::new(path))
        } else {
            None
        }
    }

    /// ~/.archflow/cache/ (for HTTP fetch cache)
    pub fn http_cache() -> Option<Self> {
        let path = home_dir()?.join(".archflow").join("cache");
        Some(Self::new(path))
    }
}

impl IconSource for LocalSource {
    fn fetch_svg(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.base.join(path)).ok()
    }

    fn fetch_manifest(&self, path: &str) -> Option<ProviderManifest> {
        let content = std::fs::read_to_string(self.base.join(path)).ok()?;
        serde_json::from_str(&content).ok()
    }
}

/// Fetches icons via HTTP (blocking). Results are cached to disk.
#[derive(Debug)]
pub struct HttpSource {
    base_url: String,
    cache_dir: Option<PathBuf>,
}

impl HttpSource {
    pub fn new(base_url: String, cache_dir: Option<PathBuf>) -> Self {
        Self {
            base_url,
            cache_dir,
        }
    }

    /// Create from the official registry with default cache dir.
    pub fn official_registry() -> Self {
        Self {
            base_url: DEFAULT_REGISTRY.to_string(),
            cache_dir: home_dir().map(|h| h.join(".archflow").join("cache")),
        }
    }

    /// Create from a `github:org/repo` source string.
    pub fn from_github(org_repo: &str) -> Self {
        Self {
            base_url: format!("https://raw.githubusercontent.com/{}/main", org_repo),
            cache_dir: home_dir().map(|h| h.join(".archflow").join("cache")),
        }
    }

    /// Create from an HTTP/HTTPS URL.
    pub fn from_url(url: &str) -> Self {
        Self {
            base_url: url.trim_end_matches('/').to_string(),
            cache_dir: home_dir().map(|h| h.join(".archflow").join("cache")),
        }
    }

    fn read_cache(&self, path: &str) -> Option<String> {
        let cache_dir = self.cache_dir.as_ref()?;
        let cache_path = cache_dir.join(path);
        std::fs::read_to_string(cache_path).ok()
    }

    fn write_cache(&self, path: &str, content: &str) {
        if let Some(cache_dir) = &self.cache_dir {
            let cache_path = cache_dir.join(path);
            if let Some(parent) = cache_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(cache_path, content);
        }
    }

    fn http_get(&self, url: &str) -> Option<String> {
        // Minimal HTTP GET using std only (no external crate).
        // For robustness, users should install icons locally.
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Use a simple TCP connection for HTTP GET
            use std::io::{Read, Write};
            use std::net::TcpStream;

            let url = url.to_string();
            let is_https = url.starts_with("https://");
            let without_scheme = url
                .strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))?;
            let (host_port, path) = without_scheme.split_once('/')?;
            let host = host_port.split(':').next()?;
            let _port = if is_https { 443 } else { 80 };

            // For HTTPS we can't do raw TCP without TLS.
            // Fall back to command-line curl if available.
            if is_https {
                let output = std::process::Command::new("curl")
                    .args(["-sf", "--max-time", "10", &url])
                    .output()
                    .ok()?;
                if output.status.success() {
                    return Some(String::from_utf8_lossy(&output.stdout).to_string());
                }
                return None;
            }

            // Plain HTTP
            let addr = format!("{}:{}", host_port, 80);
            let mut stream = TcpStream::connect(&addr).ok()?;
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .ok()?;
            let request = format!(
                "GET /{} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: archflow\r\n\r\n",
                path, host
            );
            stream.write_all(request.as_bytes()).ok()?;
            let mut response = String::new();
            stream.read_to_string(&mut response).ok()?;

            // Parse HTTP response
            let body_start = response.find("\r\n\r\n")?;
            let headers = &response[..body_start];
            if !headers.contains("200") {
                return None;
            }
            Some(response[body_start + 4..].to_string())
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = url;
            None // WASM uses JS fetch, not this
        }
    }
}

impl IconSource for HttpSource {
    fn fetch_svg(&self, path: &str) -> Option<String> {
        // Check cache first
        if let Some(cached) = self.read_cache(path) {
            return Some(cached);
        }
        // HTTP fetch
        let url = format!("{}/{}", self.base_url, path);
        let content = self.http_get(&url)?;
        self.write_cache(path, &content);
        Some(content)
    }

    fn fetch_manifest(&self, path: &str) -> Option<ProviderManifest> {
        // Check cache first
        if let Some(cached) = self.read_cache(path) {
            return serde_json::from_str(&cached).ok();
        }
        // HTTP fetch
        let url = format!("{}/{}", self.base_url, path);
        let content = self.http_get(&url)?;
        let manifest = serde_json::from_str(&content).ok()?;
        self.write_cache(path, &content);
        Some(manifest)
    }
}

// ─── Resolve logic ───

/// Build the source chain for a provider based on its `from` declaration.
///
/// - `from` = None → local cache → official registry
/// - `from` = Some("./path") → local path only
/// - `from` = Some("github:org/repo") → local cache → HTTP (github raw)
/// - `from` = Some("https://...") → local cache → HTTP
fn build_sources_for(from: &Option<String>) -> Vec<Box<dyn IconSource>> {
    let mut sources: Vec<Box<dyn IconSource>> = Vec::new();

    match from {
        None => {
            // Default: local cache → official registry
            if let Some(local) = LocalSource::default_cache() {
                sources.push(Box::new(local));
            }
            sources.push(Box::new(HttpSource::official_registry()));
        }
        Some(source)
            if source.starts_with("./") || source.starts_with("../") || source.starts_with('/') =>
        {
            // Local path only, no fallback
            let path = PathBuf::from(source);
            if path.is_dir() {
                sources.push(Box::new(LocalSource::new(path)));
            }
        }
        Some(source) if source.starts_with("github:") => {
            let org_repo = &source[7..];
            // Cache first, then GitHub
            if let Some(cache) = LocalSource::http_cache() {
                sources.push(Box::new(cache));
            }
            sources.push(Box::new(HttpSource::from_github(org_repo)));
        }
        Some(source) if source.starts_with("https://") || source.starts_with("http://") => {
            // Cache first, then URL
            if let Some(cache) = LocalSource::http_cache() {
                sources.push(Box::new(cache));
            }
            sources.push(Box::new(HttpSource::from_url(source)));
        }
        Some(_) => {
            // Unknown source format, try as local path
            if let Some(local) = LocalSource::default_cache() {
                sources.push(Box::new(local));
            }
        }
    }

    sources
}

/// Resolve all provider icons and styles in the IR.
pub fn resolve_ir(ir: &mut DiagramIR, extra_sources: &[&dyn IconSource]) {
    let declared: HashMap<String, Option<String>> = ir.metadata.provider_sources.clone();
    if declared.is_empty() {
        return;
    }

    // Build per-provider source chains
    let provider_sources: HashMap<String, Vec<Box<dyn IconSource>>> = declared
        .iter()
        .map(|(provider, from)| (provider.clone(), build_sources_for(from)))
        .collect();

    // Helper: get sources for a provider (per-provider chain + extra)
    let get_sources = |provider: &str| -> Vec<&dyn IconSource> {
        let mut s: Vec<&dyn IconSource> = Vec::new();
        if let Some(chain) = provider_sources.get(provider) {
            for src in chain {
                s.push(src.as_ref());
            }
        }
        for src in extra_sources {
            s.push(*src);
        }
        s
    };

    // Load manifests
    let mut manifests: HashMap<String, ProviderManifest> = HashMap::new();
    for provider in declared.keys() {
        let path = format!("{}/manifest.json", provider);
        for source in get_sources(provider) {
            if let Some(mf) = source.fetch_manifest(&path) {
                manifests.insert(provider.clone(), mf);
                break;
            }
        }
    }

    // Apply cluster_styles from manifests
    for cluster in &mut ir.clusters {
        let provider = match &cluster.provider {
            Some(p) if declared.contains_key(p) => p.clone(),
            _ => continue,
        };
        let cluster_type = match &cluster.cluster_type {
            Some(t) => t.clone(),
            None => continue,
        };
        if cluster.style.is_some() {
            continue;
        }
        if let Some(mf) = manifests.get(&provider) {
            if let Some(preset) = mf.cluster_styles.get(&cluster_type) {
                cluster.style = Some(crate::model::Style {
                    fill: preset.fill.clone(),
                    stroke: preset.stroke.clone(),
                    stroke_dasharray: preset.stroke_dasharray.clone(),
                    corner_radius: preset.corner_radius,
                    ..Default::default()
                });
            }
        }
    }

    // Apply node_render_modes
    for (provider, mf) in &manifests {
        if let Some(mode) = &mf.node_render_mode {
            ir.metadata
                .node_render_modes
                .insert(provider.clone(), mode.clone());
        }
    }

    // Resolve node icons
    for node in &mut ir.nodes {
        if node.icon_svg.is_some() {
            continue;
        }
        let provider = match &node.provider {
            Some(p) if declared.contains_key(p) => p.clone(),
            _ => continue,
        };
        let icon = match &node.icon {
            Some(i) => i.clone(),
            None => continue,
        };
        let path = format!("{}/nodes/{}.svg", provider, icon);
        for source in get_sources(&provider) {
            if let Some(svg) = source.fetch_svg(&path) {
                node.icon_svg = Some(sanitize_svg(&svg));
                break;
            }
        }
    }

    // Resolve cluster icons
    for cluster in &mut ir.clusters {
        if cluster.icon_svg.is_some() {
            continue;
        }
        let provider = match &cluster.provider {
            Some(p) if declared.contains_key(p) => p.clone(),
            _ => continue,
        };
        let cluster_type = match &cluster.cluster_type {
            Some(t) => t.clone(),
            None => continue,
        };
        let path = format!("{}/clusters/{}.svg", provider, cluster_type);
        for source in get_sources(&provider) {
            if let Some(svg) = source.fetch_svg(&path) {
                cluster.icon_svg = Some(sanitize_svg(&svg));
                break;
            }
        }
    }
}

// ─── Helpers ───

fn sanitize_svg(svg: &str) -> String {
    let mut result = svg.to_string();
    while let Some(start) = result.to_lowercase().find("<script") {
        if let Some(end) = result.to_lowercase()[start..].find("</script>") {
            result = format!("{}{}", &result[..start], &result[start + end + 9..]);
        } else {
            break;
        }
    }
    result
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockSource {
        files: HashMap<String, String>,
    }

    impl IconSource for MockSource {
        fn fetch_svg(&self, path: &str) -> Option<String> {
            self.files.get(path).cloned()
        }

        fn fetch_manifest(&self, path: &str) -> Option<ProviderManifest> {
            let content = self.files.get(path)?;
            serde_json::from_str(content).ok()
        }
    }

    #[test]
    fn test_resolve_node_icons() {
        let mut ir = crate::dsl::parse_dsl("use aws\naws:EC2 Web >> aws:RDS DB").unwrap();
        let mut files = HashMap::new();
        files.insert(
            "aws/manifest.json".into(),
            r#"{"provider":"aws","node_render_mode":"icon_only","nodes":["ec2","rds"]}"#.into(),
        );
        files.insert("aws/nodes/ec2.svg".into(), "<svg>ec2</svg>".into());
        files.insert("aws/nodes/rds.svg".into(), "<svg>rds</svg>".into());

        let source = MockSource { files };
        resolve_ir(&mut ir, &[&source]);

        assert!(ir.nodes[0].icon_svg.is_some());
        assert!(ir.nodes[1].icon_svg.is_some());
        assert_eq!(
            ir.metadata.node_render_modes.get("aws"),
            Some(&"icon_only".to_string())
        );
    }

    #[test]
    fn test_resolve_cluster_styles() {
        let mut ir = crate::dsl::parse_dsl(
            "use aws\ncluster:aws:vpc My VPC {\n  aws:EC2 Web\n}\naws:EC2 Web >> Node B",
        )
        .unwrap();
        let mut files = HashMap::new();
        files.insert(
            "aws/manifest.json".into(),
            r##"{"provider":"aws","cluster_styles":{"vpc":{"stroke":"#8C4FFF","fill":"rgba(140,79,255,0.04)","stroke_dasharray":"6 3","corner_radius":0}}}"##.into(),
        );
        files.insert("aws/nodes/ec2.svg".into(), "<svg>ec2</svg>".into());

        let source = MockSource { files };
        resolve_ir(&mut ir, &[&source]);

        let style = ir.clusters[0].style.as_ref().unwrap();
        assert_eq!(style.stroke, Some("#8C4FFF".to_string()));
    }

    #[test]
    fn test_no_resolve_without_use() {
        let mut ir = crate::dsl::parse_dsl("Node A >> Node B").unwrap();
        let source = MockSource {
            files: HashMap::new(),
        };
        resolve_ir(&mut ir, &[&source]);
        assert!(ir.nodes[0].icon_svg.is_none());
    }

    #[test]
    fn test_sanitize_svg() {
        let dirty = r#"<svg><script>alert('xss')</script><circle/></svg>"#;
        let clean = sanitize_svg(dirty);
        assert!(!clean.contains("script"));
        assert!(clean.contains("circle"));
    }

    #[test]
    fn test_build_sources_default() {
        let sources = build_sources_for(&None);
        // Should have at least 1 source (official registry), maybe 2 (local + registry)
        assert!(!sources.is_empty());
    }

    #[test]
    fn test_build_sources_github() {
        let sources = build_sources_for(&Some("github:my-org/icons".into()));
        // Cache + HTTP
        assert!(!sources.is_empty());
    }

    #[test]
    fn test_build_sources_url() {
        let sources = build_sources_for(&Some("https://cdn.example.com/icons".into()));
        assert!(!sources.is_empty());
    }

    #[test]
    fn test_build_sources_local_path() {
        // Path doesn't exist, so no source created
        let sources = build_sources_for(&Some("./nonexistent-path".into()));
        assert!(sources.is_empty());
    }
}
