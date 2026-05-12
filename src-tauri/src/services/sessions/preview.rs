use std::path::Path;

const TRANSCRIPT_PREVIEW_BYTES: usize = 320;
const TRANSCRIPT_PREVIEW_CHARS: usize = 180;

pub(super) fn read_transcript_preview(path: &str) -> Option<String> {
    let p = Path::new(path);
    if !p.exists() {
        return None;
    }
    // Read only the first N bytes so a very long transcript does not slow down
    // every session reload.
    use std::io::Read;
    let mut f = std::fs::File::open(p).ok()?;
    let mut buf = vec![0u8; TRANSCRIPT_PREVIEW_BYTES];
    let n = f.read(&mut buf).ok()?;
    buf.truncate(n);
    let text = String::from_utf8_lossy(&buf);
    let stripped = text
        .trim_start()
        .lines()
        .map(strip_leading_markers)
        .collect::<Vec<_>>()
        .join(" ");
    let compacted = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
    if compacted.is_empty() {
        return None;
    }
    let truncated: String = compacted.chars().take(TRANSCRIPT_PREVIEW_CHARS).collect();
    if compacted.chars().count() > TRANSCRIPT_PREVIEW_CHARS {
        Some(format!("{truncated}…"))
    } else {
        Some(truncated)
    }
}

fn strip_leading_markers(line: &str) -> String {
    let mut s = line.trim_start().to_string();
    for _ in 0..4 {
        if let Some(rest) = s.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                s = rest[end + 1..].trim_start().to_string();
                continue;
            }
        }
        break;
    }
    if let Some(pos) = s.find(':') {
        if pos <= 20 {
            s = s[pos + 1..].trim_start().to_string();
        }
    }
    s
}
