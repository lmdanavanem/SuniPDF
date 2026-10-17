//! Per-user encrypted visual signatures; legacy migration never removes an unverified original.

use image::{ImageFormat, RgbaImage};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use sunipdf_core::EngineError;
use zeroize::Zeroizing;

const MAGIC: &[u8] = b"SUNISIG\x01";
const MAX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_BLOB: u64 = MAX_BYTES + 65536;

fn failure(message: &str) -> EngineError {
    EngineError::Other(message.into())
}

fn checked_path(dir: &Path, path: &Path, exists: bool) -> Result<(), EngineError> {
    if dir
        .components()
        .chain(path.components())
        .any(|p| matches!(p, Component::ParentDir))
        || path.parent() != Some(dir)
        || path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(super::SignKind::from_file_name)
            .is_none()
    {
        return Err(failure("signature library path is invalid"));
    }
    for ancestor in dir.ancestors() {
        let metadata = std::fs::symlink_metadata(ancestor)?;
        if !metadata.is_dir() || native::is_reparse(&metadata) {
            return Err(failure(
                "signature library cannot use redirected directories",
            ));
        }
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !native::is_reparse(&metadata) => Ok(()),
        Err(error) if !exists && error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(failure(
            "signature library file is unavailable or redirected",
        )),
    }
}

fn read(dir: &Path, path: &Path, max: u64) -> Result<Zeroizing<Vec<u8>>, EngineError> {
    checked_path(dir, path, true)?;
    let file = native::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || native::is_reparse(&metadata) || metadata.len() > max {
        return Err(failure(
            "signature library file exceeds its limit or is redirected",
        ));
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(failure("signature library file exceeds its limit"));
    }
    Ok(bytes)
}

fn decode(bytes: &[u8]) -> Result<RgbaImage, EngineError> {
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(super::MAX_SIDE);
    limits.max_image_height = Some(super::MAX_SIDE);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map(image::DynamicImage::into_rgba8)
        .map_err(|_| failure("signature library PNG is invalid or oversized"))
}

fn protect(png: &[u8]) -> Result<Vec<u8>, EngineError> {
    if png.len() as u64 > MAX_BYTES {
        return Err(failure("signature library PNG exceeds its limit"));
    }
    let sid = native::sid()?;
    let encrypted = native::crypt(png, true)?;
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&(sid.len() as u16).to_le_bytes());
    bytes.extend_from_slice(sid.as_bytes());
    bytes.extend_from_slice(&encrypted);
    Ok(bytes)
}

fn unprotect(bytes: &[u8]) -> Result<Zeroizing<Vec<u8>>, EngineError> {
    if bytes.len() as u64 > MAX_BLOB || !bytes.starts_with(MAGIC) {
        return Err(failure("signature library encrypted file is invalid"));
    }
    let offset = MAGIC.len();
    let length = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| failure("signature library header is incomplete"))?;
    let length = usize::from(u16::from_le_bytes([length[0], length[1]]));
    if !(1..=256).contains(&length) {
        return Err(failure("signature library profile identifier is invalid"));
    }
    let end = offset + 2 + length;
    let sid = bytes
        .get(offset + 2..end)
        .ok_or_else(|| failure("signature library header is incomplete"))?;
    if sid != native::sid()?.as_bytes() {
        return Err(failure("signature library belongs to another Windows profile; restore that profile or reimport the original signature"));
    }
    let data = bytes
        .get(end..)
        .filter(|b| !b.is_empty())
        .ok_or_else(|| failure("signature library encrypted data is missing"))?;
    let plain = native::crypt(data, false)?;
    if plain.len() as u64 > MAX_BYTES {
        return Err(failure(
            "signature library decrypted file exceeds its limit",
        ));
    }
    Ok(plain)
}

fn store(dir: &Path, path: &Path, png: &[u8]) -> Result<(), EngineError> {
    checked_path(dir, path, false)?;
    let bytes = protect(png)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".signature-")
        .tempfile_in(dir)?;
    native::private_acl(temporary.path())?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(path)
        .map_err(|e| EngineError::Io(e.error))?;
    let saved = read(dir, path, MAX_BLOB)?;
    if &*unprotect(&saved)? != png {
        return Err(failure("signature library persisted verification failed"));
    }
    Ok(())
}

pub(super) fn save(dir: &Path, name: &str, image: &RgbaImage) -> Result<PathBuf, EngineError> {
    if dir.components().any(|p| matches!(p, Component::ParentDir)) {
        return Err(failure("signature library path is invalid"));
    }
    for ancestor in dir.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.is_dir() && !native::is_reparse(&metadata) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => {
                return Err(failure(
                    "signature library cannot use redirected directories",
                ))
            }
        }
    }
    std::fs::create_dir_all(dir)?;
    let path = dir.join(name).with_extension("dpapi");
    checked_path(dir, &path, false)?;
    let mut png = Zeroizing::new(Vec::new());
    image::write_buffer_with_format(
        &mut Cursor::new(&mut *png),
        image.as_raw(),
        image.width(),
        image.height(),
        image::ColorType::Rgba8,
        ImageFormat::Png,
    )
    .map_err(|_| failure("signature library PNG encoding failed"))?;
    decode(&png)?;
    store(dir, &path, &png)?;
    Ok(path)
}

fn migrate(dir: &Path, path: &Path, png: &[u8]) -> Result<PathBuf, EngineError> {
    let protected = path.with_extension("dpapi");
    if protected.exists() {
        let existing = read(dir, &protected, MAX_BLOB)?;
        if &*unprotect(&existing)? != png {
            return Err(failure(
                "signature library migration conflicts with an existing encrypted signature",
            ));
        }
        native::private_acl(&protected)?;
    } else {
        store(dir, &protected, png)?;
    }
    if &*read(dir, path, MAX_BYTES)? != png {
        return Err(failure(
            "signature library original changed during migration",
        ));
    }
    std::fs::remove_file(path)?;
    Ok(protected)
}

pub(crate) fn load(dir: &Path, path: &Path) -> Result<(PathBuf, RgbaImage), EngineError> {
    let protected = path.extension().is_some_and(|e| e == "dpapi");
    let bytes = read(dir, path, if protected { MAX_BLOB } else { MAX_BYTES })?;
    if protected {
        return Ok((path.into(), decode(&unprotect(&bytes)?)?));
    }
    let image = decode(&bytes)?;
    match migrate(dir, path, &bytes) {
        Ok(protected) => Ok((protected, image)),
        Err(_) => Ok((path.into(), image)),
    }
}

pub(super) fn delete(dir: &Path, path: &Path) -> Result<(), EngineError> {
    checked_path(dir, path, true)?;
    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree},
        Security::{Authorization::*, Cryptography::*, *},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    pub(super) fn is_reparse(metadata: &std::fs::Metadata) -> bool {
        metadata.file_attributes() & 0x400 != 0
    }

    pub(super) fn open(path: &Path) -> std::io::Result<std::fs::File> {
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(0x00200000)
            .open(path)
    }

    pub(super) fn sid() -> Result<String, EngineError> {
        let mut token = std::ptr::null_mut();
        // SAFETY: current-process pseudo-handle and initialized output remain live through each call.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let mut size = 0;
        // SAFETY: null first buffer queries the allocation required for TOKEN_USER.
        unsafe {
            GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut size);
        }
        if size < std::mem::size_of::<TOKEN_USER>() as u32 || size > 65536 {
            unsafe {
                CloseHandle(token);
            }
            return Err(failure("Windows profile identifier is unavailable"));
        }
        let mut data = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
        // SAFETY: word-aligned initialized storage has the queried capacity; the token is owned here.
        let ok = unsafe {
            GetTokenInformation(token, TokenUser, data.as_mut_ptr().cast(), size, &mut size)
        };
        unsafe {
            CloseHandle(token);
        }
        if ok == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let user = unsafe { &*data.as_ptr().cast::<TOKEN_USER>() };
        let mut text = std::ptr::null_mut();
        // SAFETY: the SID points into the live token-information buffer; Windows allocates the string.
        if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if text.is_null() {
            return Err(failure("Windows profile identifier is unavailable"));
        }
        let mut len = 0;
        while len < 256 && unsafe { *text.add(len) } != 0 {
            len += 1;
        }
        let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(text, len) });
        unsafe {
            LocalFree(text.cast());
        }
        let sid = sid.map_err(|_| failure("Windows profile identifier is invalid"))?;
        if len == 256 || !sid.starts_with("S-1-") {
            return Err(failure("Windows profile identifier is invalid"));
        }
        Ok(sid)
    }

    pub(super) fn private_acl(path: &Path) -> Result<(), EngineError> {
        let sddl: Vec<u16> = format!("D:P(A;;FA;;;{})", sid()?)
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut descriptor = std::ptr::null_mut();
        // SAFETY: the terminated SDDL is live; the returned descriptor is released with LocalFree.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        if descriptor.is_null() {
            return Err(failure(
                "Windows signature file permissions are unavailable",
            ));
        }
        let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: path and descriptor are live; a protected DACL grants only this user's SID access.
        let ok = unsafe {
            SetFileSecurityW(
                name.as_ptr(),
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                descriptor,
            )
        };
        unsafe {
            LocalFree(descriptor);
        }
        if ok == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }

    pub(super) fn crypt(data: &[u8], protect: bool) -> Result<Zeroizing<Vec<u8>>, EngineError> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(data.len())
                .map_err(|_| failure("signature protection input exceeds its limit"))?,
            pbData: data.as_ptr().cast_mut(),
        };
        let entropy = CRYPT_INTEGER_BLOB {
            cbData: MAGIC.len() as u32,
            pbData: MAGIC.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        // SAFETY: DPAPI owns its output; inputs remain live and no machine-wide or interactive flag is set.
        let ok = unsafe {
            if protect {
                CryptProtectData(
                    &input,
                    std::ptr::null(),
                    &entropy,
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            } else {
                CryptUnprotectData(
                    &input,
                    std::ptr::null_mut(),
                    &entropy,
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            }
        };
        if ok == 0 {
            return Err(failure(
                "Windows signature protection is unavailable for this profile",
            ));
        }
        if output.pbData.is_null() || u64::from(output.cbData) > MAX_BLOB {
            if !output.pbData.is_null() {
                unsafe {
                    LocalFree(output.pbData.cast());
                }
            }
            return Err(failure(
                "Windows signature protection returned an invalid size",
            ));
        }
        let source =
            unsafe { std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize) };
        let bytes = Zeroizing::new(source.to_vec());
        use zeroize::Zeroize;
        source.zeroize();
        unsafe {
            LocalFree(output.pbData.cast());
        }
        Ok(bytes)
    }
}

#[cfg(not(windows))]
mod native {
    use super::*;
    pub(super) fn is_reparse(metadata: &std::fs::Metadata) -> bool {
        metadata.file_type().is_symlink()
    }
    pub(super) fn open(path: &Path) -> std::io::Result<std::fs::File> {
        std::fs::File::open(path)
    }
    pub(super) fn sid() -> Result<String, EngineError> {
        Err(failure("Windows profile protection is unavailable"))
    }
    pub(super) fn private_acl(_: &Path) -> Result<(), EngineError> {
        Err(failure("Windows profile protection is unavailable"))
    }
    pub(super) fn crypt(_: &[u8], _: bool) -> Result<Zeroizing<Vec<u8>>, EngineError> {
        Err(failure("Windows profile protection is unavailable"))
    }
}

#[cfg(test)]
#[path = "sign_library_tests.rs"]
mod tests;
