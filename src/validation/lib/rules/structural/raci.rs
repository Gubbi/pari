use crate::entity::types::Raci;
use crate::error::primitive::PrimitiveError;

/// `Raci.responsible` must be non-empty.
pub fn raci_structural(value: &Raci) -> Vec<PrimitiveError> {
    if value.responsible.is_empty() {
        vec![PrimitiveError::empty_required_value(
            "responsible must not be empty",
            Some(".responsible"),
            "raci_structural",
        )]
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{entities::role::Role, entity::EntityRef};

    #[test]
    fn valid_when_responsible_non_empty() {
        let raci = crate::entity::types::Raci {
            responsible: vec![EntityRef::<Role>::new("eng-lead")],
            accountable: EntityRef::new("pm"),
            consulted: None,
            informed: None,
        };
        assert!(raci_structural(&raci).is_empty());
    }

    #[test]
    fn rejects_empty_responsible() {
        let raci = crate::entity::types::Raci {
            responsible: vec![],
            accountable: EntityRef::<Role>::new("pm"),
            consulted: None,
            informed: None,
        };
        let v = raci_structural(&raci);
        assert!(!v.is_empty());
        match &v[0] {
            PrimitiveError::EmptyRequiredValue { sub_path, .. } => {
                assert!(sub_path.as_ref().map(|p| p.contains("responsible")).unwrap_or(false));
            }
            other => panic!("expected EmptyRequiredValue, got {other:?}"),
        }
    }
}
