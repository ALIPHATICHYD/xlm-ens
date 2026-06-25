use crate::constants::{MAX_NAME_LENGTH, MAX_SUBDOMAIN_DEPTH, MIN_NAME_LENGTH};
use crate::types::Tld;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// Structured errors for offline name validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameValidationError {
    TooShort {
        min: usize,
        actual: usize,
    },
    TooLong {
        max: usize,
        actual: usize,
    },
    /// First invalid character and its char-index within the label.
    InvalidCharacter {
        ch: char,
        position: usize,
    },
    /// Label starts or ends with a hyphen.
    InvalidLabelBoundary,
    UnsupportedTld(String),
    MissingTld,
    InvalidName,
    ReservedName(String),
    TooManyLabels {
        max: usize,
        actual: usize,
    },
}

impl fmt::Display for NameValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { min, actual } => {
                write!(f, "label too short: minimum {min} chars, got {actual}")
            }
            Self::TooLong { max, actual } => {
                write!(f, "label too long: maximum {max} chars, got {actual}")
            }
            Self::InvalidCharacter { ch, position } => {
                write!(f, "invalid character {ch:?} at position {position}")
            }
            Self::InvalidLabelBoundary => {
                write!(f, "label must not start or end with a hyphen")
            }
            Self::UnsupportedTld(tld) => write!(f, "unsupported TLD: {tld:?}"),
            Self::MissingTld => write!(f, "name must include a TLD (e.g. .xlm)"),
            Self::InvalidName => write!(f, "name is structurally malformed"),
            Self::ReservedName(label) => write!(f, "name {label:?} is reserved"),
            Self::TooManyLabels { max, actual } => {
                write!(f, "too many labels: maximum {max}, got {actual}")
            }
        }
    }
}

impl core::error::Error for NameValidationError {}

/// Parsed, validated name. Labels are ordered left-to-right (leaf first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedName {
    /// Non-TLD labels from left to right, e.g. `["pay", "timmy"]` for `pay.timmy.xlm`.
    pub labels: Vec<String>,
    pub tld: Tld,
}

impl ValidatedName {
    /// Reconstruct the full FQDN.
    pub fn fqdn(&self) -> String {
        format!("{}.{}", self.labels.join("."), self.tld.as_str())
    }

    pub fn is_subdomain(&self) -> bool {
        self.labels.len() > 1
    }

    /// Leftmost (leaf) label — e.g. `"pay"` for `pay.timmy.xlm`.
    pub fn leaf_label(&self) -> &str {
        &self.labels[0]
    }

    /// Rightmost (base) label — e.g. `"timmy"` for `pay.timmy.xlm`.
    pub fn base_label(&self) -> &str {
        self.labels
            .last()
            .expect("validated name always has at least one label")
    }
}

/// Pure offline validation — does NOT check reserved names.
pub fn validate_name(name: &str) -> Result<ValidatedName, NameValidationError> {
    validate_name_with_reserved(name, &[])
}

/// Offline validation with an optional caller-supplied reserved-name list.
/// Pass an empty slice for format-only checks.
pub fn validate_name_with_reserved(
    name: &str,
    reserved: &[&str],
) -> Result<ValidatedName, NameValidationError> {
    // Split on the rightmost "." to extract the TLD.
    let dot_pos = name.rfind('.').ok_or(NameValidationError::MissingTld)?;
    let labels_part = &name[..dot_pos];
    let tld_str = &name[dot_pos + 1..];

    if tld_str.is_empty() {
        return Err(NameValidationError::InvalidName);
    }

    let tld = Tld::parse(tld_str)
        .ok_or_else(|| NameValidationError::UnsupportedTld(tld_str.to_string()))?;

    if labels_part.is_empty() {
        return Err(NameValidationError::TooShort {
            min: MIN_NAME_LENGTH,
            actual: 0,
        });
    }

    let raw_labels: Vec<&str> = labels_part.split('.').collect();

    if raw_labels.len() > MAX_SUBDOMAIN_DEPTH {
        return Err(NameValidationError::TooManyLabels {
            max: MAX_SUBDOMAIN_DEPTH,
            actual: raw_labels.len(),
        });
    }

    for label in &raw_labels {
        let len = label.len();

        if len == 0 {
            return Err(NameValidationError::TooShort {
                min: MIN_NAME_LENGTH,
                actual: 0,
            });
        }
        if len < MIN_NAME_LENGTH {
            return Err(NameValidationError::TooShort {
                min: MIN_NAME_LENGTH,
                actual: len,
            });
        }
        if len > MAX_NAME_LENGTH {
            return Err(NameValidationError::TooLong {
                max: MAX_NAME_LENGTH,
                actual: len,
            });
        }

        for (pos, ch) in label.chars().enumerate() {
            if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
                return Err(NameValidationError::InvalidCharacter { ch, position: pos });
            }
        }

        if label.starts_with('-') || label.ends_with('-') {
            return Err(NameValidationError::InvalidLabelBoundary);
        }
    }

    for label in &raw_labels {
        if reserved.contains(label) {
            return Err(NameValidationError::ReservedName(label.to_string()));
        }
    }

    Ok(ValidatedName {
        labels: raw_labels.into_iter().map(|s| s.to_string()).collect(),
        tld,
    })
}

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::string::ToString;
    use alloc::{format, vec};

    use super::*;
    use crate::constants::{MAX_NAME_LENGTH, MIN_NAME_LENGTH};
    use proptest::prelude::*;

    #[test]
    fn accepts_simple_valid_name() {
        let v = validate_name("timmy.xlm").unwrap();
        assert_eq!(v.labels, vec!["timmy"]);
        assert_eq!(v.tld, Tld::Xlm);
        assert!(!v.is_subdomain());
        assert_eq!(v.leaf_label(), "timmy");
        assert_eq!(v.base_label(), "timmy");
        assert_eq!(v.fqdn(), "timmy.xlm");
    }

    #[test]
    fn accepts_subdomain() {
        let v = validate_name("pay.timmy.xlm").unwrap();
        assert_eq!(v.labels, vec!["pay", "timmy"]);
        assert!(v.is_subdomain());
        assert_eq!(v.leaf_label(), "pay");
        assert_eq!(v.base_label(), "timmy");
        assert_eq!(v.fqdn(), "pay.timmy.xlm");
    }

    #[test]
    fn accepts_name_with_digits_and_hyphens() {
        let v = validate_name("abc-123.xlm").unwrap();
        assert_eq!(v.labels, vec!["abc-123"]);
    }

    #[test]
    fn accepts_max_depth_subdomain() {
        // a.b.c.xlm has 3 non-TLD labels = MAX_SUBDOMAIN_DEPTH
        // but each label must be ≥ MIN_NAME_LENGTH chars
        let v = validate_name("foo.bar.baz.xlm").unwrap();
        assert_eq!(v.labels, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn rejects_too_short_label() {
        let err = validate_name("ab.xlm").unwrap_err();
        assert_eq!(
            err,
            NameValidationError::TooShort {
                min: MIN_NAME_LENGTH,
                actual: 2
            }
        );
    }

    #[test]
    fn rejects_too_long_label() {
        let label = "a".repeat(MAX_NAME_LENGTH + 1);
        let name = format!("{label}.xlm");
        let err = validate_name(&name).unwrap_err();
        assert_eq!(
            err,
            NameValidationError::TooLong {
                max: MAX_NAME_LENGTH,
                actual: MAX_NAME_LENGTH + 1
            }
        );
    }

    #[test]
    fn rejects_uppercase_character() {
        let err = validate_name("Tim.xlm").unwrap_err();
        assert_eq!(
            err,
            NameValidationError::InvalidCharacter {
                ch: 'T',
                position: 0
            }
        );
    }

    #[test]
    fn rejects_emoji() {
        let err = validate_name("tim\u{1F980}.xlm").unwrap_err(); // 🦀
        assert_eq!(
            err,
            NameValidationError::InvalidCharacter {
                ch: '\u{1F980}',
                position: 3
            }
        );
    }

    #[test]
    fn rejects_leading_hyphen() {
        let err = validate_name("-tim.xlm").unwrap_err();
        assert_eq!(err, NameValidationError::InvalidLabelBoundary);
    }

    #[test]
    fn rejects_trailing_hyphen() {
        let err = validate_name("tim-.xlm").unwrap_err();
        assert_eq!(err, NameValidationError::InvalidLabelBoundary);
    }

    #[test]
    fn rejects_unsupported_tld() {
        let err = validate_name("timmy.eth").unwrap_err();
        assert_eq!(err, NameValidationError::UnsupportedTld("eth".to_string()));
    }

    #[test]
    fn rejects_missing_tld() {
        let err = validate_name("timmy").unwrap_err();
        assert_eq!(err, NameValidationError::MissingTld);
    }

    #[test]
    fn rejects_too_many_labels() {
        // a.b.c.d.xlm → 4 non-TLD labels > MAX_SUBDOMAIN_DEPTH (3)
        let err = validate_name("aaa.bbb.ccc.ddd.xlm").unwrap_err();
        assert_eq!(
            err,
            NameValidationError::TooManyLabels {
                max: MAX_SUBDOMAIN_DEPTH,
                actual: 4
            }
        );
    }

    #[test]
    fn rejects_reserved_name() {
        let err = validate_name_with_reserved("admin.xlm", &["admin"]).unwrap_err();
        assert_eq!(err, NameValidationError::ReservedName("admin".to_string()));
    }

    #[test]
    fn reserved_check_applies_to_each_label() {
        let err = validate_name_with_reserved("pay.root.xlm", &["root"]).unwrap_err();
        assert_eq!(err, NameValidationError::ReservedName("root".to_string()));
    }

    #[test]
    fn non_reserved_name_passes_reserved_check() {
        assert!(validate_name_with_reserved("alice.xlm", &["admin", "root"]).is_ok());
    }

    #[test]
    fn rejects_empty_label_before_tld() {
        let err = validate_name(".xlm").unwrap_err();
        assert!(matches!(
            err,
            NameValidationError::TooShort { actual: 0, .. }
        ));
    }

    #[test]
    fn rejects_empty_intermediate_label() {
        // "ti..xlm" → labels_part = "ti." → labels = ["ti", ""]
        // "ti" is too short (2 < 3) so TooShort fires first
        let err = validate_name("ti..xlm").unwrap_err();
        assert!(matches!(err, NameValidationError::TooShort { .. }));
    }

    #[test]
    fn error_display_is_human_readable() {
        assert!(NameValidationError::MissingTld.to_string().contains("TLD"));
        assert!(NameValidationError::InvalidCharacter {
            ch: 'A',
            position: 0
        }
        .to_string()
        .contains("position 0"));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        #[test]
        fn doesnt_panic_on_random_input(s in "\\PC*") {
            let _ = validate_name(&s);
        }

        #[test]
        fn accepts_strictly_valid_single_label_names(label in "[a-z0-9]([a-z0-9-]*[a-z0-9])?") {
            if label.len() >= MIN_NAME_LENGTH && label.len() <= MAX_NAME_LENGTH {
                let name = format!("{label}.xlm");
                prop_assert!(validate_name(&name).is_ok());
            }
        }

        #[test]
        fn rejects_uppercase_in_any_position(label in "[a-zA-Z0-9-]*[A-Z][a-zA-Z0-9-]*") {
            if label.len() >= MIN_NAME_LENGTH && label.len() <= MAX_NAME_LENGTH {
                let name = format!("{label}.xlm");
                let result = validate_name(&name);
                let is_invalid = matches!(result,
                    Err(NameValidationError::InvalidCharacter { .. })
                    | Err(NameValidationError::InvalidLabelBoundary)
                );
                prop_assert!(is_invalid);
            }
        }
    }
}
