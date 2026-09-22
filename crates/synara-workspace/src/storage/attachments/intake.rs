//! Explicit local file/clipboard input. No network fetch, directory walk or source mutation.
use super::*;
use image::{ImageFormat, ImageReader, Limits};
use std::{io::Cursor, path::Path, time::Duration};
static DECODER: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

pub(super) async fn prepare(inputs: Vec<AttachmentInput>) -> WorkspaceResult<Vec<StoredAttachment>> {
    if inputs.is_empty() || inputs.len() > MAX_ATTACHMENTS { return Err(invalid("Choose between one and eight files.")); }
    run(move || {
        let mut result = Vec::new(); let mut total = 0usize;
        for input in inputs {
            let source = match &input { AttachmentInput::Capture { source, .. } if source.is_capture() => *source,
                AttachmentInput::Capture { .. } => return Err(invalid("Invalid capture provenance.")), _ => synara_core::ImageSource::Uploaded };
            let (name, bytes) = match input {
                AttachmentInput::Bytes { name, bytes } | AttachmentInput::Capture { name, bytes, .. } => (name,bytes),
                AttachmentInput::File(path) => {
                    let raw = path.to_str().ok_or_else(||invalid("Choose a file with a UTF-8 path."))?;
                    if !path.is_absolute() || raw.len() > 8192 || raw.starts_with("//") || raw.starts_with("\\\\") || raw.chars().any(char::is_control) {
                        return Err(invalid("Choose local files, not URLs or network shares."));
                    }
                    let parent = path.parent().ok_or(WorkspaceError::NotFound)?;
                    let leaf = path.file_name().ok_or(WorkspaceError::NotFound)?;
                    let fs = synara_runtime::WorkspaceFs::open(parent)?;
                    if fs.file_length(Path::new(leaf))? > MAX_ATTACHMENT_BATCH_BYTES as u64 { return Err(invalid("A file exceeds 2 MiB. Nothing was attached.")); }
                    (leaf.to_string_lossy().into_owned(),fs.read_blob(Path::new(leaf))?)
                }
            };
            total = total.saturating_add(bytes.len());
            if total > MAX_ATTACHMENT_BATCH_BYTES { return Err(invalid("The selected files exceed 2 MiB combined. Nothing was attached.")); }
            let mut info = inspect(name,&bytes)?;
            if source.is_capture() && !info.kind.is_image() { return Err(invalid("Capture output is not an image.")); }
            info.source = source;
            result.push(StoredAttachment { info, hex: hex::encode(bytes) });
        }
        Ok(result)
    }).await
}
pub(super) async fn run<T: Send + 'static>(work: impl FnOnce() -> WorkspaceResult<T> + Send + 'static) -> WorkspaceResult<T> {
    let permit = tokio::time::timeout(Duration::from_secs(10), DECODER.acquire()).await
        .map_err(|_|invalid("Attachment reader is busy. Try again."))?.map_err(|_|WorkspaceError::Worker)?;
    tokio::task::spawn_blocking(move || { let _permit = permit; work() })
        .await.map_err(|_|WorkspaceError::Worker)?
}
pub(super) fn inspect(name: String, bytes: &[u8]) -> WorkspaceResult<AttachmentInfo> {
    if !valid_name(&name) || bytes.is_empty() || bytes.len() > MAX_ATTACHMENT_BATCH_BYTES {
        return Err(invalid("An attachment has an invalid name, is empty or exceeds 2 MiB."));
    }
    let (kind,dimensions) = if let Some((format,w,h)) = crate::studio::image_size(bytes) {
        if w == 0 || h == 0 || w > 8192 || h > 8192 || u64::from(w) * u64::from(h) > 16_000_000 {
            return Err(invalid("Image dimensions exceed 8192 per side or 16 megapixels."));
        }
        let kind = match format { crate::PreviewImageFormat::Png => AttachmentKind::Png, crate::PreviewImageFormat::Jpeg => AttachmentKind::Jpeg };
        if kind == AttachmentKind::Png { still_png(bytes)?; }
        let mut reader = ImageReader::with_format(Cursor::new(bytes),if kind == AttachmentKind::Png { ImageFormat::Png } else { ImageFormat::Jpeg });
        let mut limits = Limits::default(); limits.max_image_width = Some(w); limits.max_image_height = Some(h); limits.max_alloc = Some(128*1024*1024);
        reader.limits(limits);
        let decoded = reader.decode().map_err(|_|invalid("The image is damaged or exceeds decoder limits."))?;
        if decoded.width() != w || decoded.height() != h { return Err(StorageError::Identity.into()); }
        (kind,Some((w,h)))
    } else {
        let extension = Path::new(&name).extension().and_then(|x|x.to_str()).unwrap_or("").to_ascii_lowercase();
        if matches!(extension.as_str(),"png"|"jpg"|"jpeg"|"gif"|"webp"|"pdf"|"zip"|"mp4"|"mp3"|"wav"|"docx")
            || bytes.contains(&0) || std::str::from_utf8(bytes).is_err() {
            return Err(invalid("Only still PNG/JPEG images and UTF-8 text/code files are supported. Binary files were not attached."));
        }
        (AttachmentKind::Text,None)
    };
    Ok(AttachmentInfo { id: uuid::Uuid::new_v4().to_string(),name,kind,bytes:bytes.len(),dimensions,source:synara_core::ImageSource::Uploaded })
}
fn still_png(bytes: &[u8]) -> WorkspaceResult<()> {
    let mut at=8usize;
    while at.checked_add(12).is_some_and(|end|end <= bytes.len()) {
        let n=u32::from_be_bytes(bytes[at..at+4].try_into().map_err(|_|StorageError::Identity)?) as usize;
        if &bytes[at+4..at+8] == b"acTL" { return Err(invalid("Animated PNG is unsupported. Choose a still image.")); }
        let next=at.checked_add(12).and_then(|x|x.checked_add(n)).filter(|x|*x <= bytes.len()).ok_or(StorageError::Identity)?;
        if &bytes[at+4..at+8] == b"IEND" { return Ok(()); }
        at=next;
    }
    Err(invalid("Truncated PNG attachment."))
}
/// RFC 4648 standard alphabet, with padding. No dependency change is needed.
pub(super) fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8;64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out=String::with_capacity(bytes.len().div_ceil(3)*4);
    for chunk in bytes.chunks(3) {
        let a=chunk[0];let b=chunk.get(1).copied().unwrap_or(0);let c=chunk.get(2).copied().unwrap_or(0);
        out.push(ALPHABET[(a>>2) as usize] as char);
        out.push(ALPHABET[(((a&3)<<4)|(b>>4)) as usize] as char);
        out.push(if chunk.len()>1 {ALPHABET[(((b&15)<<2)|(c>>6)) as usize] as char} else {'='});
        out.push(if chunk.len()>2 {ALPHABET[(c&63) as usize] as char} else {'='});
    }
    out
}
