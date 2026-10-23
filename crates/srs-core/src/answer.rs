//! Checking typed answers.

use crate::model::{CardTemplate, Note};

/// Lowercase, trim, and collapse internal whitespace.
pub fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// True if `input` matches any accepted value, ignoring case and spacing.
pub fn matches_text(input: &str, accepted: &[String]) -> bool {
    let input = normalize(input);
    !input.is_empty() && accepted.iter().any(|a| normalize(a) == input)
}

/// True if `input` parses as a number within `tolerance` of `expected`. A trailing unit such as
/// "Da" is ignored.
pub fn matches_number(input: &str, expected: f64, tolerance: f64) -> bool {
    let number = input.trim().trim_end_matches(|c: char| c.is_alphabetic()).trim();
    number
        .parse::<f64>()
        .is_ok_and(|v| (v - expected).abs() <= tolerance)
}

/// Check a typed answer for a card made from `template` and `note`.
pub fn check(template: &CardTemplate, note: &Note, input: &str) -> bool {
    if let Some(tolerance) = template.tolerance {
        return template
            .answer
            .iter()
            .filter_map(|f| note.get(f)?.as_f64())
            .any(|expected| matches_number(input, expected, tolerance));
    }
    let accepted: Vec<String> = template
        .accepted_fields()
        .iter()
        .filter_map(|f| note.text(f))
        .collect();
    matches_text(input, &accepted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::SetFile;

    fn seed() -> SetFile {
        SetFile::from_yaml(include_str!("../../../seed/amino-acids/amino-acids.yaml")).unwrap()
    }

    #[test]
    fn structure_to_name_accepts_name_or_either_code() {
        let file = seed();
        let t = file.note_type.template("structure-to-name").unwrap();
        let ile = file.notes.iter().find(|n| n.id == "aa-ile").unwrap();
        for ok in ["Isoleucine", "  isoleucine ", "ILE", "i"] {
            assert!(check(t, ile, ok), "{ok:?} should be accepted");
        }
        for bad in ["leucine", "L", "", "iso leucine"] {
            assert!(!check(t, ile, bad), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn multi_word_names_ignore_spacing() {
        let file = seed();
        let t = file.note_type.template("structure-to-name").unwrap();
        let asp = file.notes.iter().find(|n| n.id == "aa-asp").unwrap();
        assert!(check(t, asp, "aspartic   ACID"));
    }

    #[test]
    fn residue_mass_uses_tolerance() {
        let file = seed();
        let t = file.note_type.template("name-to-residue-mass").unwrap();
        let trp = file.notes.iter().find(|n| n.id == "aa-trp").unwrap();
        assert!(check(t, trp, "186.079"));
        assert!(check(t, trp, "186.08 Da"));
        assert!(!check(t, trp, "186.1"));
        assert!(!check(t, trp, "heavy"));
    }
}
