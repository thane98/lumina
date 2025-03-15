use mila::BinArchive;
use std::collections::HashMap;

pub fn unpack_bin_archive(archive: &BinArchive) -> anyhow::Result<String> {
    let mut pointers: HashMap<usize, usize> = HashMap::new();
    let mut pointer_destinations: HashMap<usize, usize> = HashMap::new();
    for addr in (0..archive.size()).step_by(4) {
        if let Some(ptr) = archive.read_pointer(addr)? {
            let id = if let Some(id) = pointer_destinations.get(&ptr) {
                *id
            } else {
                pointer_destinations.len()
            };
            pointer_destinations.insert(ptr, id);
            pointers.insert(addr, id);
        }
    }

    let mut lines: Vec<String> = Vec::new();
    for addr in (0..archive.size()).step_by(4) {
        if let Some(id) = pointer_destinations.get(&addr) {
            lines.push(format!("DEST: {}", id));
        }
        if let Some(labels) = archive.read_labels(addr)? {
            for label in labels {
                lines.push(format!("LABEL: {}", label));
            }
        }
        if let Some(id) = pointers.get(&addr) {
            if let Some(text) = archive.read_c_string(addr)? {
                if !text.trim().is_empty() {
                    lines.push(format!("SRC: {} // {}", id, text));
                } else {
                    lines.push(format!("SRC: {}", id));
                }
            } else {
                lines.push(format!("SRC: {}", id));
            }
        } else if let Some(text) = archive.read_string(addr)? {
            lines.push(text.to_string());
        } else {
            let data = archive.read_bytes(addr, 4)?;
            lines.push(format!(
                "{:02X}{:02X}{:02X}{:02X}",
                data[0], data[1], data[2], data[3]
            ));
        }
    }
    Ok(lines.join("\n"))
}
