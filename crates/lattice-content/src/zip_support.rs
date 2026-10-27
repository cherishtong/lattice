use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use flate2::read::DeflateDecoder;

use crate::{INSTALL_CANCELLED, InstallControl};

const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;

struct ZipEntry {
    name: String,
    method: u16,
    compressed_size: u64,
    uncompressed_size: u64,
    crc32: u32,
    local_offset: u64,
    directory: bool,
}

/// 解压 ZIP 到目标目录；拒绝路径穿越，并校验每个文件的 CRC32。
pub(crate) fn extract_zip(
    zip_path: &Path,
    destination: &Path,
    control: &InstallControl,
) -> Result<(), String> {
    let mut archive = File::open(zip_path).map_err(|error| format!("打开 ZIP 失败: {error}"))?;
    let length = archive
        .metadata()
        .map_err(|error| format!("读取 ZIP 信息失败: {error}"))?
        .len();
    if length < 22 {
        return Err("ZIP 文件过小或已损坏".to_owned());
    }
    let tail_length = length.min(65_557);
    archive
        .seek(SeekFrom::Start(length - tail_length))
        .map_err(|error| error.to_string())?;
    let mut tail = vec![0u8; tail_length as usize];
    archive
        .read_exact(&mut tail)
        .map_err(|error| error.to_string())?;
    let eocd = tail
        .windows(4)
        .rposition(|window| read_u32(window) == EOCD_SIGNATURE)
        .ok_or_else(|| "ZIP 缺少中央目录".to_owned())?;
    if eocd + 22 > tail.len() {
        return Err("ZIP 中央目录尾部损坏".to_owned());
    }
    let directory_size = read_u32(&tail[eocd + 12..eocd + 16]) as u64;
    let directory_offset = read_u32(&tail[eocd + 16..eocd + 20]) as u64;
    if directory_size == u32::MAX as u64 || directory_offset == u32::MAX as u64 {
        return Err("暂不支持 ZIP64 源码包".to_owned());
    }
    if directory_offset + directory_size > length {
        return Err("ZIP 中央目录越界".to_owned());
    }

    archive
        .seek(SeekFrom::Start(directory_offset))
        .map_err(|error| error.to_string())?;
    let mut directory = vec![0u8; directory_size as usize];
    archive
        .read_exact(&mut directory)
        .map_err(|error| error.to_string())?;

    let mut entries = Vec::new();
    let mut cursor = 0usize;
    while cursor + 46 <= directory.len()
        && read_u32(&directory[cursor..cursor + 4]) == CENTRAL_SIGNATURE
    {
        let method = read_u16(&directory[cursor + 10..cursor + 12]);
        let crc32 = read_u32(&directory[cursor + 16..cursor + 20]);
        let compressed_size = read_u32(&directory[cursor + 20..cursor + 24]) as u64;
        let uncompressed_size = read_u32(&directory[cursor + 24..cursor + 28]) as u64;
        let name_length = read_u16(&directory[cursor + 28..cursor + 30]) as usize;
        let extra_length = read_u16(&directory[cursor + 30..cursor + 32]) as usize;
        let comment_length = read_u16(&directory[cursor + 32..cursor + 34]) as usize;
        let local_offset = read_u32(&directory[cursor + 42..cursor + 46]) as u64;
        let name_start = cursor + 46;
        let name_end = name_start + name_length;
        if name_end > directory.len() {
            return Err("ZIP 中央目录损坏".to_owned());
        }
        let name = String::from_utf8_lossy(&directory[name_start..name_end]).replace('\\', "/");
        let directory_entry = name.ends_with('/');
        if let Some(relative) = safe_relative(&name) {
            if !is_skipped(&relative) {
                entries.push(ZipEntry {
                    name: relative.to_string_lossy().replace('\\', "/"),
                    method,
                    compressed_size,
                    uncompressed_size,
                    crc32,
                    local_offset,
                    directory: directory_entry,
                });
            }
        } else {
            return Err(format!("ZIP 包含不安全路径: {name}"));
        }
        cursor = name_end + extra_length + comment_length;
    }

    let total_files = entries.iter().filter(|entry| !entry.directory).count() as u64;
    control.set_total(total_files);
    for entry in entries {
        if control.is_cancelled() {
            return Err(INSTALL_CANCELLED.to_owned());
        }
        let target = destination.join(&entry.name);
        if entry.directory {
            fs::create_dir_all(&target).map_err(|error| error.to_string())?;
            continue;
        }
        if entry.compressed_size == u32::MAX as u64 || entry.uncompressed_size == u32::MAX as u64 {
            return Err("暂不支持 ZIP64 条目".to_owned());
        }
        archive
            .seek(SeekFrom::Start(entry.local_offset))
            .map_err(|error| error.to_string())?;
        let mut header = [0u8; 30];
        archive
            .read_exact(&mut header)
            .map_err(|error| format!("读取 ZIP 条目失败: {error}"))?;
        if read_u32(&header[..4]) != LOCAL_SIGNATURE {
            return Err(format!("ZIP 条目头损坏: {}", entry.name));
        }
        let name_length = read_u16(&header[26..28]) as u64;
        let extra_length = read_u16(&header[28..30]) as u64;
        archive
            .seek(SeekFrom::Current((name_length + extra_length) as i64))
            .map_err(|error| error.to_string())?;
        let mut compressed = vec![0u8; entry.compressed_size as usize];
        archive
            .read_exact(&mut compressed)
            .map_err(|error| format!("读取 ZIP 数据失败: {error}"))?;
        let data = match entry.method {
            0 => compressed,
            8 => {
                let mut decoder = DeflateDecoder::new(compressed.as_slice());
                let mut output = Vec::with_capacity(entry.uncompressed_size as usize);
                decoder
                    .read_to_end(&mut output)
                    .map_err(|error| format!("解压失败 {}: {error}", entry.name))?;
                output
            }
            method => return Err(format!("不支持的 ZIP 压缩方式 {method}: {}", entry.name)),
        };
        if data.len() as u64 != entry.uncompressed_size {
            return Err(format!("ZIP 条目大小不匹配: {}", entry.name));
        }
        if crc32fast::hash(&data) != entry.crc32 {
            return Err(format!("ZIP 条目校验失败: {}", entry.name));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut file = File::create(&target).map_err(|error| error.to_string())?;
        file.write_all(&data).map_err(|error| error.to_string())?;
        control.record_copy();
    }
    Ok(())
}

fn safe_relative(name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.starts_with('/') || name.contains(':') {
        return None;
    }
    let mut path = PathBuf::new();
    for part in name.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return None;
        }
        path.push(part);
    }
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path)
    }
}

fn is_skipped(relative: &Path) -> bool {
    let text = relative.to_string_lossy().replace('\\', "/");
    text.starts_with("__MACOSX/")
        || text.contains("/.git/")
        || text.starts_with(".git/")
        || text.contains("/.lattice/documents/")
        || text.starts_with(".lattice/documents/")
        || text
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("symbol-index-"))
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InstallControl;

    fn build_zip(name: &str, data: &[u8]) -> Vec<u8> {
        let crc = crc32fast::hash(data);
        let mut zip = Vec::new();
        let name_bytes = name.as_bytes();
        let local_offset = 0u32;

        zip.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&20u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&(data.len() as u32).to_le_bytes());
        zip.extend_from_slice(&(data.len() as u32).to_le_bytes());
        zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(name_bytes);
        zip.extend_from_slice(data);

        let directory_offset = zip.len() as u32;
        zip.extend_from_slice(&CENTRAL_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&20u16.to_le_bytes());
        zip.extend_from_slice(&20u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&(data.len() as u32).to_le_bytes());
        zip.extend_from_slice(&(data.len() as u32).to_le_bytes());
        zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u32.to_le_bytes());
        zip.extend_from_slice(&local_offset.to_le_bytes());
        zip.extend_from_slice(name_bytes);

        let directory_size = zip.len() as u32 - directory_offset;
        zip.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&1u16.to_le_bytes());
        zip.extend_from_slice(&1u16.to_le_bytes());
        zip.extend_from_slice(&directory_size.to_le_bytes());
        zip.extend_from_slice(&directory_offset.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip
    }

    #[test]
    fn extracts_stored_entries_and_rejects_traversal() {
        let root = std::env::temp_dir().join(format!("lattice-zip-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create");
        let zip_path = root.join("pack.zip");
        fs::write(&zip_path, build_zip("pack/demo.c", b"int value;\n")).expect("write zip");
        let destination = root.join("out");
        let control = InstallControl::new();
        extract_zip(&zip_path, &destination, &control).expect("extract");
        assert_eq!(
            fs::read_to_string(destination.join("pack/demo.c")).expect("read"),
            "int value;\n"
        );
        assert_eq!(control.copied_files(), 1);
        assert!(safe_relative("../escape.c").is_none());
        assert!(safe_relative("/absolute.c").is_none());
        let _ = fs::remove_dir_all(root);
    }
}
