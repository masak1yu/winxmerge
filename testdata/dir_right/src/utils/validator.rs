/// Input validation helpers used across request handlers.

/// Maximum allowed length for a username.
pub const MAX_USERNAME_LEN: usize = 32;

/// Minimum allowed length for a password.
pub const MIN_PASSWORD_LEN: usize = 8;

/// Returns `Ok(())` if `email` looks like a valid e-mail address, otherwise
/// returns an `Err` with a human-readable message.
pub fn validate_email(email: &str) -> Result<(), String> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err("Email cannot be empty".into());
    }
    let at = trimmed.find('@').ok_or_else(|| "Email must contain '@'".to_string())?;
    let domain = &trimmed[at + 1..];
    if !domain.contains('.') {
        return Err("Email domain must contain a dot".into());
    }
    Ok(())
}

/// Returns `Ok(())` if `username` is valid (alphanumeric + underscores,
/// 1–MAX_USERNAME_LEN characters).
pub fn validate_username(username: &str) -> Result<(), String> {
    if username.is_empty() {
        return Err("Username cannot be empty".into());
    }
    if username.len() > MAX_USERNAME_LEN {
        return Err(format!(
            "Username exceeds maximum length of {MAX_USERNAME_LEN}"
        ));
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Username may only contain letters, digits, and underscores".into());
    }
    Ok(())
}

/// Returns `Ok(())` if `password` meets the minimum strength requirements.
pub fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(format!(
            "Password must be at least {MIN_PASSWORD_LEN} characters"
        ));
    }
    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !has_upper || !has_digit {
        return Err("Password must contain at least one uppercase letter and one digit".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_email_accepted() {
        assert!(validate_email("user@example.com").is_ok());
    }

    #[test]
    fn email_missing_at_rejected() {
        assert!(validate_email("userexample.com").is_err());
    }

    #[test]
    fn username_too_long_rejected() {
        let long: String = "a".repeat(MAX_USERNAME_LEN + 1);
        assert!(validate_username(&long).is_err());
    }

    #[test]
    fn weak_password_rejected() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("alllowercase1").is_err());
        assert!(validate_password("NoDigitsHere").is_err());
    }
}
