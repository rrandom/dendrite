pub mod id;
pub mod time;

use std::path::Path;

/// Normalize a file path to a note ID.
///
/// Converts a file path to a normalized ID string by:
/// - Converting backslashes to forward slashes (Windows compatibility)
/// - Removing the `.md` extension
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use dendrite_core::normalize_path_to_id;
///
/// assert_eq!(normalize_path_to_id(Path::new("foo/bar.md")), "foo/bar");
/// assert_eq!(normalize_path_to_id(Path::new("note.md")), "note");
/// # if cfg!(windows) {
/// #     assert_eq!(normalize_path_to_id(Path::new("foo\\bar.md")), "foo/bar");
/// # }
/// ```
pub fn normalize_path_to_id(path: &Path) -> String {
    let mut s = path.to_string_lossy().to_string();
    if std::path::MAIN_SEPARATOR == '\\' {
        s = s.replace('\\', "/");
    }
    s = s.trim_end_matches(".md").to_string();
    s
}

/// Slugify a heading text to create a URL-safe anchor ID.
///
/// Rules (following Dendron behavior):
/// - Convert to lowercase
/// - Preserve Unicode letters and digits (e.g., Chinese characters)
/// - Filter out emoji
/// - Replace whitespace with hyphens
/// - Remove ASCII special characters (parentheses, exclamation marks, etc.)
/// - Trim leading/trailing hyphens
///
/// # Examples
///
/// ```
/// use dendrite_core::slugify_heading;
///
/// assert_eq!(slugify_heading("My Cool Header"), "my-cool-header");
/// assert_eq!(slugify_heading("你好 World"), "你好-world");
/// assert_eq!(slugify_heading("Hello (World)!"), "hello-world");
/// assert_eq!(slugify_heading("A  B"), "a--b"); // Consecutive spaces → consecutive hyphens
/// assert_eq!(slugify_heading("🎉 Party"), "party"); // Emoji filtered
/// assert_eq!(slugify_heading("-Hello-"), "hello"); // Trim leading/trailing hyphens
/// assert_eq!(slugify_heading("!!!"), ""); // Empty slug for pure special chars
/// ```
pub fn slugify_heading(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            // Preserve: letters (including Unicode), digits, hyphens, underscores
            // Filter: emoji, ASCII special characters
            if c.is_alphabetic() || c.is_numeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '\0' // Filter out (including emoji)
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_heading() {
        assert_eq!(slugify_heading("My Header"), "my-header");
        assert_eq!(slugify_heading("Hello World"), "hello-world");
        assert_eq!(slugify_heading("你好 World"), "你好-world");
        assert_eq!(slugify_heading("Hello (World)!"), "hello-world");
        assert_eq!(slugify_heading("A  B  C"), "a--b--c"); // Consecutive spaces
        assert_eq!(slugify_heading("🎉 Party"), "party"); // Emoji filtered
        assert_eq!(slugify_heading("-Hello-"), "hello"); // Trim hyphens
        assert_eq!(slugify_heading("!!!"), ""); // Empty slug
        assert_eq!(slugify_heading("  Hello  "), "hello"); // Trim whitespace
        assert_eq!(slugify_heading("你好"), "你好"); // Pure Chinese
    }
}
