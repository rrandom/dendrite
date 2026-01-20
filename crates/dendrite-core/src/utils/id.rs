/// Generate a unique ID for a note (Dendron compatible)
///
/// Returns a 23-character URL-friendly string.
/// Dendron uses nanoid with a default alphabet and length of 21-23.
/// We use 23 closely matching their behavior.
pub fn generate_id() -> String {
    nanoid::nanoid!(23)
}
