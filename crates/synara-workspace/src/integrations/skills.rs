use super::*;
use std::{fs, io::Read, path::PathBuf};

/// Immutable review snapshot. Installation rechecks the exact selected bytes.
/// No archive extraction, package manager, shell or provider-directory writes.
#[derive(Clone, Debug)]
pub struct SkillReview {
    skill: InstalledSkill,
}
impl SkillReview {
    pub fn document(&self) -> &InstalledSkill { &self.skill }
    pub fn inspect(path: PathBuf) -> WorkspaceResult<Self> {
        if !path.is_absolute() || !path.extension().is_some_and(|v| v.eq_ignore_ascii_case("md")) {
            return Err(invalid("Choose an absolute path to a Markdown (.md) skill document."));
        }
        for part in path.ancestors() {
            if fs::symlink_metadata(part).map_err(|_| invalid("The selected skill path is unavailable."))?.file_type().is_symlink() {
                return Err(invalid("Skill imports cannot follow symbolic links."));
            }
        }
        let meta = fs::symlink_metadata(&path).map_err(|_| invalid("Cannot inspect the selected skill."))?;
        if !meta.is_file() || meta.len() > MAX_SKILL_BYTES as u64 {
            return Err(invalid("Choose a regular Markdown file no larger than 64 KiB."));
        }
        let mut bytes = Vec::new();
        fs::File::open(&path).map_err(|_| invalid("Cannot open the selected skill."))?
            .take(MAX_SKILL_BYTES as u64 + 1).read_to_end(&mut bytes)
            .map_err(|_| invalid("Cannot read the selected skill."))?;
        let markdown = String::from_utf8(bytes).map_err(|_| invalid("Skill documents must be UTF-8 text."))?;
        let mut title = path.file_stem().and_then(|p| p.to_str()).unwrap_or("Skill document").to_owned();
        let mut description = String::new();
        let mut version = None;
        // Display a bounded literal frontmatter subset. No YAML tags, evaluation,
        // includes, network references, or claims of full Agent Skills compatibility.
        if markdown.starts_with("---\n") || markdown.starts_with("---\r\n") {
            for line in markdown.lines().skip(1).take(64) {
                if line == "---" { break; }
                if let Some((key, value)) = line.split_once(':') {
                    let value = value.trim().trim_matches(['\'', '"']).to_string();
                    match key.trim() {
                        "name" | "title" if text(&value,160) => title = value,
                        "description" if text(&value,1024) => description = value,
                        "version" if text(&value,128) => version = Some(value),
                        _ => {},
                    }
                }
            }
        } else if let Some(heading) = markdown.lines().find_map(|line|line.strip_prefix("# ")) {
            if text(heading,160) { title = heading.to_owned(); }
        }
        let skill = InstalledSkill {
            id: uuid::Uuid::new_v4().to_string(), title, description,
            origin: SkillOrigin {
                path: path.to_str().ok_or_else(|| invalid("Skill paths must be UTF-8."))?.into(),
                sha256: digest(&markdown), version,
            }, previous_origins: vec![], markdown, enabled: false,
        };
        skill.validate()?;
        Ok(Self { skill })
    }
    pub(crate) fn recheck(self) -> WorkspaceResult<InstalledSkill> {
        let current = Self::inspect(PathBuf::from(&self.skill.origin.path))?;
        if current.skill.origin != self.skill.origin {
            return Err(invalid("The skill changed after review. Review the new document before installing."));
        }
        Ok(self.skill)
    }
}
