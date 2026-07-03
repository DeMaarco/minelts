use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

pub const JAVA_RUNTIME_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub main_class: String,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<ArgumentsBlock>,
    pub asset_index: AssetIndex,
    pub downloads: VersionDownloads,
    pub libraries: Vec<Library>,
    pub java_version: Option<JavaVersionSpec>,
    pub minimum_launcher_version: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArgumentsBlock {
    pub game: Vec<ArgumentValue>,
    pub jvm: Vec<ArgumentValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Plain(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentList,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ArgumentList {
    Single(String),
    Multiple(Vec<String>),
}

impl ArgumentList {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s],
            Self::Multiple(v) => v,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
    pub total_size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionDownloads {
    pub client: DownloadEntry,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadEntry {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<HashMap<String, String>>,
    pub extract: Option<ExtractRules>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryDownloads {
    pub artifact: Option<DownloadEntry>,
    pub classifiers: Option<HashMap<String, DownloadEntry>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractRules {
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersionSpec {
    pub component: String,
    pub major_version: u32,
}

/// Manifiesto de runtimes Java de Mojang: plataforma → componente → entradas.
pub type JavaRuntimeManifest = HashMap<String, HashMap<String, Vec<JavaRuntimeEntry>>>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntimeEntry {
    pub manifest: JavaRuntimeManifestRef,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntimeManifestRef {
    pub url: String,
    #[allow(dead_code)]
    pub sha1: String,
    #[allow(dead_code)]
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntimeDetail {
    pub files: HashMap<String, JavaRuntimeFile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntimeFile {
    #[serde(rename = "type")]
    pub file_type: String,
    pub downloads: Option<JavaRuntimeDownloads>,
    #[allow(dead_code)]
    pub executable: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JavaRuntimeDownloads {
    pub raw: Option<DownloadEntry>,
    pub lzma: Option<DownloadEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexJson {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

pub fn fetch_version_manifest() -> Result<VersionManifest, String> {
    let body = ureq::get(VERSION_MANIFEST_URL)
        .call()
        .map_err(|e| format!("Error al descargar manifiesto de versiones: {e}"))?
        .into_string()
        .map_err(|e| format!("Error al leer manifiesto: {e}"))?;
    serde_json::from_str(&body).map_err(|e| format!("Error al parsear manifiesto: {e}"))
}

pub fn fetch_version_json(url: &str) -> Result<VersionJson, String> {
    let body = ureq::get(url)
        .call()
        .map_err(|e| format!("Error al descargar versión: {e}"))?
        .into_string()
        .map_err(|e| format!("Error al leer versión: {e}"))?;
    serde_json::from_str(&body).map_err(|e| format!("Error al parsear versión: {e}"))
}

pub fn resolve_java_runtime_entry<'a>(
    manifest: &'a JavaRuntimeManifest,
    platform: &str,
    component: &str,
) -> Result<&'a JavaRuntimeEntry, String> {
    let components = manifest
        .get(platform)
        .ok_or_else(|| format!("Plataforma {platform} no encontrada en manifiesto Java"))?;

    let entries = components
        .get(component)
        .ok_or_else(|| format!("Componente Java no encontrado: {component}"))?;

    entries
        .first()
        .ok_or_else(|| format!("Sin runtime disponible para {component} en {platform}"))
}

pub fn fetch_java_runtime_manifest() -> Result<JavaRuntimeManifest, String> {
    let body = ureq::get(JAVA_RUNTIME_MANIFEST_URL)
        .call()
        .map_err(|e| format!("Error al descargar manifiesto Java: {e}"))?
        .into_string()
        .map_err(|e| format!("Error al leer manifiesto Java: {e}"))?;
    serde_json::from_str(&body).map_err(|e| format!("Error al parsear manifiesto Java: {e}"))
}

pub fn fetch_java_runtime_detail(url: &str) -> Result<JavaRuntimeDetail, String> {
    let body = ureq::get(url)
        .call()
        .map_err(|e| format!("Error al descargar runtime Java: {e}"))?
        .into_string()
        .map_err(|e| format!("Error al leer runtime Java: {e}"))?;
    serde_json::from_str(&body).map_err(|e| format!("Error al parsear runtime Java: {e}"))
}

pub fn fetch_asset_index(url: &str) -> Result<AssetIndexJson, String> {
    let body = ureq::get(url)
        .call()
        .map_err(|e| format!("Error al descargar índice de assets: {e}"))?
        .into_string()
        .map_err(|e| format!("Error al leer índice de assets: {e}"))?;
    serde_json::from_str(&body).map_err(|e| format!("Error al parsear índice de assets: {e}"))
}

/// Convierte el nombre Maven de una librería a ruta relativa en libraries/.
pub fn maven_to_path(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return name.replace(':', "/");
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts.get(3).copied();
    let ext = parts.get(4).copied().unwrap_or("jar");

    if let Some(classifier) = classifier {
        format!("{group}/{artifact}/{version}/{artifact}-{version}-{classifier}.{ext}")
    } else {
        format!("{group}/{artifact}/{version}/{artifact}-{version}.{ext}")
    }
}

/// Resuelve el componente Java para versiones sin javaVersion en el manifiesto.
pub fn fallback_java_component(version_id: &str) -> (&'static str, u32) {
    let parts: Vec<u32> = version_id
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();

    let major = parts.first().copied().unwrap_or(1);
    let minor = parts.get(1).copied().unwrap_or(0);
    let patch = parts.get(2).copied().unwrap_or(0);

    if major >= 26 {
        ("java-runtime-epsilon", 25)
    } else if major == 1 && minor <= 16 {
        ("jre-legacy", 8)
    } else if major == 1 && minor == 17 && patch <= 1 {
        ("java-runtime-alpha", 16)
    } else if major == 1 && minor <= 20 && (minor < 20 || patch < 5) {
        ("java-runtime-gamma", 17)
    } else {
        ("java-runtime-delta", 21)
    }
}
