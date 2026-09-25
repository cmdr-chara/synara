//! Immutable PDF snapshots. Switching pages never rereads a changing source.
use super::*;
use std::{collections::HashSet, io::Cursor, sync::Arc};
use synara_runtime::{MAX_PDF_BYTES, MAX_PDF_PAGES, PDF_PAGE_EDGE, PdfTool, run_pdf_tool};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use tokio_util::sync::CancellationToken;

const MAX_PDF_TEXT_BYTES: usize = 1024 * 1024;
const MAX_PDF_DIAGNOSTIC_BYTES: usize = 64 * 1024;
const MAX_PDF_LINKS: usize = 128;
const MAX_PDF_FORM_FIELDS: usize = 128;
const MAX_PDF_FORM_OPTIONS: usize = 64;
const MAX_PDF_FORM_VALUE_BYTES: usize = 8 * 1024;
const MAX_PDF_FORM_METADATA_BYTES: usize = 256 * 1024;
const MAX_PDF_FILLED_BYTES: usize = 16 * 1024 * 1024;
const MAX_PDF_OCR_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfFormKind {
    None,
    AcroForm,
    Xfa,
    Unknown,
}
impl PdfFormKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No interactive form reported",
            Self::AcroForm => "AcroForm detected",
            Self::Xfa => "XFA form detected",
            Self::Unknown => "Unrecognized PDF form metadata",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfFormFieldType {
    Text,
    Button,
    Choice,
    Signature,
    Unknown,
}
impl PdfFormFieldType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Button => "Button",
            Self::Choice => "Choice",
            Self::Signature => "Signature",
            Self::Unknown => "Unsupported",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfFormField {
    pub name: String,
    pub alternate_name: Option<String>,
    pub kind: PdfFormFieldType,
    pub value: String,
    pub default_value: Option<String>,
    pub flags: u32,
    pub options: Vec<String>,
}
impl PdfFormField {
    pub fn read_only(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn required(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn editable(&self) -> bool {
        if self.read_only() {
            return false;
        }
        match self.kind {
            PdfFormFieldType::Text => {
                const PASSWORD: u32 = 1 << 13;
                const FILE_SELECT: u32 = 1 << 20;
                const COMB: u32 = 1 << 24;
                const RICH_TEXT: u32 = 1 << 25;
                self.flags & (PASSWORD | FILE_SELECT | COMB | RICH_TEXT) == 0
            }
            PdfFormFieldType::Button => {
                const PUSHBUTTON: u32 = 1 << 16;
                self.flags & PUSHBUTTON == 0 && !self.options.is_empty()
            }
            PdfFormFieldType::Choice => {
                const EDITABLE_COMBO: u32 = 1 << 18;
                const MULTI_SELECT: u32 = 1 << 21;
                self.flags & (EDITABLE_COMBO | MULTI_SELECT) == 0 && !self.options.is_empty()
            }
            PdfFormFieldType::Signature | PdfFormFieldType::Unknown => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfFormEdit {
    pub name: String,
    pub value: String,
}

#[derive(Clone)]
pub struct StudioPdf {
    bytes: Arc<[u8]>,
    pub pages: u32,
    pub form: PdfFormKind,
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

    /// Inspect AcroForm fields without executing document actions or scripts.
    pub async fn form_fields(
        &self,
        cancel: &CancellationToken,
    ) -> WorkspaceResult<Vec<PdfFormField>> {
        match self.form {
            PdfFormKind::None => return Ok(Vec::new()),
            PdfFormKind::Xfa => {
                return Err(RuntimeError::Unsupported(
                    "XFA form fields are not inspected or executed.".into(),
                )
                .into());
            }
            PdfFormKind::Unknown => {
                return Err(RuntimeError::Unsupported(
                    "This PDF reports an unsupported form technology.".into(),
                )
                .into());
            }
            PdfFormKind::AcroForm => {}
        }
        let permit = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(RuntimeError::Closed.into()),
            result = tokio::time::timeout(Duration::from_secs(3), PREVIEWS.acquire()) => result
                .map_err(|_| WorkspaceError::Invalid("Another preview is still rendering. Try again.".into()))?
                .map_err(|_| WorkspaceError::Worker)?,
        };
        let output = run_pdf_form_dump(self.bytes.clone(), cancel).await?;
        let fields = parse_pdf_form_fields(&output)?;
        drop(permit);
        Ok(fields)
    }

    /// OCR one rendered page when the fixed system Tesseract executable exists.
    /// OCR is explicit and never replaces ordinary text extraction silently.
    pub async fn page_ocr_text(
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
        let png = run_pdf_tool(self.bytes.clone(), PdfTool::Page(number), cancel).await?;
        let output = run_pdf_ocr_tool(png.into(), cancel).await?;
        let text = String::from_utf8(output).map_err(|_| {
            WorkspaceError::Invalid("The OCR helper returned invalid UTF-8.".into())
        })?;
        drop(permit);
        if text.trim().is_empty() {
            return Err(WorkspaceError::Invalid(
                "OCR did not find readable text on this page.".into(),
            ));
        }
        Ok(text)
    }

    /// Fill a reviewed safe subset of AcroForm fields and save a new PDF.
    /// The immutable source snapshot is never changed, and PDF actions/scripts
    /// are never executed or submitted to the network.
    pub async fn export_filled_form(
        &self,
        edits: Vec<PdfFormEdit>,
        destination: PathBuf,
    ) -> WorkspaceResult<()> {
        if edits.is_empty() {
            return Err(WorkspaceError::Invalid(
                "Change at least one editable field before saving a filled copy.".into(),
            ));
        }
        let cancel = CancellationToken::new();
        let fields = self.form_fields(&cancel).await?;
        validate_pdf_form_edits(&fields, &edits)?;
        let xfdf = build_pdf_xfdf(&edits)?;
        let filled = run_pdftk_fill(self.bytes.clone(), xfdf, &cancel).await?;
        if !filled.starts_with(b"%PDF-") || filled.len() > MAX_PDF_FILLED_BYTES {
            return Err(WorkspaceError::Invalid(
                "The form helper returned an invalid or oversized PDF.".into(),
            ));
        }
        // Re-inspect the generated copy before writing it. Requested values must
        // round-trip exactly through the helper.
        let verified =
            parse_pdf_form_fields(&run_pdf_form_dump(filled.clone().into(), &cancel).await?)?;
        for edit in &edits {
            let field = verified
                .iter()
                .find(|field| field.name == edit.name)
                .ok_or_else(|| {
                    WorkspaceError::Invalid(
                        "The filled PDF no longer contains a reviewed form field.".into(),
                    )
                })?;
            if field.value != edit.value {
                return Err(WorkspaceError::Invalid(
                    "The filled PDF did not retain the reviewed field value.".into(),
                ));
            }
        }
        tokio::task::spawn_blocking(move || {
            crate::storage::write_new_export(&destination, &filled)
        })
        .await
        .map_err(|_| WorkspaceError::Worker)??;
        Ok(())
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

fn parse_pdf_form_fields(output: &[u8]) -> WorkspaceResult<Vec<PdfFormField>> {
    let text = std::str::from_utf8(output)
        .map_err(|_| WorkspaceError::Invalid("Invalid PDF form metadata.".into()))?;
    #[derive(Default)]
    struct Pending {
        kind: Option<PdfFormFieldType>,
        name: Option<String>,
        alternate_name: Option<String>,
        value: String,
        default_value: Option<String>,
        flags: Option<u32>,
        options: Vec<String>,
    }
    fn finish(
        pending: Pending,
        fields: &mut Vec<PdfFormField>,
        names: &mut HashSet<String>,
    ) -> WorkspaceResult<()> {
        if pending.name.is_none() && pending.kind.is_none() && pending.flags.is_none() {
            return Ok(());
        }
        let name = pending
            .name
            .ok_or_else(|| WorkspaceError::Invalid("PDF form field has no name.".into()))?;
        if name.is_empty()
            || name.len() > 1024
            || name.chars().any(|c| c.is_control())
            || !names.insert(name.clone())
        {
            return Err(WorkspaceError::Invalid(
                "PDF form field names are invalid or ambiguous.".into(),
            ));
        }
        if fields.len() >= MAX_PDF_FORM_FIELDS {
            return Err(RuntimeError::Limit.into());
        }
        if pending.options.len() > MAX_PDF_FORM_OPTIONS
            || pending.value.len() > MAX_PDF_FORM_VALUE_BYTES
            || pending
                .default_value
                .as_ref()
                .is_some_and(|v| v.len() > MAX_PDF_FORM_VALUE_BYTES)
        {
            return Err(RuntimeError::Limit.into());
        }
        fields.push(PdfFormField {
            name,
            alternate_name: pending.alternate_name,
            kind: pending.kind.unwrap_or(PdfFormFieldType::Unknown),
            value: pending.value,
            default_value: pending.default_value,
            flags: pending.flags.unwrap_or(0),
            options: pending.options,
        });
        Ok(())
    }

    let mut fields = Vec::new();
    let mut names = HashSet::new();
    let mut pending = Pending::default();
    for line in text.lines() {
        if line == "---" {
            finish(std::mem::take(&mut pending), &mut fields, &mut names)?;
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.strip_prefix(' ').unwrap_or(value);
        if value.len() > MAX_PDF_FORM_VALUE_BYTES || value.contains('\0') {
            return Err(RuntimeError::Limit.into());
        }
        match key {
            "FieldType" => {
                pending.kind = Some(match value {
                    "Text" => PdfFormFieldType::Text,
                    "Button" => PdfFormFieldType::Button,
                    "Choice" => PdfFormFieldType::Choice,
                    "Signature" => PdfFormFieldType::Signature,
                    _ => PdfFormFieldType::Unknown,
                });
            }
            "FieldName" => pending.name = Some(value.to_owned()),
            "FieldNameAlt" => pending.alternate_name = Some(value.to_owned()),
            "FieldValue" => pending.value = value.to_owned(),
            "FieldValueDefault" => pending.default_value = Some(value.to_owned()),
            "FieldFlags" => {
                pending.flags =
                    Some(value.parse::<u32>().map_err(|_| {
                        WorkspaceError::Invalid("Invalid PDF form field flags.".into())
                    })?)
            }
            "FieldStateOption" => {
                if pending.options.len() == MAX_PDF_FORM_OPTIONS {
                    return Err(RuntimeError::Limit.into());
                }
                pending.options.push(value.to_owned());
            }
            _ => {}
        }
    }
    finish(pending, &mut fields, &mut names)?;
    Ok(fields)
}

fn valid_pdf_form_value(value: &str) -> bool {
    value.len() <= MAX_PDF_FORM_VALUE_BYTES
        && !value.contains('\0')
        && value
            .chars()
            .all(|c| c == '\t' || c == '\n' || c == '\r' || !c.is_control())
}

fn validate_pdf_form_edits(fields: &[PdfFormField], edits: &[PdfFormEdit]) -> WorkspaceResult<()> {
    if edits.len() > MAX_PDF_FORM_FIELDS {
        return Err(RuntimeError::Limit.into());
    }
    let mut names = HashSet::new();
    for edit in edits {
        if !names.insert(edit.name.as_str()) || !valid_pdf_form_value(&edit.value) {
            return Err(WorkspaceError::Invalid("Invalid PDF form edit.".into()));
        }
        let field = fields
            .iter()
            .find(|field| field.name == edit.name)
            .ok_or_else(|| {
                WorkspaceError::Invalid("PDF form field changed after review.".into())
            })?;
        if !field.editable() {
            return Err(WorkspaceError::Invalid(
                "This PDF field type or flag combination is inspection-only.".into(),
            ));
        }
        match field.kind {
            PdfFormFieldType::Text => {}
            PdfFormFieldType::Button | PdfFormFieldType::Choice => {
                if !field.options.iter().any(|option| option == &edit.value) {
                    return Err(WorkspaceError::Invalid(
                        "Choose one of the PDF field's reported states/options.".into(),
                    ));
                }
            }
            PdfFormFieldType::Signature | PdfFormFieldType::Unknown => {
                return Err(WorkspaceError::Invalid(
                    "This PDF field cannot be edited safely.".into(),
                ));
            }
        }
    }
    Ok(())
}

fn xml_escape(value: &str, output: &mut String) {
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(ch),
        }
    }
}

fn build_pdf_xfdf(edits: &[PdfFormEdit]) -> WorkspaceResult<Vec<u8>> {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><xfdf xmlns=\"http://ns.adobe.com/xfdf/\" xml:space=\"preserve\"><fields>",
    );
    for edit in edits {
        if edit.name.is_empty()
            || edit.name.len() > 1024
            || edit.name.chars().any(|c| c.is_control())
            || !valid_pdf_form_value(&edit.value)
        {
            return Err(WorkspaceError::Invalid("Invalid PDF form edit.".into()));
        }
        xml.push_str("<field name=\"");
        xml_escape(&edit.name, &mut xml);
        xml.push_str("\"><value>");
        xml_escape(&edit.value, &mut xml);
        xml.push_str("</value></field>");
        if xml.len() > MAX_PDF_FORM_METADATA_BYTES {
            return Err(RuntimeError::Limit.into());
        }
    }
    xml.push_str("</fields></xfdf>");
    Ok(xml.into_bytes())
}

async fn run_pdf_form_dump(
    bytes: Arc<[u8]>,
    cancel: &CancellationToken,
) -> WorkspaceResult<Vec<u8>> {
    run_pdf_helper(
        bytes,
        "/usr/bin/pdftk",
        vec![
            "-".into(),
            "dump_data_fields_utf8".into(),
            "output".into(),
            "-".into(),
        ],
        MAX_PDF_FORM_METADATA_BYTES,
        "PDF AcroForm inspection",
        cancel,
    )
    .await
}

async fn run_pdf_ocr_tool(png: Arc<[u8]>, cancel: &CancellationToken) -> WorkspaceResult<Vec<u8>> {
    if cancel.is_cancelled() {
        return Err(RuntimeError::Closed.into());
    }
    if png.len() > 12 * 1024 * 1024 || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(RuntimeError::Limit.into());
    }
    let executable = "/usr/bin/tesseract";
    if !cfg!(target_os = "linux")
        || !std::path::Path::new(executable).is_file()
        || !std::path::Path::new("/usr/bin/prlimit").is_file()
    {
        return Err(RuntimeError::Unsupported(
            "PDF OCR currently requires Linux with the system tesseract-ocr and util-linux packages installed. No helper was downloaded or started".into(),
        )
        .into());
    }
    let scratch = tempfile::Builder::new()
        .prefix("synara-pdf-ocr-")
        .tempdir()
        .map_err(RuntimeError::from)?;
    let mut command = Command::new("/usr/bin/prlimit");
    command.args([
        "--as=805306368",
        "--cpu=15",
        "--fsize=16777216",
        "--nofile=64",
        "--",
        executable,
        "stdin",
        "stdout",
        "-l",
        "eng",
        "--psm",
        "3",
    ]);
    command
        .current_dir(scratch.path())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("HOME", scratch.path())
        .env("TMPDIR", scratch.path())
        .env("OMP_THREAD_LIMIT", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(RuntimeError::from)?;
    let mut stdin = child.stdin.take().ok_or_else(|| WorkspaceError::Worker)?;
    let stdout = child.stdout.take().ok_or_else(|| WorkspaceError::Worker)?;
    let stderr = child.stderr.take().ok_or_else(|| WorkspaceError::Worker)?;
    let input = png.clone();
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(RuntimeError::Closed.into()),
        result = tokio::time::timeout(Duration::from_secs(20), async {
            let write = async move {
                stdin.write_all(&input).await?;
                stdin.shutdown().await?;
                Ok::<_, RuntimeError>(())
            };
            let wait = async { child.wait().await.map_err(RuntimeError::from) };
            let (_, output, _, status) = tokio::try_join!(
                write,
                bounded_pdf_read(stdout, MAX_PDF_OCR_BYTES),
                bounded_pdf_read(stderr, MAX_PDF_DIAGNOSTIC_BYTES),
                wait,
            )?;
            if !status.success() {
                return Err(RuntimeError::Invalid("PDF OCR failed. The source PDF was not modified".into()));
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

async fn run_pdftk_fill(
    bytes: Arc<[u8]>,
    xfdf: Vec<u8>,
    cancel: &CancellationToken,
) -> WorkspaceResult<Vec<u8>> {
    if cancel.is_cancelled() {
        return Err(RuntimeError::Closed.into());
    }
    if bytes.len() > MAX_PDF_BYTES
        || !bytes.starts_with(b"%PDF-")
        || xfdf.len() > MAX_PDF_FORM_METADATA_BYTES
    {
        return Err(RuntimeError::Limit.into());
    }
    let executable = "/usr/bin/pdftk";
    if !cfg!(target_os = "linux")
        || !std::path::Path::new(executable).is_file()
        || !std::path::Path::new("/usr/bin/prlimit").is_file()
    {
        return Err(RuntimeError::Unsupported(
            "PDF AcroForm editing currently requires Linux with the system pdftk-java and util-linux packages installed. No helper was downloaded or started".into(),
        )
        .into());
    }
    let scratch = tempfile::Builder::new()
        .prefix("synara-pdf-form-")
        .tempdir()
        .map_err(RuntimeError::from)?;
    let xfdf_path = scratch.path().join("reviewed-values.xfdf");
    tokio::fs::write(&xfdf_path, xfdf)
        .await
        .map_err(RuntimeError::from)?;
    let mut command = Command::new("/usr/bin/prlimit");
    command.args([
        "--as=805306368",
        "--cpu=15",
        "--fsize=16777216",
        "--nofile=64",
        "--",
        executable,
        "-",
        "fill_form",
        "reviewed-values.xfdf",
        "output",
        "-",
        "need_appearances",
        "keep_first_id",
    ]);
    command
        .current_dir(scratch.path())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("HOME", scratch.path())
        .env("TMPDIR", scratch.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(RuntimeError::from)?;
    let mut stdin = child.stdin.take().ok_or_else(|| WorkspaceError::Worker)?;
    let stdout = child.stdout.take().ok_or_else(|| WorkspaceError::Worker)?;
    let stderr = child.stderr.take().ok_or_else(|| WorkspaceError::Worker)?;
    let input = bytes.clone();
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(RuntimeError::Closed.into()),
        result = tokio::time::timeout(Duration::from_secs(20), async {
            let write = async move {
                stdin.write_all(&input).await?;
                stdin.shutdown().await?;
                Ok::<_, RuntimeError>(())
            };
            let wait = async { child.wait().await.map_err(RuntimeError::from) };
            let (_, output, _, status) = tokio::try_join!(
                write,
                bounded_pdf_read(stdout, MAX_PDF_FILLED_BYTES),
                bounded_pdf_read(stderr, MAX_PDF_DIAGNOSTIC_BYTES),
                wait,
            )?;
            if !status.success() {
                return Err(RuntimeError::Invalid("PDF form filling failed. The source PDF was not modified".into()));
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
        let package = if executable == "/usr/bin/pdftk" {
            "pdftk-java"
        } else {
            "poppler-utils"
        };
        return Err(RuntimeError::Unsupported(format!(
            "{operation} currently requires Linux with the system {package} and util-linux packages installed. No helper was downloaded or started"
        ))
        .into());
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
fn form_kind(info: &[u8]) -> WorkspaceResult<PdfFormKind> {
    let text = std::str::from_utf8(info)
        .map_err(|_| WorkspaceError::Invalid("Invalid PDF metadata.".into()))?;
    let mut values = text.lines().filter_map(|line| line.strip_prefix("Form:"));
    let value = values.next().map(str::trim);
    if values.next().is_some() {
        return Err(WorkspaceError::Invalid(
            "The PDF form metadata is ambiguous.".into(),
        ));
    }
    Ok(match value {
        Some("none") | None => PdfFormKind::None,
        Some("AcroForm") => PdfFormKind::AcroForm,
        Some("XFA") => PdfFormKind::Xfa,
        Some(_) => PdfFormKind::Unknown,
    })
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
            form: form_kind(&info)?,
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
    fn form_dump_parser_and_xfdf_are_bounded_and_safe() {
        let dump = b"---\nFieldType: Text\nFieldName: name\nFieldNameAlt: Full name\nFieldFlags: 2\nFieldValue: Ada\n---\nFieldType: Button\nFieldName: agree\nFieldFlags: 0\nFieldValue: Off\nFieldStateOption: Off\nFieldStateOption: Yes\n---\nFieldType: Signature\nFieldName: signature\nFieldFlags: 0\n";
        let fields = parse_pdf_form_fields(dump).unwrap();
        assert_eq!(fields.len(), 3);
        assert!(fields[0].editable());
        assert!(fields[0].required());
        assert!(fields[1].editable());
        assert!(!fields[2].editable());
        validate_pdf_form_edits(
            &fields,
            &[
                PdfFormEdit {
                    name: "name".into(),
                    value: "A&B <reviewed>".into(),
                },
                PdfFormEdit {
                    name: "agree".into(),
                    value: "Yes".into(),
                },
            ],
        )
        .unwrap();
        assert!(
            validate_pdf_form_edits(
                &fields,
                &[PdfFormEdit {
                    name: "agree".into(),
                    value: "invented".into(),
                }],
            )
            .is_err()
        );
        let xfdf = String::from_utf8(
            build_pdf_xfdf(&[PdfFormEdit {
                name: "name".into(),
                value: "A&B <reviewed>".into(),
            }])
            .unwrap(),
        )
        .unwrap();
        assert!(xfdf.contains("A&amp;B &lt;reviewed&gt;"));
        assert!(!xfdf.contains("A&B <reviewed>"));
    }

    #[test]
    fn unsafe_form_field_flags_stay_inspection_only() {
        let text = PdfFormField {
            name: "password".into(),
            alternate_name: None,
            kind: PdfFormFieldType::Text,
            value: String::new(),
            default_value: None,
            flags: 1 << 13,
            options: vec![],
        };
        let push = PdfFormField {
            name: "submit".into(),
            alternate_name: None,
            kind: PdfFormFieldType::Button,
            value: String::new(),
            default_value: None,
            flags: 1 << 16,
            options: vec!["Go".into()],
        };
        let multi = PdfFormField {
            name: "many".into(),
            alternate_name: None,
            kind: PdfFormFieldType::Choice,
            value: String::new(),
            default_value: None,
            flags: 1 << 21,
            options: vec!["a".into(), "b".into()],
        };
        assert!(!text.editable());
        assert!(!push.editable());
        assert!(!multi.editable());
    }

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
    fn form_metadata_is_bounded_read_only_and_never_inferred_from_other_fields() {
        assert_eq!(
            form_kind(b"Pages: 2\nForm: none\n").unwrap(),
            PdfFormKind::None
        );
        assert_eq!(
            form_kind(b"Pages: 2\nForm: AcroForm\n").unwrap(),
            PdfFormKind::AcroForm
        );
        assert_eq!(
            form_kind(b"Pages: 2\nForm: XFA\n").unwrap(),
            PdfFormKind::Xfa
        );
        assert_eq!(
            form_kind(b"Pages: 2\nForm: SomethingNew\n").unwrap(),
            PdfFormKind::Unknown
        );
        assert_eq!(
            form_kind(b"Pages: 2\nTitle: Form: AcroForm\n").unwrap(),
            PdfFormKind::None
        );
        assert!(form_kind(b"Form: none\nForm: AcroForm\n").is_err());
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
        assert_eq!(
            parse_pdf_links(output, 2).unwrap(),
            ["https://second.example/"]
        );
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
            form: PdfFormKind::None,
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
