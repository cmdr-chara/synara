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
const MAX_PDF_LINKS: usize = 128;

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
    /// Read a bounded, labeled text selection from this immutable snapshot.
    /// The caller's cancellation token also stops work after navigation.
    pub async fn first_pages_text(&self, cancel: &CancellationToken) -> WorkspaceResult<String> {
        extract_first_pages_text(self.bytes.clone(), self.pages, cancel).await
    }

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

    /// Inspect Poppler's external web link annotations on one page.
    /// Unsupported actions and schemes are omitted; URLs are never fetched.
    pub async fn page_links(
        &self,
        number: u32,
        cancel: &CancellationToken,
    ) -> WorkspaceResult<Vec<String>> {
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
        let output = run_pdf_link_tool(self.bytes.clone(), number, cancel).await?;
        let links = parse_pdf_links(&output, number)?;
        drop(permit);
        Ok(links)
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

fn pdf_link_arguments(number: u32) -> WorkspaceResult<Vec<String>> {
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
        "-url".into(),
        "-".into(),
    ])
}

fn parse_pdf_links(output: &[u8], requested_page: u32) -> WorkspaceResult<Vec<String>> {
    let text = std::str::from_utf8(output)
        .map_err(|_| WorkspaceError::Invalid("Invalid PDF link metadata.".into()))?;
    let mut lines = text.lines();
    if lines.next().is_none_or(|header| {
        header.split_whitespace().collect::<Vec<_>>() != ["Page", "Type", "URL"]
    }) {
        return Err(WorkspaceError::Invalid(
            "The PDF helper returned unrecognized link metadata.".into(),
        ));
    }

    let mut links = Vec::new();
    for line in lines {
        let mut fields = line.split_whitespace();
        let Some(page) = fields.next() else {
            continue;
        };
        let page = page.parse::<u32>().map_err(|_| {
            WorkspaceError::Invalid("The PDF helper returned invalid link metadata.".into())
        })?;
        let Some(kind) = fields.next() else {
            return Err(WorkspaceError::Invalid(
                "The PDF helper returned invalid link metadata.".into(),
            ));
        };
        let url = fields.collect::<Vec<_>>().join(" ");
        if page != requested_page || kind != "Annotation" || url.is_empty() {
            continue;
        }
        // Treat PDF-provided destinations as inert strings and only surface
        // regular HTTP(S) navigation. Reject credentials and malformed URLs
        // with the same validator used for other user-visible web links.
        if synara_agent::validate_web_url(&url).is_err() || links.contains(&url) {
            continue;
        }
        if links.len() == MAX_PDF_LINKS {
            return Err(RuntimeError::Limit.into());
        }
        links.push(url);
    }
    Ok(links)
}

/// Match the rendering helper's fixed executable, limits, empty environment,
/// stdin-only source and bounded output. These Poppler calls return inert data;
/// they do not execute PDF actions.
async fn run_pdf_text_tool(
    bytes: Arc<[u8]>,
    number: u32,
    cancel: &CancellationToken,
) -> WorkspaceResult<Vec<u8>> {
    let args = pdf_text_arguments(number)?;
    run_pdf_helper(
        bytes,
        "/usr/bin/pdftotext",
        args,
        MAX_PDF_TEXT_BYTES,
        "PDF text extraction",
        cancel,
    )
    .await
}

async fn run_pdf_link_tool(
    bytes: Arc<[u8]>,
    number: u32,
    cancel: &CancellationToken,
) -> WorkspaceResult<Vec<u8>> {
    let args = pdf_link_arguments(number)?;
    run_pdf_helper(
        bytes,
        "/usr/bin/pdfinfo",
        args,
        MAX_PDF_DIAGNOSTIC_BYTES,
        "PDF link inspection",
        cancel,
    )
    .await
}

/// Run a fixed Poppler executable with a fixed-argument builder, the immutable
/// PDF snapshot on stdin, and bounded output. The operation name and executable
/// come only from the private wrappers above, never from PDF content.
async fn run_pdf_helper(
    bytes: Arc<[u8]>,
    executable: &'static str,
    args: Vec<String>,
    output_limit: usize,
    operation: &'static str,
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
    if !cfg!(target_os = "linux")
        || !std::path::Path::new(executable).is_file()
        || !std::path::Path::new("/usr/bin/prlimit").is_file()
    {
        return Err(RuntimeError::Unsupported(format!("{operation} currently requires Linux with the system poppler-utils and util-linux packages installed. No helper was downloaded or started")).into());
    }
    let scratch = tempfile::Builder::new()
        .prefix("synara-pdf-helper-")
        .tempdir()
        .map_err(RuntimeError::from)?;
    let mut command = Command::new("/usr/bin/prlimit");
    command.args([
        "--as=805306368",
        "--cpu=10",
        "--fsize=16777216",
        "--nofile=64",
        "--",
        executable,
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
                bounded_pdf_read(stdout, output_limit),
                bounded_pdf_read(stderr, MAX_PDF_DIAGNOSTIC_BYTES),
                wait,
            )?;
            if !status.success() {
                return Err(RuntimeError::Invalid(format!("{operation} failed. The PDF may be encrypted, damaged or beyond the helper limits. The source file was not modified")));
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

/// Extract bounded text from a user-selected attachment snapshot. Each page is
/// labeled so the agent can cite its origin; pages beyond the preview window
/// are explicitly disclosed rather than silently disappearing.
pub async fn attachment_text(bytes: Vec<u8>) -> WorkspaceResult<String> {
    let bytes: Arc<[u8]> = bytes.into();
    let cancel = CancellationToken::new();
    let pages = page_count(&run_pdf_tool(bytes.clone(), PdfTool::Information, &cancel).await?)?;
    extract_first_pages_text(bytes, pages, &cancel).await
}

async fn extract_first_pages_text(
    bytes: Arc<[u8]>,
    pages: u32,
    cancel: &CancellationToken,
) -> WorkspaceResult<String> {
    let mut text = String::new();
    let mut has_text = false;
    for number in 1..=pages.min(12) {
        let page = run_pdf_text_tool(bytes.clone(), number, cancel).await?;
        let page = std::str::from_utf8(&page).map_err(|_| {
            WorkspaceError::Invalid("The PDF helper returned invalid UTF-8 text.".into())
        })?;
        has_text |= !page.trim().is_empty();
        let header = format!("\n[PDF page {number}]\n");
        if text
            .len()
            .saturating_add(header.len())
            .saturating_add(page.len())
            > 512 * 1024
        {
            return Err(WorkspaceError::Invalid(
                "PDF text exceeds the 512 KiB extraction limit. Choose a smaller document.".into(),
            ));
        }
        text.push_str(&header);
        text.push_str(page);
    }
    if !has_text {
        return Err(WorkspaceError::Invalid(
            "This PDF has no extractable text in its first 12 pages.".into(),
        ));
    }
    if pages > 12 {
        text.push_str(&format!(
            "\n[Only the first 12 of {pages} PDF pages were included.]"
        ));
    }
    Ok(text)
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
                "-f", "7", "-l", "7", "-layout", "-enc", "UTF-8", "-nopgbrk", "-", "-",
            ]
        );
    }

    #[test]
    fn link_inspection_arguments_are_single_page_fixed_and_bounded() {
        assert!(pdf_link_arguments(0).is_err());
        assert!(pdf_link_arguments(MAX_PDF_PAGES + 1).is_err());
        assert_eq!(
            pdf_link_arguments(7).unwrap(),
            ["-f", "7", "-l", "7", "-url", "-"]
        );
    }

    #[test]
    fn link_metadata_exposes_only_supported_web_annotations_for_the_requested_page() {
        let output = b"Page  Type          URL\n   1  Annotation    https://example.com/report?q=private\n   1  Annotation    javascript:alert(1)\n   1  Annotation    file:///etc/passwd\n   1  Link           https://ignored.example/\n   2  Annotation    https://second.example/\n";
        assert_eq!(
            parse_pdf_links(output, 1).unwrap(),
            ["https://example.com/report?q=private"]
        );
        assert!(parse_pdf_links(output, 2).unwrap().is_empty());
        assert!(parse_pdf_links(b"not pdfinfo output", 1).is_err());
    }

    #[test]
    fn link_metadata_deduplicates_and_caps_page_links() {
        let duplicate = b"Page Type URL\n1 Annotation https://example.com/\n1 Annotation https://example.com/\n";
        assert_eq!(parse_pdf_links(duplicate, 1).unwrap().len(), 1);

        let mut too_many = String::from("Page Type URL\n");
        for index in 0..=MAX_PDF_LINKS {
            too_many.push_str(&format!("1 Annotation https://example{index}.com/\n"));
        }
        assert!(matches!(
            parse_pdf_links(too_many.as_bytes(), 1),
            Err(WorkspaceError::Runtime(RuntimeError::Limit))
        ));
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
