/// A single violation returned by a rule function.
/// `sub_path = None` means the violation is at the field itself.
/// `sub_path = Some("[0].role")` means a nested sub-field of the field.
pub struct RuleViolation {
    pub sub_path: Option<String>,
    pub message: String,
}

impl RuleViolation {
    pub fn field(message: impl Into<String>) -> Self {
        Self {
            sub_path: None,
            message: message.into(),
        }
    }

    pub fn sub(sub_path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            sub_path: Some(sub_path.into()),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_violation_field_has_no_sub_path() {
        let v = RuleViolation::field("bad value");
        assert!(v.sub_path.is_none());
        assert_eq!(v.message, "bad value");
    }

    #[test]
    fn rule_violation_sub_has_sub_path() {
        let v = RuleViolation::sub("[0].role", "not found");
        assert_eq!(v.sub_path.as_deref(), Some("[0].role"));
    }
}
