//! Opt-in local PDF helpers. Only fixed system executables receive the bounded
//! snapshot on stdin, never a document-controlled path or shell expression.
//! Process/CPU/address-space limits are not an operating-system security sandbox.
use crate::{RuntimeError, process::spawn_owned};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use tokio_util::sync::CancellationToken;

pub const MAX_PDF_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_PDF_PAGES: u32 = 2000;
pub const PDF_PAGE_EDGE: u32 = 1600;

#[derive(Clone, Copy, Debug)]
pub enum PdfTool {
    Information,
    Page(u32),
}
impl PdfTool {
    fn arguments(self) -> Result<(&'static str, Vec<String>, usize), RuntimeError> {
        match self {
            Self::Information => Ok((
                "/usr/bin/pdfinfo",
                vec!["-f".into(), "1".into(), "-l".into(), "1".into(), "-".into()],
                64 * 1024,
            )),
            Self::Page(page) if (1..=MAX_PDF_PAGES).contains(&page) => Ok((
                "/usr/bin/pdftoppm",
                vec![
                    "-f".into(),
                    page.to_string(),
                    "-l".into(),
                    page.to_string(),
                    "-singlefile".into(),
                    "-scale-to".into(),
                    PDF_PAGE_EDGE.to_string(),
                    "-png".into(),
                    "-".into(),
                ],
                12 * 1024 * 1024,
            )),
            Self::Page(_) => Err(RuntimeError::Invalid("PDF page is out of range".into())),
        }
    }
}

/// Linux currently requires Poppler utilities and util-linux in /usr/bin. We do
/// not discover executables on PATH, install software or fall back to a browser.
pub async fn run_pdf_tool(
    bytes: Arc<[u8]>,
    tool: PdfTool,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, RuntimeError> {
    if cancel.is_cancelled() {
        return Err(RuntimeError::Closed);
    }
    if bytes.len() > MAX_PDF_BYTES {
        return Err(RuntimeError::Limit);
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err(RuntimeError::Invalid(
            "The selected file has no PDF header".into(),
        ));
    }
    let (executable, args, limit) = tool.arguments()?;
    if !cfg!(target_os = "linux")
        || !Path::new(executable).is_file()
        || !Path::new("/usr/bin/prlimit").is_file()
    {
        return Err(RuntimeError::Unsupported("PDF viewing currently requires Linux with the system poppler-utils and util-linux packages installed. No helper was downloaded or started".into()));
    }
    let scratch = tempfile::Builder::new().prefix("synara-pdf-").tempdir()?;
    let mut command = Command::new("/usr/bin/prlimit");
    command.args([
        "--as=805306368",
        "--cpu=10",
        "--fsize=16777216",
        "--nofile=64",
        "--",
        executable,
    ]);
    command
        .args(args)
        .current_dir(scratch.path())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("HOME", scratch.path())
        .env("XDG_CONFIG_HOME", scratch.path())
        .env("XDG_CACHE_HOME", scratch.path())
        .env("TMPDIR", scratch.path());
    let process = spawn_owned(command)?;
    let handle = process.handle;
    // The only handle lives in this future. Dropping it cancels the process tree.
    let mut stdin = process.stdin;
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(RuntimeError::Closed),
        result = tokio::time::timeout(Duration::from_secs(15), async {
            let input = async {
                stdin.write_all(&bytes).await?;
                stdin.shutdown().await?;
                drop(stdin); // Ensure EOF before waiting for helpers that consume all input.
                Ok::<_, RuntimeError>(())
            };
            let (_, output, _, status) = tokio::try_join!(
                input, bounded_read(process.stdout, limit),
                bounded_read(process.stderr, 64 * 1024), handle.wait()
            )?;
            if !status.success() {
                return Err(RuntimeError::Invalid("The PDF could not be rendered. It may be encrypted, damaged or beyond the renderer limits. The source file was not modified".into()));
            }
            Ok(output)
        }) => result.unwrap_or(Err(RuntimeError::Timeout)),
    };
    if result.is_err() {
        let _ = handle.shutdown().await;
    }
    result
}

async fn bounded_read(
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_arguments_are_single_page_stdin_only_and_bounded() {
        for n in [0, MAX_PDF_PAGES + 1] {
            assert!(PdfTool::Page(n).arguments().is_err());
        }
        let (exe, args, limit) = PdfTool::Page(17).arguments().unwrap();
        assert_eq!(exe, "/usr/bin/pdftoppm");
        assert_eq!(
            args,
            [
                "-f",
                "17",
                "-l",
                "17",
                "-singlefile",
                "-scale-to",
                "1600",
                "-png",
                "-"
            ]
        );
        assert_eq!(limit, 12 * 1024 * 1024);
    }
    #[tokio::test]
    async fn cancellation_and_invalid_input_refuse_before_helper_spawn() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(matches!(
            run_pdf_tool(Arc::from(&b"%PDF-1.4"[..]), PdfTool::Information, &cancel).await,
            Err(RuntimeError::Closed)
        ));
        assert!(
            run_pdf_tool(
                Arc::from(&b"not a PDF"[..]),
                PdfTool::Information,
                &CancellationToken::new()
            )
            .await
            .is_err()
        );
    }
    #[tokio::test]
    async fn renderer_output_cannot_exceed_the_reader_bound() {
        assert_eq!(bounded_read(&b"1234"[..], 4).await.unwrap(), b"1234");
        assert!(matches!(
            bounded_read(&b"12345"[..], 4).await,
            Err(RuntimeError::Limit)
        ));
    }
}
