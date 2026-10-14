//! Polices TrueType installées sur le système, incorporables dans un PDF.
//!
//! PDFium (`FPDFText_LoadFont`, police CID Identity-H) copie le fichier **entier** dans le PDF :
//! pas de sous-ensemble. Seules les polices `.ttf` à contours TrueType sont proposées ; la
//! licence d'incorporation (`OS/2.fsType`) est vérifiée avant toute incorporation.

use crate::EngineError;
use std::path::{Path, PathBuf};

/// Taille maximale d'une police incorporée.
const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct SystemFont {
    /// Nom affiché, ex. « Arial Bold ».
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
}

const FONTS_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";

/// Polices `.ttf` installées (machine puis utilisateur), triées par nom, sans doublon. Jamais
/// d'erreur : liste vide si rien n'est lisible.
// ponytail: `reg.exe` plutôt que l'API du registre (feature `Win32_System_Registry` de la crate
// `windows`, lourde à compiler) ; passer à `RegEnumValueW` si `reg.exe` pose problème.
pub fn system_fonts() -> Vec<SystemFont> {
    let fonts_dir = std::env::var_os("WINDIR")
        .map(|w| Path::new(&w).join("Fonts"))
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows\Fonts"));
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for hive in ["HKLM", "HKCU"] {
        if let Some(out) = reg_query(&format!(r"{hive}\{FONTS_KEY}")) {
            for (name, file) in parse_reg_output(&out) {
                let path = Path::new(&file);
                let path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    fonts_dir.join(path)
                };
                entries.push((name, path));
            }
        }
    }
    if entries.is_empty() {
        // Repli : le dossier des polices, nom = nom de fichier.
        if let Ok(dir) = std::fs::read_dir(&fonts_dir) {
            for e in dir.flatten() {
                let path = e.path();
                if is_ttf(&path) {
                    let name = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    entries.push((name, path));
                }
            }
        }
    }
    let mut fonts: Vec<SystemFont> = entries
        .into_iter()
        .filter_map(|(name, path)| {
            let size_bytes = std::fs::metadata(&path).ok().filter(|m| m.is_file())?.len();
            Some(SystemFont {
                name,
                path,
                size_bytes,
            })
        })
        .collect();
    fonts.sort_by_key(|a| a.name.to_lowercase());
    fonts.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    fonts
}

fn is_ttf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ttf"))
}

fn reg_query(key: &str) -> Option<String> {
    let mut cmd = std::process::Command::new("reg");
    cmd.args(["query", key]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Lignes `    Arial Bold (TrueType)    REG_SZ    arialbd.ttf` → (« Arial Bold », « arialbd.ttf »),
/// fichiers `.ttf` seulement.
pub(crate) fn parse_reg_output(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|line| {
            let (name, file) = line.split_once("REG_SZ")?;
            let file = file.trim();
            if !is_ttf(Path::new(file)) {
                return None;
            }
            let name = name.trim();
            let name = name
                .strip_suffix("(TrueType)")
                .or_else(|| name.strip_suffix("(OpenType)"))
                .unwrap_or(name)
                .trim();
            (!name.is_empty()).then(|| (name.to_string(), file.to_string()))
        })
        .collect()
}

fn be_u16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(at)?, *b.get(at + 1)?]))
}

fn be_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Contrôle avant incorporation : contours TrueType (`00 01 00 00` ou `true` ; `OTTO` et `ttcf`
/// refusés), taille ≤ 32 Mio, licence `OS/2.fsType` permettant l'incorporation modifiable ou
/// installable (« restreinte », « aperçu et impression » et « bitmap seulement » refusées).
pub(crate) fn check_embeddable(bytes: &[u8]) -> Result<(), EngineError> {
    let refuse = Err(EngineError::FontNotEmbeddable);
    if bytes.len() < 12 || bytes.len() > MAX_FONT_BYTES {
        return refuse;
    }
    if !(bytes[..4] == [0, 1, 0, 0] || &bytes[..4] == b"true") {
        return refuse;
    }
    let Some(tables) = be_u16(bytes, 4) else {
        return refuse;
    };
    for t in 0..tables as usize {
        let rec = 12 + t * 16;
        let Some(tag) = bytes.get(rec..rec + 4) else {
            return refuse;
        };
        if tag != b"OS/2" {
            continue;
        }
        let Some(fs_type) = be_u32(bytes, rec + 8).and_then(|off| be_u16(bytes, off as usize + 8))
        else {
            return refuse;
        };
        let usage = fs_type & 0x000E;
        // Le droit le moins restrictif l'emporte (anciennes versions de la spécification).
        let allowed = usage == 0 || usage & 0x0008 != 0;
        return if allowed && fs_type & 0x0200 == 0 {
            Ok(())
        } else {
            refuse
        };
    }
    Ok(()) // pas de table OS/2 : aucune restriction déclarée
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_registry_listing() {
        let out = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts\r\n    Arial (TrueType)    REG_SZ    arial.ttf\r\n    Arial Bold (TrueType)    REG_SZ    arialbd.ttf\r\n    Cambria & Cambria Math (TrueType)    REG_SZ    cambria.ttc\r\n    Gabriola (OpenType)    REG_SZ    GABRIOLA.TTF\r\n    Nope    REG_DWORD    0x1\r\n\r\n    Mine (TrueType)    REG_SZ    C:\\Users\\x\\Fonts\\mine.ttf\r\n";
        assert_eq!(
            parse_reg_output(out),
            vec![
                ("Arial".to_string(), "arial.ttf".to_string()),
                ("Arial Bold".to_string(), "arialbd.ttf".to_string()),
                ("Gabriola".to_string(), "GABRIOLA.TTF".to_string()),
                (
                    "Mine".to_string(),
                    "C:\\Users\\x\\Fonts\\mine.ttf".to_string()
                ),
            ]
        );
    }

    /// Police minimale : en-tête sfnt, une table OS/2 dont `fsType` vaut `fs`.
    fn font(magic: &[u8; 4], fs: Option<u16>) -> Vec<u8> {
        let mut b = magic.to_vec();
        b.extend_from_slice(&(fs.is_some() as u16).to_be_bytes());
        b.extend_from_slice(&[0; 6]);
        if let Some(fs) = fs {
            b.extend_from_slice(b"OS/2");
            b.extend_from_slice(&[0; 4]);
            b.extend_from_slice(&28u32.to_be_bytes());
            b.extend_from_slice(&10u32.to_be_bytes());
            b.extend_from_slice(&[0; 8]);
            b.extend_from_slice(&fs.to_be_bytes());
        }
        b
    }

    #[test]
    fn checks_format_and_licence() {
        let ok = |b: Vec<u8>| check_embeddable(&b).is_ok();
        assert!(ok(font(&[0, 1, 0, 0], Some(0))));
        assert!(ok(font(&[0, 1, 0, 0], Some(0x0008))));
        assert!(ok(font(b"true", None)), "sans table OS/2");
        assert!(!ok(font(&[0, 1, 0, 0], Some(0x0002))), "restreinte");
        assert!(!ok(font(&[0, 1, 0, 0], Some(0x0004))), "aperçu seul");
        assert!(!ok(font(&[0, 1, 0, 0], Some(0x0208))), "bitmap seul");
        assert!(
            ok(font(&[0, 1, 0, 0], Some(0x000C))),
            "le moins restrictif l'emporte"
        );
        assert!(!ok(font(b"OTTO", Some(0))), "contours PostScript");
        assert!(!ok(font(b"ttcf", Some(0))), "collection");
        assert!(!ok(vec![0, 1, 0, 0]), "trop court");
    }
}
