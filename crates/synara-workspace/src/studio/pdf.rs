//! Immutable PDF snapshots. Switching pages never rereads a changing source.
use super::*;
use std::{io::Cursor, sync::Arc};
use synara_runtime::{MAX_PDF_BYTES, MAX_PDF_PAGES, PDF_PAGE_EDGE, PdfTool, run_pdf_tool};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use tokio_util::sync::CancellationToken;

const MAX_PDF_TEXT_BYTES: usize = 1024 * 1024;
const MAX_PDF_DIAGNOSTIC_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct StudioPdf {
    bytes: Arc<[u8]>,
    pub pages: u32,
}
#[derive(Debug)]
pub struct StudioPdfPage {
    pub number: u32,
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
impl StudioPdf {
    pub async fn page(
        &self,
        number: u32,
        cancel: &CancellationToken,
    ) -> WorkspaceResult<StudioPdfPage> {
        if number == 0 || number > self.pages {
            return Err(WorkspaceError::Invalid(
                "Choose a page within this PDF.".into(),
            ));
        }
        let permit = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(RuntimeError::Closed.into()),
            result = tokio::time::timeout(Duration::from_secs(3), PREVIEWS.acquire()) => result
                .map_err(|_| WorkspaceError::Invalid("Another preview is still rendering. Try again.".into()))?
                .map_err(|_| WorkspaceError::Worker)?,
        };
        let png = run_pdf_tool(self.bytes.clone(), PdfTool::Page(number), cancel).await?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let reader =
                image::ImageReader::with_format(Cursor::new(&png), image::ImageFormat::Png);
            let (width, height) = reader.into_dimensions().map_err(|_| {
                WorkspaceError::Invalid("The PDF helper returned an invalid page image.".into())
            })?;
            if width == 0 || height == 0 || width > PDF_PAGE_EDGE || height > PDF_PAGE_EDGE {
                return Err(RuntimeError::Limit.into());
            }
            // Check real decode, not just the header, before handing bytes to GPUI.
            image::load_from_memory_with_format(&png, image::ImageFormat::Png).map_err(|_| {
                WorkspaceError::Invalid("The PDF helper returned a damaged page image.".into())
            })?;
            Ok(StudioPdfPage {
                number,
                png,
                width,
                height,
            })
        })
        .await
        .map_err(|_| WorkspaceError::Worker)?
    }

    /// Extract one page as inert UTF-8 text for reading and copying. The fixed
    /// Poppler helper receives only this immutable PDF snapshot on stdin.
    pub async fn page_text(
        &self,
        number: u32,
        cancel: &CancellationToken,
    ) -> WorkspaceResult<String> {
        if number == 0 || number > self.pages {
            return Err(WorkspaceError::Invalid(
                "Choose a page within this PDF.".into(),
            ));
        }
        let permit = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(RuntimeError::Closed.into()),
            result = tokio::time::timeout(Duration::from_secs(3), PREVIEWS.acquire()) => result
                .map_err(|_| WorkspaceError::Invalid("Another preview is still rendering. Try again.".into()))?
                .map_err(|_| WorkspaceError::Worker)?,
        };
        let output = run_pdf_text_tool(self.bytes.clone(), number, cancel).await?;
        let text = String::from_utf8(output).map_err(|_| {
            WorkspaceError::Invalid("The PDF helper returned invalid UTF-8 text.".into())
        })?;
        drop(permit);
        Ok(text)
    }
}

fn pdf_text_arguments(number: u32) -> WorkspaceResult<Vec<String>> {
    if !(1..=MAX_PDF_PAGES).contains(&number) {
        return Err(WorkspaceError::Invalid(
            "Choose a page within this PDF.".into(),
        ));
    }
    Ok(vec![
        "-f".into(),
        number.to_string(),
        "-l".into(),
        number.to_string(),
        "-layout".into(),
        "-enc".into(),
        "UTF-8".into(),
        "-nopgbrk".into(),
        "-".into(),
        "-".into(),
    ])
}

/// Match the rendering helper's fixed executable, limits, empty environment,
/// stdin-only source and bounded output. `pdftotext` does not interpret the
/// extracted bytes as markup or execute PDF actions.
async fn run_pdf_text_tool(
    bytes: Arc<[u8]>,
    number: u32,
    cancel: &CancellationToken,
) -> WorkspaceResult<Vec<u8>> {
    if cancel.is_cancelled() {
        return Err(RuntimeError::Closed.into());
    }
    if bytes.len() > MAX_PDF_BYTES {
        return Err(RuntimeError::Limit.into());
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err(WorkspaceError::Invalid(
            "The selected file has no PDF header.".into(),
        ));
    }
    let args = pdf_text_arguments(number)?;
    if !cfg!(target_os = "linux")
        || !std::path::Path::new("/usr/bin/pdftotext").is_file()
        || !std::path::Path::new("/usr/bin/prlimit").is_file()
    {
        return Err(RuntimeError::Unsupported("PDF text extraction currently requires Linux with the system poppler-utils and util-linux packages installed. No helper was downloaded or started".into()).into());
    }
    let scratch = tempfile::Builder::new()
        .prefix("synara-pdf-text-")
        .tempdir()
        .map_err(RuntimeError::from)?;
    let mut command = Command::new("/usr/bin/prlimit");
    command.args([
        "--as=805306368",
        "--cpu=10",
        "--fsize=16777216",
        "--nofile=64",
        "--",
        "/usr/bin/pdftotext",
    ]);
    command.args(args);
    command
        .current_dir(scratch.path())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("HOME", scratch.path())
        .env("XDG_CONFIG_HOME", scratch.path())
        .env("XDG_CACHE_HOME", scratch.path())
        .env("TMPDIR", scratch.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.as_std_mut().process_group(0);
    }
    let mut child = command.spawn().map_err(RuntimeError::from)?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| WorkspaceError::Invalid("PDF helper input is unavailable.".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| WorkspaceError::Invalid("PDF helper output is unavailable.".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| WorkspaceError::Invalid("PDF helper diagnostics are unavailable.".into()))?;
    let input = bytes.clone();
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(RuntimeError::Closed.into()),
        result = tokio::time::timeout(Duration::from_secs(15), async {
            let write = async move {
                stdin.write_all(&input).await?;
                stdin.shutdown().await?;
                Ok::<_, RuntimeError>(())
            };
            let wait = async { child.wait().await.map_err(RuntimeError::from) };
            let (_, output, _, status) = tokio::try_join!(
                write,
                bounded_pdf_read(stdout, MAX_PDF_TEXT_BYTES),
                bounded_pdf_read(stderr, MAX_PDF_DIAGNOSTIC_BYTES),
                wait,
            )?;
            if !status.success() {
                return Err(RuntimeError::Invalid("The PDF text could not be extracted. It may be encrypted, damaged or beyond the helper limits. The source file was not modified".into()));
            }
            Ok(output)
        }) => match result {
            Ok(result) => result.map_err(WorkspaceError::from),
            Err(_) => Err(RuntimeError::Timeout.into()),
        },
    };
    if result.is_err() {
        let _ = child.start_kill();
        let _ = tokio::time::timeout(Duration::from_secs(1), child.wait()).await;
    }
    result
}

async fn bounded_pdf_read(
    reader: impl AsyncRead + Unpin,
    limit: usize,
) -> Result<Vec<u8>, RuntimeError> {
    let mut bytes = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .await?;
    if bytes.len() > limit {
        return Err(RuntimeError::Limit);
    }
    Ok(bytes)
}
fn page_count(info: &[u8]) -> WorkspaceResult<u32> {
    let text = std::str::from_utf8(info)
        .map_err(|_| WorkspaceError::Invalid("Invalid PDF metadata.".into()))?;
    let mut values = text.lines().filter_map(|line| line.strip_prefix("Pages:"));
    let pages = values
        .next()
        .and_then(|value| value.trim().parse::<u32>().ok());
    match pages {
        Some(n) if (1..=MAX_PDF_PAGES).contains(&n) && values.next().is_none() => Ok(n),
        _ => Err(WorkspaceError::Invalid(format!(
            "The PDF page count is invalid or exceeds {MAX_PDF_PAGES} pages."
        ))),
    }
}
impl WorkspaceService {
    pub async fn studio_pdf(
        &self,
        task: TaskId,
        path: PathBuf,
        cancel: &CancellationToken,
    ) -> WorkspaceResult<StudioPdf> {
        let root = self.local_studio_root(task).await?;
        if !visible(&path) {
            return Err(
                RuntimeError::Denied("This path is not a visible Library file.".into()).into(),
            );
        }
        if cancel.is_cancelled() {
            return Err(RuntimeError::Closed.into());
        }
        let bytes: Arc<[u8]> =
            tokio::task::spawn_blocking(move || WorkspaceFs::open(&root)?.read_blob(&path))
                .await
                .map_err(|_| WorkspaceError::Worker)??
                .into();
        let _permit = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(RuntimeError::Closed.into()),
            result = tokio::time::timeout(Duration::from_secs(3), PREVIEWS.acquire()) => result
                .map_err(|_| WorkspaceError::Invalid("Another preview is still rendering. Try again.".into()))?
                .map_err(|_| WorkspaceError::Worker)?,
        };
        let info = run_pdf_tool(bytes.clone(), PdfTool::Information, cancel).await?;
        Ok(StudioPdf {
            bytes,
            pages: page_count(&info)?,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_metadata_refuses_ambiguous_unbounded_and_injected_values() {
        assert_eq!(
            page_count(b"Title: report\nPages: 2\nEncrypted: no\n").unwrap(),
            2
        );
        for bad in [
            "Pages: 0",
            "Pages: 2001",
            "Pages: -1",
            "Pages: 2\nPages: 1",
            "Pages: 2 trailing",
            "Title: no count",
        ] {
            assert!(page_count(bad.as_bytes()).is_err(), "{bad}");
        }
    }
    #[test]
    fn text_extraction_arguments_are_single_page_fixed_and_bounded() {
        assert!(pdf_text_arguments(0).is_err());
        assert!(pdf_text_arguments(MAX_PDF_PAGES + 1).is_err());
        assert_eq!(
            pdf_text_arguments(7).unwrap(),
            [
                "-f",
                "7",
                "-l",
                "7",
                "-layout",
                "-enc",
                "UTF-8",
                "-nopgbrk",
                "-",
                "-",
            ]
        );
    }
    #[tokio::test]
    async fn extracted_text_output_stops_at_its_byte_limit() {
        assert_eq!(bounded_pdf_read(&b"text"[..], 4).await.unwrap(), b"text");
        assert!(matches!(
            bounded_pdf_read(&b"large"[..], 4).await,
            Err(RuntimeError::Limit)
        ));
    }
    #[tokio::test]
    async fn page_text_rejects_invalid_or_cancelled_requests_before_helper_spawn() {
        let pdf = StudioPdf {
            bytes: Arc::from(&b"%PDF-1.4"[..]),
            pages: 1,
        };
        let cancel = CancellationToken::new();
        assert!(pdf.page_text(0, &cancel).await.is_err());
        cancel.cancel();
        assert!(pdf.page_text(1, &cancel).await.is_err());
    }
    #[tokio::test]
    #[ignore = "requires Linux system Poppler and prlimit; run explicitly in PDF validation"]
    async fn real_pdf_pages_are_bounded_snapshot_owned_and_preserve_source() {
        let dir = tempfile::tempdir().unwrap();
        let source = include_bytes!("../../../../scripts/fixtures/parity-document.pdf");
        std::fs::write(dir.path().join("report.pdf"), source).unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let task = service
            .create_scoped_task(project.id, "PDF".into(), agent, TaskScope::Studio)
            .await
            .unwrap();
        let cancel = CancellationToken::new();
        let doc = service
            .studio_pdf(task.id, "report.pdf".into(), &cancel)
            .await
            .unwrap();
        assert_eq!(doc.pages, 2);
        assert_eq!(doc.page_text(1, &cancel).await.unwrap(), "First PDF page\n");
        assert!(doc.page_text(3, &cancel).await.is_err());
        let first = doc.page(1, &cancel).await.unwrap();
        assert!(first.width <= PDF_PAGE_EDGE && first.height <= PDF_PAGE_EDGE);
        assert_eq!(
            std::fs::read(dir.path().join("report.pdf")).unwrap(),
            source
        );
        std::fs::write(dir.path().join("report.pdf"), b"newer invalid source").unwrap();
        let second = doc.page(2, &cancel).await.unwrap();
        assert_ne!(first.png, second.png);
        assert!(doc.page(3, &cancel).await.is_err());
        assert!(
            service
                .studio_pdf(task.id, "report.pdf".into(), &cancel)
                .await
                .is_err()
        );
        assert!(
            service
                .studio_pdf(task.id, "../report.pdf".into(), &cancel)
                .await
                .is_err()
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("report.pdf", dir.path().join("link.pdf")).unwrap();
            assert!(
                service
                    .studio_pdf(task.id, "link.pdf".into(), &cancel)
                    .await
                    .is_err()
            );
        }
        cancel.cancel();
        assert!(doc.page(1, &cancel).await.is_err());
    }
}
