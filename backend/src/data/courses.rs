//! Course catalogue lookup. The data comes from the profile's `courses.json` (see `profile.rs`).
//!
//! Format:
//! - `courses[]`: `id`, `name`, optional `aliases`, `lessons` (one description per lesson, ideally
//!   starting with the chapter title in 【】).
//! - Optional numbering across a series: courses sharing a `series` with a `first_lesson` are
//!   numbered continuously, e.g. L1-L15 for the first course, L16-L30 for the next. A lesson number
//!   inside a course's range then picks that course on its own.
//! - `series[]`: `id` plus `aliases`, so "機器人2" or "rb 2" normalise to the series id.

use crate::profile;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct Catalogue {
    #[serde(default)]
    pub series: Vec<Series>,
    pub courses: Vec<Course>,
}

#[derive(Debug, Deserialize)]
pub struct Series {
    pub id: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Course {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub series: Option<String>,
    /// Global number of this course's first lesson within its series (1, 16, 31, ...).
    #[serde(default = "one")]
    pub first_lesson: usize,
    pub lessons: Vec<String>,
}

fn one() -> usize {
    1
}

impl Course {
    fn contains(&self, n: usize) -> bool {
        self.series.is_some()
            && n >= self.first_lesson
            && n < self.first_lesson + self.lessons.len()
    }
}

/// What the extension needs to build its menus (`GET /v1/courses`).
#[derive(Debug, Serialize)]
pub struct CourseSummary {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub series: Option<String>,
    pub first_lesson: usize,
    /// Chapter titles (the text in the first 【】 of each lesson), for nicer lesson menus.
    pub chapters: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CatalogueSummary {
    pub series: Vec<SeriesSummary>,
    pub courses: Vec<CourseSummary>,
}

#[derive(Debug, Serialize)]
pub struct SeriesSummary {
    pub id: String,
    pub aliases: Vec<String>,
}

fn chapter_title(lesson: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"【([^】]+)】").unwrap());
    re.captures(lesson)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default()
}

impl Catalogue {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let cat: Catalogue = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if cat.courses.is_empty() {
            return Err("no courses defined".into());
        }
        for c in &cat.courses {
            if c.lessons.is_empty() {
                return Err(format!("course {} has no lessons", c.id));
            }
            if let Some(s) = &c.series {
                if !cat.series.iter().any(|x| &x.id == s) {
                    return Err(format!("course {} uses undefined series {s}", c.id));
                }
            }
        }
        Ok(cat)
    }

    pub fn summary(&self) -> CatalogueSummary {
        CatalogueSummary {
            series: self
                .series
                .iter()
                .map(|s| SeriesSummary {
                    id: s.id.clone(),
                    aliases: s.aliases.clone(),
                })
                .collect(),
            courses: self
                .courses
                .iter()
                .map(|c| CourseSummary {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    aliases: c.aliases.clone(),
                    series: c.series.clone(),
                    first_lesson: c.first_lesson,
                    chapters: c.lessons.iter().map(|l| chapter_title(l)).collect(),
                })
                .collect(),
        }
    }

    /// Lowercase, drop whitespace / punctuation, and map series aliases to the series id,
    /// so "Robot 0", "機器人0" and "RB-0" all become "robot0".
    fn normalize(&self, s: &str) -> String {
        let input = compact(s);
        for series in &self.series {
            let mut names: Vec<String> = series.aliases.iter().map(|a| compact(a)).collect();
            names.push(compact(&series.id));
            // Longest alias first so "我的世界" wins over a shorter overlapping alias.
            names.sort_by_key(|a| std::cmp::Reverse(a.chars().count()));
            for alias in names.iter().filter(|a| !a.is_empty()) {
                if let Some(rest) = input.strip_prefix(alias.as_str()) {
                    return format!("{}{rest}", compact(&series.id));
                }
            }
        }
        input
    }

    fn find_course(&self, course: &str) -> Option<&Course> {
        let input = self.normalize(course);
        if input.is_empty() {
            return None;
        }
        let names = |c: &Course| {
            std::iter::once(&c.id)
                .chain(std::iter::once(&c.name))
                .chain(c.aliases.iter())
                .map(|n| self.normalize(n))
                .collect::<Vec<_>>()
        };
        self.courses
            .iter()
            .find(|c| names(c).contains(&input))
            // Tolerate trailing extras such as "Robot 0 L4"; longest id wins.
            .or_else(|| {
                self.courses
                    .iter()
                    .filter(|c| input.starts_with(&self.normalize(&c.id)))
                    .max_by_key(|c| c.id.len())
            })
            // A shortened name ("Scratch" for "Scratch 入門") when exactly one course matches.
            .or_else(|| {
                let mut hits = self
                    .courses
                    .iter()
                    .filter(|c| names(c).iter().any(|n| n.starts_with(&input)));
                match (hits.next(), hits.next()) {
                    (Some(c), None) => Some(c),
                    _ => None,
                }
            })
    }

    /// The series a free-text course refers to: blank means "any series".
    fn series_of(&self, course: &str) -> Option<Option<&str>> {
        let input = self.normalize(course);
        if input.is_empty() {
            return Some(None);
        }
        self.series
            .iter()
            .find(|s| input.starts_with(&compact(&s.id)))
            .map(|s| Some(s.id.as_str()))
    }

    /// Returns `(course display name, lesson description)`; empty strings when unknown.
    ///
    /// - A number inside a series course's global range (e.g. L20) picks that course on its own.
    /// - A small number with a course named (e.g. "機器人進階" + "L5") is a lesson within that course.
    pub fn lookup(&self, course: &str, lesson: &str) -> (String, String) {
        let named = self.find_course(course);
        let n = parse_lesson_number(lesson);

        let global = |n: usize, series: Option<&str>| {
            self.courses
                .iter()
                .filter(|c| series.is_none() || c.series.as_deref() == series)
                .find(|c| c.contains(n))
                .map(|c| (c, n - c.first_lesson))
        };

        let resolved = match (named, n) {
            (Some(c), Some(n)) if c.contains(n) => Some((c, n - c.first_lesson)),
            (Some(c), Some(n)) if n <= c.lessons.len() => Some((c, n - 1)),
            // Number outside the named course: look it up within the same series.
            (Some(c), Some(n)) => c.series.as_deref().and_then(|s| global(n, Some(s))),
            // Unknown course text: only a blank course or a series name may use global numbering,
            // so e.g. "Scratch L3" never picks up another course's content.
            (None, Some(n)) => self.series_of(course).and_then(|series| global(n, series)),
            (Some(c), None) => return (c.name.clone(), String::new()),
            (None, None) => None,
        };

        match resolved {
            Some((c, idx)) => (
                c.name.clone(),
                c.lessons.get(idx).cloned().unwrap_or_default(),
            ),
            None => {
                tracing::warn!(course, lesson, "Course/lesson not found in catalogue");
                (String::new(), String::new())
            }
        }
    }
}

fn compact(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_lesson_number(lesson: &str) -> Option<usize> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\d+").unwrap());
    re.find(lesson)
        .and_then(|m| m.as_str().parse::<usize>().ok())
        .filter(|&n| n >= 1)
}

/// Look up a lesson in the loaded profile's catalogue.
pub fn get_course_detail(course: &str, lesson: &str) -> (String, String) {
    profile::get().catalogue.lookup(course, lesson)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> Catalogue {
        Catalogue::from_json(include_str!("../../config/courses.example.json")).unwrap()
    }

    /// The same shape as a real three-course series numbered L1-L45.
    fn series_catalogue() -> Catalogue {
        let lessons = |tag: &str| {
            (1..=15)
                .map(|i| format!("【{tag}{i}】說明"))
                .collect::<Vec<_>>()
        };
        let json = serde_json::json!({
            "series": [{ "id": "robot", "aliases": ["機器人", "樂高機器人", "rb"] }],
            "courses": [
                { "id": "Robot0", "name": "機器人初階", "series": "robot", "first_lesson": 1, "lessons": lessons("初") },
                { "id": "Robot1", "name": "機器人進階", "series": "robot", "first_lesson": 16, "lessons": lessons("進") },
                { "id": "Robot2", "name": "機器人應用", "series": "robot", "first_lesson": 31, "lessons": lessons("應") },
                { "id": "Scratch1", "name": "Scratch 入門", "lessons": lessons("S") }
            ]
        });
        Catalogue::from_json(&json.to_string()).unwrap()
    }

    #[test]
    fn test_example_catalogue_is_valid() {
        let cat = example();
        assert!(cat.courses.len() >= 2);
        assert!(cat
            .summary()
            .courses
            .iter()
            .all(|c| c.chapters.iter().all(|t| !t.is_empty())));
    }

    #[test]
    fn test_invalid_catalogue() {
        assert!(Catalogue::from_json(r#"{"courses":[]}"#).is_err());
        assert!(Catalogue::from_json(
            r#"{"courses":[{"id":"a","name":"A","series":"x","lessons":["1"]}]}"#
        )
        .is_err());
    }

    #[test]
    fn test_parse_lesson_number() {
        assert_eq!(parse_lesson_number("L1"), Some(1));
        assert_eq!(parse_lesson_number("Lesson 27"), Some(27));
        assert_eq!(parse_lesson_number("L0"), None);
        assert_eq!(parse_lesson_number("invalid"), None);
    }

    #[test]
    fn test_lenient_course_names() {
        let cat = series_catalogue();
        for input in [
            "Robot 0",
            "robot0",
            "機器人0",
            "機器人 0",
            "RB-0",
            "機器人初階",
            "Robot 0 L4",
        ] {
            assert_eq!(cat.lookup(input, "L4").0, "機器人初階", "input: {input}");
        }
        assert!(cat.lookup("Robot 0", "L4").1.contains("初4"));
    }

    #[test]
    fn test_global_lesson_numbers() {
        let cat = series_catalogue();
        assert_eq!(cat.lookup("", "L1").0, "機器人初階");
        assert!(cat.lookup("", "L24").1.contains("進9"));
        assert!(cat.lookup("Robot0", "L41").1.contains("應11"));
        assert!(cat.lookup("機器人", "L20").1.contains("進5"));
        assert!(cat.lookup("機器人進階", "L24").1.contains("進9"));
        assert!(cat.lookup("機器人進階", "L9").1.contains("進9"));
        assert_eq!(cat.lookup("", "L46"), (String::new(), String::new()));
    }

    #[test]
    fn test_courses_outside_a_series() {
        let cat = series_catalogue();
        assert!(cat.lookup("Scratch", "L3").1.contains("S3"));
        // Unknown course never borrows series content
        assert_eq!(cat.lookup("Python", "L1"), (String::new(), String::new()));
        // A series lesson number doesn't leak into a non-series course
        assert_eq!(cat.lookup("Scratch", "L20"), (String::new(), String::new()));
    }

    #[test]
    fn test_example_series() {
        let cat = example();
        assert_eq!(cat.lookup("Python", "L5").0, "Python 進階");
        assert!(cat.lookup("py", "L5").1.contains("單字卡"));
        assert!(cat.lookup("Scratch", "L2").1.contains("貓咪抓老鼠"));
    }
}
