use crate::api::schemas::GenerateRequest;
use crate::error::AppResult;
use crate::profile::{self, Style};
use crate::prompts::builder::{build_prompt, ensure_last_week_prefix, to_single_paragraph};
use crate::services::gemini::GeminiClient;

/// Deterministic clean-up for rules the model reliably ignores: swap stiff wording and keep only
/// the first `max_name_mentions` mentions of the student's name (Chinese reads fine without the subject).
fn polish_style(style: &Style, student: &str, comment: &str) -> String {
    let mut text = comment.to_string();
    for (from, to) in &style.replacements {
        text = text.replace(from.as_str(), to);
    }
    let name = student.trim();
    if name.is_empty() {
        return text;
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    let mut seen = 0;
    while let Some(pos) = rest.find(name) {
        out.push_str(&rest[..pos]);
        rest = &rest[pos + name.len()..];
        seen += 1;
        // Only drop a name that starts a clause; mid-clause ("讓小明練習") it is load-bearing.
        let clause_start = out.is_empty() || out.ends_with(['，', '。', '！', '；']);
        if seen <= style.max_name_mentions || !clause_start {
            out.push_str(name);
        } else if let Some(after) = rest.strip_prefix('的') {
            rest = after; // "小明的作品" -> "作品"
        }
    }
    out.push_str(rest);
    out
}

/// Returns a short description of the first rule the comment breaks, if any.
fn style_problem(style: &Style, request: &GenerateRequest, comment: &str) -> Option<String> {
    let chars = comment.chars().count();
    if chars < style.min_chars {
        return Some(format!("too short ({chars} chars)"));
    }
    let name = request.student.trim();
    // Words like 其他 / 吉他 contain 他 but aren't pronouns.
    let text = comment.replace("其他", "").replace("吉他", "");
    // A teacher-typed issue may legitimately contain a banned word; only flag words the model added.
    style
        .banned_words
        .iter()
        .find(|w| {
            text.contains(w.as_str())
                && !request.issue.contains(w.as_str())
                && !name.contains(w.as_str())
        })
        .map(|w| format!("banned word {w}"))
}

pub async fn generate_comment(
    request: &GenerateRequest,
    gemini_client: &GeminiClient,
) -> AppResult<String> {
    let profile = profile::get();
    let (system_prompt, style) = (profile.system_prompt.as_str(), &profile.style);
    let user_prompt = build_prompt(request);

    tracing::debug!("System prompt: {}", system_prompt);
    tracing::debug!("User prompt: {}", user_prompt);

    // The model doesn't always follow the style rules; regenerate when the draft clearly breaks one.
    let mut comment = String::new();
    for attempt in 1..=style.max_attempts.max(1) {
        comment = to_single_paragraph(
            &gemini_client
                .generate_text(system_prompt, &user_prompt)
                .await?,
        );
        match style_problem(style, request, &comment) {
            None => break,
            Some(problem) => {
                tracing::warn!(
                    attempt,
                    problem,
                    "Generated comment breaks style rules, regenerating"
                )
            }
        }
    }

    let comment = ensure_last_week_prefix(request, polish_style(style, &request.student, &comment));

    tracing::info!("Successfully generated comment");
    Ok(comment)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(student: &str) -> GenerateRequest {
        serde_json::from_str(&format!(
            r#"{{"student":"{student}","course":"","lesson":"","progress":"","performance":"","issue":""}}"#
        ))
        .unwrap()
    }

    #[test]
    fn test_polish_style() {
        assert_eq!(
            polish_style(&profile::get().style, "小明", "小明很好。小明對於迴圈有掌握。小明展現出耐心。小明的作品完成了，小明也通過驗收，讓小明很開心。"),
            "小明很好。小明對迴圈有掌握。小明有耐心。作品完成了，也通過驗收，讓小明很開心。"
        );
    }

    #[test]
    fn test_style_problem() {
        let long = "做得不錯。".repeat(50);
        assert_eq!(
            style_problem(&profile::get().style, &req("小明"), &long),
            None
        );
        assert!(style_problem(&profile::get().style, &req("小明"), "太短了").is_some());
        assert!(style_problem(&profile::get().style, &req("小明"), &format!("{long}他")).is_some());
        assert_eq!(
            style_problem(&profile::get().style, &req("小明"), &format!("{long}其他")),
            None
        );
    }
}
