//! A school's "profile": its course catalogue, the system prompt and the style rules.
//!
//! Everything lives in plain files under `CONFIG_DIR` (default `config/`), read once at startup:
//!
//! | file            | falls back to           | what it holds                                |
//! |-----------------|-------------------------|----------------------------------------------|
//! | `courses.json`  | `courses.example.json`  | courses, lesson descriptions, name aliases   |
//! | `prompt.md`     | `prompt.example.md`     | the system prompt (how comments are written) |
//! | `style.json`    | `style.example.json`    | banned words, replacements, length limits    |
//!
//! The real files are git-ignored so each teacher keeps their own; the `.example` files ship with
//! the repo and are used when a real one is missing.

use crate::data::courses::Catalogue;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Style {
    /// Regenerate when the comment is shorter than this many characters.
    pub min_chars: usize,
    /// Keep at most this many mentions of the student's name (extra clause-initial ones are dropped).
    pub max_name_mentions: usize,
    /// Total generation attempts when a draft breaks a rule (the last draft is used regardless).
    pub max_attempts: u32,
    /// Words that trigger a regeneration when the model writes them.
    pub banned_words: Vec<String>,
    /// `[from, to]` pairs applied to every comment.
    pub replacements: Vec<(String, String)>,
    /// Ways to open the comment; one is picked at random per request.
    pub openings: Vec<String>,
    /// Which part of the input to dwell on; one is picked at random per request.
    pub focuses: Vec<String>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            min_chars: 0,
            max_name_mentions: 3,
            max_attempts: 1,
            banned_words: Vec::new(),
            replacements: Vec::new(),
            openings: Vec::new(),
            focuses: Vec::new(),
        }
    }
}

pub struct Profile {
    pub catalogue: Catalogue,
    pub system_prompt: String,
    pub style: Style,
}

static PROFILE: OnceLock<Profile> = OnceLock::new();

/// `courses.json` -> `courses.example.json`
fn example_name(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) => format!("{stem}.example.{ext}"),
        None => format!("{name}.example"),
    }
}

/// The teacher's own file if present, otherwise the shipped example.
fn resolve(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let own = dir.join(name);
    if own.is_file() {
        return Ok(own);
    }
    let example = dir.join(example_name(name));
    if example.is_file() {
        tracing::warn!("{} not found, using {}", own.display(), example.display());
        return Ok(example);
    }
    Err(format!(
        "neither {} nor {} exists",
        own.display(),
        example.display()
    ))
}

fn read(dir: &Path, name: &str) -> Result<(PathBuf, String), String> {
    let path = resolve(dir, name)?;
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok((path, text))
}

pub fn load(dir: &Path) -> Result<Profile, String> {
    let (path, text) = read(dir, "courses.json")?;
    let catalogue = Catalogue::from_json(&text).map_err(|e| format!("{}: {e}", path.display()))?;

    let (_, system_prompt) = read(dir, "prompt.md")?;
    if system_prompt.trim().is_empty() {
        return Err("prompt.md is empty".into());
    }

    let (path, text) = read(dir, "style.json")?;
    let style = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;

    Ok(Profile {
        catalogue,
        system_prompt,
        style,
    })
}

/// Load the profile once at startup; later calls to [`get`] return it.
pub fn init(dir: &Path) -> Result<&'static Profile, String> {
    let profile = load(dir)?;
    tracing::info!(
        "Profile loaded from {}: {} course(s)",
        dir.display(),
        profile.catalogue.courses.len()
    );
    Ok(PROFILE.get_or_init(|| profile))
}

/// The loaded profile. Outside the server (unit tests) it lazily loads the shipped examples,
/// so tests never depend on a teacher's private files.
pub fn get() -> &'static Profile {
    PROFILE.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("config");
        Profile {
            catalogue: Catalogue::from_json(include_str!("../config/courses.example.json"))
                .expect("courses.example.json is invalid"),
            system_prompt: include_str!("../config/prompt.example.md").to_string(),
            style: serde_json::from_str(include_str!("../config/style.example.json"))
                .unwrap_or_else(|e| panic!("{}: {e}", dir.join("style.example.json").display())),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_name() {
        assert_eq!(example_name("courses.json"), "courses.example.json");
        assert_eq!(example_name("prompt.md"), "prompt.example.md");
    }

    #[test]
    fn test_examples_load() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("config");
        // Loading the directory works whether or not private files exist next to the examples.
        let profile = load(&dir).expect("config dir should load");
        assert!(!profile.catalogue.courses.is_empty());
        assert!(!get().style.openings.is_empty());
    }
}
