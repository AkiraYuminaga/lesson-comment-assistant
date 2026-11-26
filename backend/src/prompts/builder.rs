use crate::api::schemas::GenerateRequest;
use crate::data::courses::get_course_detail;
use crate::profile;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

/// Cheap randomness without an extra crate: `RandomState` is seeded randomly per instance.
fn pick(items: &[String]) -> Option<&str> {
    if items.is_empty() {
        return None;
    }
    let n = RandomState::new().build_hasher().finish() as usize;
    Some(items[n % items.len()].as_str())
}

/// When "上週未完成" is checked the student finished last week's work *this* week and passed
/// its check, so the comment opens with this fixed sentence. The model is told NOT to write it;
/// `ensure_last_week_prefix` prepends it deterministically after generation.
pub fn last_week_prefix(req: &GenerateRequest) -> Option<String> {
    if !req.last_undone {
        return None;
    }
    let issue = req.issue.trim();
    Some(if issue.is_empty() {
        "已完成上週內容。".to_string()
    } else {
        format!("已完成上週內容，驗收問題為：【{}】。", issue)
    })
}

/// Force the comment into one paragraph: the model sometimes still splits it into several.
pub fn to_single_paragraph(comment: &str) -> String {
    comment
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("")
}

/// Make sure the generated comment starts with the last-week sentence when required.
pub fn ensure_last_week_prefix(req: &GenerateRequest, comment: String) -> String {
    match last_week_prefix(req) {
        Some(prefix) if !comment.trim_start().starts_with(&prefix) => {
            format!("{}{}", prefix, comment.trim_start())
        }
        _ => comment,
    }
}

pub fn build_prompt(req: &GenerateRequest) -> String {
    let (course_name, lesson_text) = get_course_detail(&req.course, &req.lesson);

    // Unknown course: keep what the teacher typed rather than dropping it.
    let course_label = if course_name.is_empty() {
        req.course.trim()
    } else {
        course_name.as_str()
    };

    let lesson_info = if lesson_text.is_empty() {
        "（沒有資料）"
    } else {
        lesson_text.as_str()
    };

    let mut prompt = String::new();
    prompt.push_str(&format!(
        "課程：{}\n\
        課次：{}\n\
        本課介紹（課程內容的唯一依據）：{}\n\
        學生：{}\n\
        學習表現（老師提供的關鍵字）：{}\n\
        進度：{}\n\
        驗收問題：{}\n",
        course_label,
        req.lesson,
        lesson_info,
        req.student,
        req.performance,
        req.progress,
        if req.issue.trim().is_empty() {
            "（沒有提供）"
        } else {
            req.issue.as_str()
        }
    ));

    let style = &profile::get().style;
    let directions: Vec<&str> = [pick(&style.openings), pick(&style.focuses)]
        .into_iter()
        .flatten()
        .collect();
    if !directions.is_empty() {
        prompt.push_str(&format!(
            "寫作方向（讓每次評語的寫法不一樣，但內容仍只能來自上面的資料）：{}。\
            開頭不要用「某某今天在【章節】課程中」這種固定句型。\n",
            directions.join("；")
        ));
    }

    if lesson_text.is_empty() {
        prompt.push_str(
            "備註：沒有本課的章節資料，請不要自行編造章節名稱或課程主題，\
            只根據上面提供的資訊撰寫。\n",
        );
    }

    if req.last_undone {
        prompt.push_str(
            "備註：上週補完與驗收的事系統會自動寫在開頭，你只要寫本日課程內容與學習表現，\
            不要提到上週，也不要重複驗收問題。\n",
        );
    }
    if !req.polish {
        prompt.push_str(
            "備註：請盡量保留老師原本的用字，只做必要的整理，不要大幅改寫或加入華麗修飾。\n",
        );
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt() {
        let req = GenerateRequest {
            student: "小明".to_string(),
            course: "Scratch".to_string(),
            lesson: "L1".to_string(),
            progress: "已完成".to_string(),
            performance: "表現良好".to_string(),
            issue: "會自己讓角色移動".to_string(),
            last_undone: false,
            polish: true,
        };

        let prompt = build_prompt(&req);
        assert!(prompt.contains("小明"));
        assert!(prompt.contains("Scratch 入門"));
        assert!(!prompt.contains("已完成上週內容"));
    }

    #[test]
    fn test_last_undone_camel_case() {
        let req: GenerateRequest = serde_json::from_str(
            r#"{"student":"小明","course":"","lesson":"","progress":"","performance":"","issue":"","lastUndone":true}"#,
        )
        .unwrap();
        assert!(req.last_undone);
        assert!(build_prompt(&req).contains("系統會自動寫在開頭"));
    }

    #[test]
    fn test_to_single_paragraph() {
        assert_eq!(
            to_single_paragraph("小明今天蓋了高塔。\n\n  也順利完成驗收。\n"),
            "小明今天蓋了高塔。也順利完成驗收。"
        );
    }

    #[test]
    fn test_ensure_last_week_prefix() {
        let mut req: GenerateRequest = serde_json::from_str(
            r#"{"student":"小明","course":"","lesson":"","progress":"","performance":"","issue":"迴圈是什麼？","lastUndone":true}"#,
        )
        .unwrap();
        let prefix = "已完成上週內容，驗收問題為：【迴圈是什麼？】。";
        assert_eq!(last_week_prefix(&req).as_deref(), Some(prefix));

        // Missing prefix gets added; existing prefix is not duplicated
        assert_eq!(
            ensure_last_week_prefix(&req, "小明今天很棒。".into()),
            format!("{}小明今天很棒。", prefix)
        );
        let ok = format!("{}小明今天很棒。", prefix);
        assert_eq!(ensure_last_week_prefix(&req, ok.clone()), ok);

        // Unchecked: untouched
        req.last_undone = false;
        assert_eq!(
            ensure_last_week_prefix(&req, "小明今天很棒。".into()),
            "小明今天很棒。"
        );
    }
}
