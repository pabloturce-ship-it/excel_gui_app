//! Разбор последней ячейки строки: 10 цифр лицевого счёта и ФИО сразу после них.

use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    pub personal_account: String,
    pub full_name: String,
}

fn account_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(\d{10})").expect("регулярное выражение должно быть корректным"))
}

/// Фамилия (может быть через дефис), затем инициалы с точками или без: `И.О.` / `И И`
fn fio_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?x)
            ^[\s,;:/\\]*(?:[-–—]\s*)?
            (
                [А-ЯЁA-Z][а-яёa-z]+
                (?:\s*-\s*[А-ЯЁA-Z][а-яёa-z]+)*
            )
            \s+
            ([А-ЯЁA-Z])(?:\.\s*|\s+)
            ([А-ЯЁA-Z])\.?
            ",
        )
        .expect("регулярное выражение должно быть корректным")
    })
}

fn normalize_surname(raw: &str) -> String {
    raw.split('-')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn extract_fio(after_account: &str) -> String {
    match fio_regex().captures(after_account) {
        Some(caps) => {
            let surname = normalize_surname(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
            let name_initial = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let patronymic_initial = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            format!("{surname} {name_initial}.{patronymic_initial}.")
        }
        None => String::new(),
    }
}

/// Ищет в тексте номер из 10 цифр и ФИО сразу после него.
///
/// ФИО берётся в виде `Иванов И.И.`, `Иванов И И` или `Петрова-Сидорова П.С.`.
/// В результат инициалы всегда записываются с точками. Текст после инициалов отбрасывается.
pub fn extract_account_and_name(text: &str) -> Option<Extracted> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let found = account_regex().find(text)?;
    let personal_account = found.as_str().to_string();
    let after = &text[found.end()..];
    let full_name = extract_fio(after);

    Some(Extracted {
        personal_account,
        full_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_account_and_name() {
        let got = extract_account_and_name("1234567890 Иванов И.И.").unwrap();
        assert_eq!(got.personal_account, "1234567890");
        assert_eq!(got.full_name, "Иванов И.И.");
    }

    #[test]
    fn extracts_when_prefix_exists() {
        let got = extract_account_and_name("л/с: 9876543210 Петров П.П.").unwrap();
        assert_eq!(got.personal_account, "9876543210");
        assert_eq!(got.full_name, "Петров П.П.");
    }

    #[test]
    fn ignores_text_after_initials() {
        let got = extract_account_and_name(
            "1234567890 Иванов И.И. г. Москва ул. Ленина д. 1 тел. 89001234567",
        )
        .unwrap();
        assert_eq!(got.personal_account, "1234567890");
        assert_eq!(got.full_name, "Иванов И.И.");
    }

    #[test]
    fn accepts_extra_spaces_between_parts() {
        let got = extract_account_and_name("1234567890 Сидоров   С.  С.").unwrap();
        assert_eq!(got.full_name, "Сидоров С.С.");
    }

    #[test]
    fn extracts_hyphenated_surname() {
        let got = extract_account_and_name("1234567890 Петрова-Сидорова П.С.").unwrap();
        assert_eq!(got.full_name, "Петрова-Сидорова П.С.");
    }

    #[test]
    fn extracts_hyphenated_surname_with_spaces_around_hyphen() {
        let got = extract_account_and_name("1234567890 Римский - Корсаков Р.К. лишние данные")
            .unwrap();
        assert_eq!(got.full_name, "Римский-Корсаков Р.К.");
    }

    #[test]
    fn extracts_initials_without_dots() {
        let got = extract_account_and_name("1234567890 Иванов И И г. Москва").unwrap();
        assert_eq!(got.personal_account, "1234567890");
        assert_eq!(got.full_name, "Иванов И.И.");
    }

    #[test]
    fn allows_account_without_name() {
        let got = extract_account_and_name("1234567890").unwrap();
        assert_eq!(got.personal_account, "1234567890");
        assert!(got.full_name.is_empty());
    }

    #[test]
    fn returns_none_without_ten_digits() {
        assert!(extract_account_and_name("Иванов И.И.").is_none());
        assert!(extract_account_and_name("").is_none());
        assert!(extract_account_and_name("12345").is_none());
    }
}
