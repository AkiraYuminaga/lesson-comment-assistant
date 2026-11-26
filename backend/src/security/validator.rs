use crate::error::{AppError, AppResult};

const MAX_COURSE_NAME_LENGTH: usize = 100;

/// Validate a free-text course name
pub fn validate_course_name(name: &str) -> AppResult<String> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Err(AppError::ValidationError(
            "Course name cannot be empty".to_string(),
        ));
    }

    if trimmed.chars().count() > MAX_COURSE_NAME_LENGTH {
        return Err(AppError::ValidationError(format!(
            "Course name exceeds maximum length of {} characters",
            MAX_COURSE_NAME_LENGTH
        )));
    }

    // Allow letters (incl. CJK), digits, spaces and common punctuation used in course names
    // (e.g. "Scratch / Python", "Python (進階)"); reject markup and control characters.
    const ALLOWED_PUNCT: &str = "-_/.+&()（）【】[]:：、,，·";
    if !trimmed
        .chars()
        .all(|c| c.is_alphanumeric() || c == ' ' || ALLOWED_PUNCT.contains(c))
    {
        return Err(AppError::ValidationError(
            "Course name contains invalid characters".to_string(),
        ));
    }

    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_course_name() {
        assert!(validate_course_name("Scratch1").is_ok());
        assert!(validate_course_name("Scratch Level-1").is_ok());
        assert!(validate_course_name("").is_err());
        assert!(validate_course_name("Course<script>").is_err());
        assert!(validate_course_name("Scratch / Python").is_ok());
        assert!(validate_course_name(&"麥".repeat(100)).is_ok());
        assert!(validate_course_name(&"麥".repeat(101)).is_err());
    }
}
