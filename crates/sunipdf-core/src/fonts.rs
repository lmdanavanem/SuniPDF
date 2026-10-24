//! Installed font discovery with native Unicode names and explicit collection faces.
//! Font format, embedding rights and subsetting are validated before PDFium loading.

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
    /// Zero-based face within a TrueType/OpenType collection.
    pub face_index: u32,
}

#[cfg(windows)]
const FONTS_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";

/// Installed fonts, sorted by display name without duplicates. Unreadable files are skipped.
pub fn system_fonts() -> Vec<SystemFont> {
    #[cfg(windows)]
    {
        let fonts_dir = std::env::var_os("WINDIR")
            .map(|w| Path::new(&w).join("Fonts"))
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows\Fonts"));
        let entries = {
            use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
            let mut entries = Vec::new();
            for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
                for (name, file) in registry_fonts(hive, FONTS_KEY) {
                    let path = Path::new(&file);
                    let path = if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        fonts_dir.join(path)
                    };
                    entries.push((name, path));
                }
            }
            entries
        };
        collect_system_fonts(entries, &fonts_dir)
    }
    #[cfg(target_os = "linux")]
    {
        let mut roots = vec![
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
        ];
        if let Some(home) = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
        {
            roots.push(home.join(".local/share/fonts"));
            roots.push(home.join(".fonts"));
        }
        let entries = discover_font_files(&roots);
        // No implicit current-directory fallback when all configured directories are absent.
        if entries.is_empty() {
            Vec::new()
        } else {
            collect_system_fonts(entries, Path::new("/usr/share/fonts"))
        }
    }
    // System, machine then user folders; no registry on macOS.
    #[cfg(target_os = "macos")]
    {
        let mut roots = vec![
            PathBuf::from("/System/Library/Fonts"),
            PathBuf::from("/Library/Fonts"),
        ];
        if let Some(home) = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
        {
            roots.push(home.join("Library/Fonts"));
        }
        let entries = discover_font_files(&roots);
        if entries.is_empty() {
            Vec::new()
        } else {
            collect_system_fonts(entries, Path::new("/System/Library/Fonts"))
        }
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    Vec::new()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn discover_font_files(roots: &[PathBuf]) -> Vec<(String, PathBuf)> {
    const MAX_ENTRIES: usize = 8192;
    const MAX_DEPTH: usize = 8;
    let mut remaining = MAX_ENTRIES;
    let mut entries = Vec::new();
    let mut pending: Vec<_> = roots
        .iter()
        .filter(|path| path.is_absolute())
        .rev()
        .map(|path| (path.clone(), 0))
        .collect();
    while let Some((directory, depth)) = pending.pop() {
        if remaining == 0 {
            break;
        }
        if !std::fs::symlink_metadata(&directory).is_ok_and(|m| m.file_type().is_dir()) {
            continue;
        }
        let Ok(children) = std::fs::read_dir(directory) else {
            continue;
        };
        let mut children = children.take(remaining).collect::<Vec<_>>();
        remaining -= children.len();
        children.sort_by_key(|entry| entry.as_ref().ok().map(|e| e.file_name()));
        for child in children.into_iter().flatten() {
            let Ok(kind) = child.file_type() else {
                continue;
            };
            let path = child.path();
            if kind.is_dir() && depth < MAX_DEPTH {
                pending.push((path, depth + 1));
            } else if kind.is_file() && is_font_file(&path) {
                let name = path
                    .file_stem()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                entries.push((name, path));
            }
        }
    }
    entries
}

fn collect_system_fonts(mut entries: Vec<(String, PathBuf)>, fonts_dir: &Path) -> Vec<SystemFont> {
    if entries.is_empty() {
        // Repli : le dossier des polices, nom = nom de fichier.
        if let Ok(dir) = std::fs::read_dir(fonts_dir) {
            for e in dir.flatten() {
                let path = e.path();
                if is_font_file(&path) {
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
        .flat_map(|(name, path)| {
            let Some(size_bytes) = std::fs::metadata(&path)
                .ok()
                .filter(|m| m.is_file())
                .map(|m| m.len())
            else {
                return Vec::new();
            };
            if is_ttf(&path) {
                return vec![SystemFont {
                    name,
                    path,
                    size_bytes,
                    face_index: 0,
                }];
            }
            let Ok(bytes) = crate::font_embed::read_font(&path) else {
                return Vec::new();
            };
            let Ok(faces) = crate::font_embed::faces(&bytes) else {
                return Vec::new();
            };
            faces
                .into_iter()
                .map(|(face_index, face_name)| SystemFont {
                    name: if faces_are_collection(&path) {
                        format!("{face_name} [face {}]", face_index + 1)
                    } else {
                        name.clone()
                    },
                    path: path.clone(),
                    size_bytes,
                    face_index,
                })
                .collect::<Vec<_>>()
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

fn is_font_file(path: &Path) -> bool {
    path.extension().is_some_and(|e| {
        ["ttf", "otf", "ttc", "otc"]
            .iter()
            .any(|ext| e.eq_ignore_ascii_case(ext))
    })
}

fn faces_are_collection(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ttc") || e.eq_ignore_ascii_case("otc"))
}

#[cfg(windows)]
fn registry_fonts(
    hive: windows::Win32::System::Registry::HKEY,
    subkey: &str,
) -> Vec<(String, String)> {
    use windows::core::{Owned, PCWSTR, PWSTR};
    use windows::Win32::Foundation::{ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
    use windows::Win32::System::Registry::*;

    let subkey: Vec<u16> = subkey.encode_utf16().chain(Some(0)).collect();
    let mut key = HKEY::default();
    if unsafe {
        RegOpenKeyExW(
            hive,
            PCWSTR(subkey.as_ptr()),
            None,
            KEY_QUERY_VALUE,
            &mut key,
        )
    }
    .is_err()
    {
        return Vec::new();
    }
    let key = unsafe { Owned::new(key) };
    let mut count = 0;
    if unsafe {
        RegQueryInfoKeyW(
            *key,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut count),
            None,
            None,
            None,
            None,
        )
    }
    .is_err()
    {
        return Vec::new();
    }
    // Windows limits value names to 16,383 UTF-16 units and extended paths to 32,767.
    let mut name = vec![0u16; 16_384];
    let mut data = vec![0u16; 32_768];
    let mut fonts = Vec::new();
    for index in 0..count {
        let mut name_len = name.len() as u32;
        let mut data_len = (data.len() * 2) as u32;
        let mut kind = 0;
        let result = unsafe {
            RegEnumValueW(
                *key,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                None,
                Some(&mut kind),
                Some(data.as_mut_ptr().cast()),
                Some(&mut data_len),
            )
        };
        if result == ERROR_NO_MORE_ITEMS {
            break;
        }
        if result == ERROR_MORE_DATA {
            continue;
        }
        if result != ERROR_SUCCESS {
            break;
        }
        if kind != REG_SZ.0 || !data_len.is_multiple_of(2) {
            continue;
        }
        let (Some(name), Some(data)) = (
            name.get(..name_len as usize),
            data.get(..data_len as usize / 2),
        ) else {
            continue;
        };
        let data = data.strip_suffix(&[0]).unwrap_or(data);
        if data.contains(&0) {
            continue;
        }
        let (Ok(name), Ok(file)) = (String::from_utf16(name), String::from_utf16(data)) else {
            continue;
        };
        let name = name.trim();
        let name = name
            .strip_suffix("(TrueType)")
            .or_else(|| name.strip_suffix("(OpenType)"))
            .unwrap_or(name)
            .trim();
        let file = file.trim();
        if !name.is_empty() && is_font_file(Path::new(file)) {
            fonts.push((name.to_string(), file.to_string()));
        }
    }
    fonts
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
        return embedding_permissions(fs_type).map(|_| ());
    }
    Ok(()) // pas de table OS/2 : aucune restriction déclarée
}

/// Whether editable/installable outline embedding also permits subsetting.
pub(crate) fn embedding_permissions(fs_type: u16) -> Result<bool, EngineError> {
    let usage = fs_type & 0x000E;
    // Retain the least-restrictive interpretation for older fonts with multiple usage bits.
    let allowed = usage == 0 || usage & 0x0008 != 0;
    if !allowed || fs_type & 0x0200 != 0 || fs_type & !0x030E != 0 {
        Err(EngineError::FontNotEmbeddable)
    } else {
        Ok(fs_type & 0x0100 == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn linux_font_discovery_finds_nested_fonts_without_following_links() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("fonts");
        let nested = root.join("truetype/dejavu");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("Readable.ttf"), [1, 2, 3]).unwrap();
        std::fs::write(nested.join("README"), [1]).unwrap();
        let outside = temp.path().join("Outside.ttf");
        std::fs::write(&outside, [1]).unwrap();
        symlink(&outside, nested.join("Linked.ttf")).unwrap();
        symlink(&root, nested.join("loop")).unwrap();
        let linked_root = temp.path().join("linked-root");
        symlink(&root, &linked_root).unwrap();
        assert!(discover_font_files(&[linked_root]).is_empty());
        let entries = discover_font_files(&[root]);
        assert_eq!(
            entries,
            vec![("Readable".into(), nested.join("Readable.ttf"))]
        );
        assert_eq!(collect_system_fonts(entries, temp.path()).len(), 1);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn linux_font_discovery_has_a_depth_bound() {
        let temp = tempfile::tempdir().unwrap();
        let mut directory = temp.path().to_path_buf();
        for level in 0..10 {
            directory.push(format!("level-{level}"));
            std::fs::create_dir(&directory).unwrap();
        }
        std::fs::write(directory.join("TooDeep.ttf"), [1]).unwrap();
        assert!(discover_font_files(&[temp.path().to_path_buf()]).is_empty());
    }

    #[test]
    fn font_listing_keeps_sort_precedence_files_and_folder_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let machine = dir.path().join("machine.ttf");
        let user = dir.path().join("user.ttf");
        let unicode = dir.path().join("字体-é.TTF");
        for path in [&machine, &user, &unicode] {
            std::fs::write(path, [1, 2, 3]).unwrap();
        }
        std::fs::write(dir.path().join("unsupported.otf"), [1]).unwrap();
        std::fs::create_dir(dir.path().join("directory.ttf")).unwrap();
        let fonts = collect_system_fonts(
            vec![
                ("Shared".into(), machine.clone()),
                ("sHARED".into(), user),
                ("École 中文".into(), unicode.clone()),
                ("Missing".into(), dir.path().join("missing.ttf")),
                ("Directory".into(), dir.path().join("directory.ttf")),
            ],
            dir.path(),
        );
        assert_eq!(fonts.len(), 2);
        assert_eq!(
            (fonts[0].name.as_str(), &fonts[0].path),
            ("Shared", &machine)
        );
        assert_eq!(fonts[1].name, "École 中文");
        assert_eq!(fonts[1].path, unicode);
        assert!(fonts.iter().all(|font| font.size_bytes == 3));
        let fallback = collect_system_fonts(Vec::new(), dir.path());
        assert_eq!(fallback.len(), 3);
        assert!(fallback.iter().any(|font| font.name == "字体-é"));
        assert!(collect_system_fonts(
            vec![("Missing".into(), dir.path().join("missing.ttf"))],
            dir.path(),
        )
        .is_empty());
    }

    #[cfg(windows)]
    struct TestKey {
        handle: windows::core::Owned<windows::Win32::System::Registry::HKEY>,
        name: Vec<u16>,
    }

    #[cfg(windows)]
    impl TestKey {
        fn new(dir: &Path) -> Self {
            use windows::core::PCWSTR;
            use windows::Win32::System::Registry::*;
            let name: Vec<u16> = format!(
                "Software\\SuniPDF-Font-Test-{}",
                dir.file_name().unwrap().to_string_lossy(),
            )
            .encode_utf16()
            .chain(Some(0))
            .collect();
            let mut handle = HKEY::default();
            let mut disposition = REG_CREATE_KEY_DISPOSITION::default();
            unsafe {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(name.as_ptr()),
                    None,
                    PCWSTR::null(),
                    REG_OPTION_VOLATILE,
                    KEY_READ | KEY_WRITE,
                    None,
                    &mut handle,
                    Some(&mut disposition),
                )
                .ok()
                .unwrap();
                let handle = windows::core::Owned::new(handle);
                assert_eq!(disposition, REG_CREATED_NEW_KEY);
                Self { handle, name }
            }
        }

        fn set(
            &self,
            name: &str,
            kind: windows::Win32::System::Registry::REG_VALUE_TYPE,
            data: &[u8],
        ) {
            use windows::core::PCWSTR;
            use windows::Win32::System::Registry::RegSetValueExW;
            let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            unsafe {
                RegSetValueExW(*self.handle, PCWSTR(name.as_ptr()), None, kind, Some(data))
                    .ok()
                    .unwrap();
            }
        }

        fn entries(&self) -> Vec<(String, String)> {
            registry_fonts(
                windows::Win32::System::Registry::HKEY_CURRENT_USER,
                &String::from_utf16(&self.name[..self.name.len() - 1]).unwrap(),
            )
        }
    }

    #[cfg(windows)]
    impl Drop for TestKey {
        fn drop(&mut self) {
            unsafe {
                let _ = windows::Win32::System::Registry::RegDeleteKeyW(
                    windows::Win32::System::Registry::HKEY_CURRENT_USER,
                    windows::core::PCWSTR(self.name.as_ptr()),
                );
            }
        }
    }

    #[cfg(windows)]
    fn registry_string(value: &str) -> Vec<u8> {
        value
            .encode_utf16()
            .chain(Some(0))
            .flat_map(u16::to_le_bytes)
            .collect()
    }

    #[cfg(windows)]
    #[test]
    fn registry_fonts_preserve_unicode_names_and_paths() {
        use windows::Win32::System::Registry::REG_SZ;
        let dir = tempfile::tempdir().unwrap();
        let key = TestKey::new(dir.path());
        let path = dir.path().join("字体-é.TTF");
        std::fs::write(&path, [1, 2, 3]).unwrap();
        let file = path.to_str().unwrap();
        key.set(
            "École REG_SZ 中文 (TrueType)",
            REG_SZ,
            &registry_string(file),
        );
        key.set(
            "  Gabriola (OpenType)  ",
            REG_SZ,
            &registry_string(" GABRIOLA.TTF "),
        );
        let mut entries = key.entries();
        entries.sort();
        assert_eq!(
            entries,
            vec![
                ("Gabriola".into(), "GABRIOLA.TTF".into()),
                ("École REG_SZ 中文".into(), file.into()),
            ],
        );
        let fonts = collect_system_fonts(
            entries
                .into_iter()
                .map(|(name, file)| (name, PathBuf::from(file)))
                .collect(),
            dir.path(),
        );
        assert_eq!(fonts.len(), 1);
        assert_eq!(fonts[0].name, "École REG_SZ 中文");
        assert_eq!(fonts[0].path, path);
        assert_eq!(fonts[0].size_bytes, 3);
    }

    #[cfg(windows)]
    #[test]
    fn registry_fonts_skip_invalid_entries_and_continue_after_large_values() {
        use windows::Win32::System::Registry::*;
        let dir = tempfile::tempdir().unwrap();
        let key = TestKey::new(dir.path());
        key.set("Wrong type", REG_BINARY, &registry_string("font.ttf"));
        key.set("Expandable", REG_EXPAND_SZ, &registry_string("font.ttf"));
        key.set("Collection", REG_SZ, &registry_string("font.ttc"));
        key.set("PostScript", REG_SZ, &registry_string("font.otf"));
        key.set("Empty file", REG_SZ, &registry_string(""));
        key.set("", REG_SZ, &registry_string("font.ttf"));
        key.set("(TrueType)", REG_SZ, &registry_string("font.ttf"));
        key.set("Odd bytes", REG_SZ, &[1]);
        key.set("Invalid UTF-16", REG_SZ, &[0, 0xD8, 0, 0]);
        key.set("Embedded null", REG_SZ, &registry_string("a\0font.ttf"));
        key.set("Too large", REG_SZ, &vec![1; 65_538]);
        key.set("Valid", REG_SZ, &registry_string("valid.ttf"));
        let unterminated = registry_string("unterminated.ttf");
        key.set(
            "Unterminated",
            REG_SZ,
            &unterminated[..unterminated.len() - 2],
        );
        let mut entries = key.entries();
        entries.sort();
        assert_eq!(
            entries,
            vec![
                ("Collection".into(), "font.ttc".into()),
                ("PostScript".into(), "font.otf".into()),
                ("Unterminated".into(), "unterminated.ttf".into()),
                ("Valid".into(), "valid.ttf".into()),
            ],
        );
        let name = String::from_utf16(&key.name[..key.name.len() - 1]).unwrap();
        drop(key);
        assert!(registry_fonts(HKEY_CURRENT_USER, &name).is_empty());
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let mut handle = HKEY::default();
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                windows::core::PCWSTR(name.as_ptr()),
                None,
                KEY_QUERY_VALUE,
                &mut handle,
            )
        };
        let _handle = unsafe { windows::core::Owned::new(handle) };
        assert_eq!(result, windows::Win32::Foundation::ERROR_FILE_NOT_FOUND);
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
