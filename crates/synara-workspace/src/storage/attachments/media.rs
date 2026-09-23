//! Transcript originals use the ordered event store, not the evictable Recent shelf.
use super::*;
use std::io::Cursor;
use synara_core::{EventId, ThreadEvent, TranscriptImage};

fn decode_image(image: &TranscriptImage) -> WorkspaceResult<Vec<u8>> {
    if !image.bounded() {
        return Err(invalid("Image exceeds the 2 MiB limit or is not PNG/JPEG."));
    }
    let mut result = Vec::with_capacity(image.base64.len() / 4 * 3);
    fn value(byte: u8) -> Option<u8> {
        Some(match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        })
    }
    for (index, chunk) in image
        .base64
        .as_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
    {
        let final_chunk = index + 1 == image.base64.len() / 4;
        let a = value(chunk[0]).ok_or_else(|| invalid("Malformed image encoding."))?;
        let b = value(chunk[1]).ok_or_else(|| invalid("Malformed image encoding."))?;
        let c = if chunk[2] == b'=' && final_chunk {
            0
        } else {
            value(chunk[2]).ok_or_else(|| invalid("Malformed image encoding."))?
        };
        let d = if chunk[3] == b'=' && final_chunk {
            0
        } else {
            value(chunk[3]).ok_or_else(|| invalid("Malformed image encoding."))?
        };
        if chunk[2] == b'=' && chunk[3] != b'=' {
            return Err(invalid("Malformed image padding."));
        }
        result.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            result.push((b << 4) | (c >> 2));
        }
        if chunk[3] != b'=' {
            result.push((c << 6) | d);
        }
    }
    if result.len() > MAX_ATTACHMENT_BATCH_BYTES || intake::base64(&result) != image.base64 {
        return Err(invalid("Non-canonical image encoding."));
    }
    Ok(result)
}
fn checked(image: &TranscriptImage) -> WorkspaceResult<(AttachmentInfo, Vec<u8>)> {
    let bytes = decode_image(image)?;
    let mut info = intake::inspect(
        if image.mime_type == "image/png" {
            "image.png"
        } else {
            "image.jpg"
        }
        .into(),
        &bytes,
    )?;
    if info.kind.mime_type() != image.mime_type {
        return Err(invalid("Image content does not match its declared type."));
    }
    info.source = image.source;
    Ok((info, bytes))
}
impl WorkspaceService {
    pub(crate) async fn validate_transcript_image(image: TranscriptImage) -> WorkspaceResult<()> {
        intake::run(move || checked(&image).map(|_| ())).await
    }
    async fn transcript_image(
        &self,
        task: TaskId,
        id: EventId,
    ) -> WorkspaceResult<TranscriptImage> {
        self.access(move |store| {
            let task = store.task(task)?.ok_or(WorkspaceError::NotFound)?;
            let raw: Option<String> = store
                .connection
                .query_row(
                    "SELECT data FROM events WHERE id=?1 AND thread_id=?2",
                    params![id.to_string(), task.thread_id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(StorageError::from)?;
            match decode::<ThreadEvent>(&raw.ok_or(WorkspaceError::NotFound)?)? {
                ThreadEvent::ImageMessage { image, .. } => Ok(image),
                _ => Err(WorkspaceError::NotFound),
            }
        })
        .await
    }
    pub async fn transcript_image_preview(
        &self,
        task: TaskId,
        id: EventId,
        expanded: bool,
    ) -> WorkspaceResult<AttachmentPreview> {
        let image = self.transcript_image(task, id).await?;
        intake::run(move || {
            let (mut info, bytes) = checked(&image)?;
            // Reuse strict header/decode bounds before creating a small renderer asset.
            let decoded =
                image::load_from_memory(&bytes).map_err(|_| invalid("Image decode failed."))?;
            let preview = if expanded {
                decoded.thumbnail(1600, 1200)
            } else {
                decoded.thumbnail(480, 320)
            };
            let mut output = Cursor::new(Vec::new());
            preview
                .write_to(&mut output, image::ImageFormat::Png)
                .map_err(|_| invalid("Preview encoding failed."))?;
            info.kind = AttachmentKind::Png;
            info.dimensions = Some((preview.width(), preview.height()));
            info.bytes = output.get_ref().len();
            info.id = id.to_string();
            Ok(AttachmentPreview {
                info,
                bytes: output.into_inner(),
            })
        })
        .await
    }
    pub async fn export_transcript_image(
        &self,
        task: TaskId,
        id: EventId,
        destination: PathBuf,
    ) -> WorkspaceResult<()> {
        let image = self.transcript_image(task, id).await?;
        intake::run(move || {
            let (_, bytes) = checked(&image)?;
            super::super::conversation_tools::write_new_export(&destination, &bytes)?;
            Ok(())
        })
        .await
    }
}
#[cfg(test)]
mod tests;
