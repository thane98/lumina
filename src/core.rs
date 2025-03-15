use anyhow::{anyhow, Context, Result};
use egui::Modifiers;
use indexmap::IndexMap;
use mila::{fe9_arc, BinArchive, Endian, LZ10CompressionFormat};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use walkdir::WalkDir;

pub enum Message {
    Success(String),
    Error(String),
}

pub struct Task {
    pub path: PathBuf,
    pub receiver: Receiver<Vec<Message>>,
    pub done: bool,
}

impl Task {
    pub fn new(path: PathBuf, receiver: Receiver<Vec<Message>>) -> Self {
        Self {
            path,
            receiver,
            done: false,
        }
    }

    pub fn poll(&mut self) -> Option<Vec<Message>> {
        match self.receiver.try_recv() {
            Ok(message) => {
                self.done = true;
                Some(message)
            }
            Err(err) => match err {
                TryRecvError::Empty => None,
                TryRecvError::Disconnected => {
                    self.done = true;
                    Some(vec![Message::Error("Thread panicked.".into())])
                }
            },
        }
    }
}

pub fn spawn_worker(path: PathBuf, modifiers: Modifiers) -> Task {
    let (sender, receiver) = std::sync::mpsc::channel();
    let task = Task::new(path.clone(), receiver);
    std::thread::spawn(move || {
        let path = path.as_path();
        let message = if path.is_dir() {
            pack_cmp(path)
        } else if path.is_file() {
            if modifiers.command_only() {
                compress_bin(path)
            } else if modifiers.shift_only() {
                decompress_bin(path)
            } else if is_extension(path, "cmp") {
                extract_cmp(path)
            } else if is_extension(path, "cms") {
                extract_cms(path)
            } else if is_extension(path, "m") {
                extract_message(path)
            } else if is_extension(path, "bin") {
                extract_bin(path)
            } else if is_extension(path, "yml") {
                pack_message(path)
            } else {
                Err(anyhow!(
                    "Unsupported file extension for path '{}'",
                    path.display()
                ))
            }
        } else {
            Err(anyhow!("Bad path '{}'", path.display()))
        };

        sender.send(match message {
            Ok(messages) => messages,
            Err(err) => vec![
                Message::Error(format!("Failed to process path '{}", path.display())),
                Message::Error(format!("{:?}", err)),
            ],
        })
    });
    task
}

fn is_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|p| p.to_str())
        .map(|p| p == extension)
        .unwrap_or_default()
}

fn pack_cmp(path: &Path) -> Result<Vec<Message>> {
    let components = path.components().count();
    let mut files = IndexMap::new();
    for entry in WalkDir::new(path).sort_by_file_name() {
        let entry = entry?;
        if entry.path().is_file() {
            let path_without_parent: PathBuf = entry.path().components().skip(components).collect();
            let key = path_without_parent
                .to_string_lossy()
                .to_string()
                .replace("\\", "/");
            let raw = std::fs::read(entry.path())
                .with_context(|| format!("Failed to read path '{}'", entry.path().display()))?;
            files.insert(key, raw);
        }
    }

    let output_path = match path.extension() {
        Some(_) => path.with_extension(
            path.extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
                + ".cmp",
        ),
        None => path.with_extension("cmp"),
    };
    let cmp = fe9_arc::serialize(&files)?;
    let compressed = LZ10CompressionFormat {}.compress(&cmp)?;
    std::fs::write(&output_path, compressed)?;
    Ok(vec![Message::Success(format!(
        "Packed '{}' files under path '{}' to cmp '{}'",
        files.len(),
        path.display(),
        output_path.display()
    ))])
}

fn extract_cmp(path: &Path) -> Result<Vec<Message>> {
    let raw = std::fs::read(path)?;
    let decompressed = LZ10CompressionFormat {}.decompress(&raw)?;
    let arc = fe9_arc::parse(&decompressed)?;

    let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("Unable to determine parent path."))?;
    let write_dir = parent.join(file_name);

    let mut messages = vec![];
    for (k, v) in arc {
        let key = if cfg!(windows) {
            k.replace("/", "\\")
        } else {
            k
        };

        let write_path = write_dir.join(&key);
        if let Some(parent) = Path::new(&key).parent() {
            std::fs::create_dir_all(write_dir.join(parent))?;
        }
        std::fs::write(&write_path, v)?;
        messages.push(Message::Success(format!(
            "Extracted cmp file to path '{}'",
            write_path.display()
        )));
    }
    Ok(messages)
}

fn extract_cms(path: &Path) -> Result<Vec<Message>> {
    let raw = std::fs::read(path)?;
    let decompressed = LZ10CompressionFormat {}.decompress(&raw)?;
    let output_path = path.with_extension("bin");
    std::fs::write(&output_path, &decompressed)?;

    let mut messages = vec![Message::Success(format!(
        "Decompressed cms '{}' to path '{}'",
        path.display(),
        output_path.display()
    ))];

    if let Ok(message) = extract_bin_from_bytes(path, &decompressed) {
        messages.extend(message);
    }

    Ok(messages)
}

fn extract_bin(path: &Path) -> Result<Vec<Message>> {
    let raw = std::fs::read(path)?;
    extract_bin_from_bytes(path, &raw)
}

fn extract_bin_from_bytes(path: &Path, raw: &[u8]) -> Result<Vec<Message>> {
    let archive = BinArchive::from_bytes(raw, Endian::Big)?;
    let text = crate::unpack_bin_archive(&archive)?;
    let output_path = path.with_extension("txt");
    std::fs::write(&output_path, text)?;
    Ok(vec![Message::Success(format!(
        "Extracted bin archive '{}' to path '{}'",
        path.display(),
        output_path.display()
    ))])
}

fn extract_message(path: &Path) -> Result<Vec<Message>> {
    let raw = std::fs::read(path)?;
    let message_archive =
        mila::TextArchive::from_bytes(&raw, mila::TextArchiveFormat::ShiftJIS, mila::Endian::Big)?;
    let escaped_archive: IndexMap<String, String> = message_archive
        .get_entries()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let raw = serde_yaml::to_string(&escaped_archive)?;
    let output_path = path.with_extension("yml");
    std::fs::write(&output_path, raw)?;
    Ok(vec![Message::Success(format!(
        "Extracted message archive from path '{}' to '{}'",
        path.display(),
        output_path.display()
    ))])
}

fn compress_bin(path: &Path) -> Result<Vec<Message>> {
    let raw = std::fs::read(path)?;
    let compressed = LZ10CompressionFormat {}.compress(&raw)?;
    let output_path = path.with_extension("cms");
    std::fs::write(&output_path, compressed)?;
    Ok(vec![Message::Success(format!(
        "Compressed path '{}' to path '{}'",
        path.display(),
        output_path.display()
    ))])
}

fn decompress_bin(path: &Path) -> Result<Vec<Message>> {
    let raw = std::fs::read(path)?;
    let decompressed = LZ10CompressionFormat {}.decompress(&raw)?;
    let output_path = path.with_extension("bin");
    std::fs::write(&output_path, decompressed)?;
    Ok(vec![Message::Success(format!(
        "Decompressed path '{}' to path '{}'",
        path.display(),
        output_path.display()
    ))])
}

fn pack_message(path: &Path) -> Result<Vec<Message>> {
    let raw = std::fs::read_to_string(path)?;
    let mut message_archive =
        mila::TextArchive::new(mila::TextArchiveFormat::ShiftJIS, mila::Endian::Big);
    let entries: IndexMap<String, String> = serde_yaml::from_str(&raw)?;
    for (k, v) in entries {
        message_archive.set_message(&k, &v);
    }
    let raw = message_archive.serialize()?;
    let output_path = path.with_extension("m");
    std::fs::write(&output_path, raw)?;
    Ok(vec![Message::Success(format!(
        "Packed message archive '{}' to path '{}'",
        path.display(),
        output_path.display()
    ))])
}
