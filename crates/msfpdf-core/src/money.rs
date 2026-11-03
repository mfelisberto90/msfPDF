use serde::{Deserialize, Serialize};

/// Supported ISO 4217 currencies formatted strictly from integer minor units
/// (`pence`, `cents`, `centavos`, `kopecks`, `fils`, `halalas`, `yen`).
/// Zero binary floating-point operations are permitted in monetary formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    #[default]
    GBP,
    EUR,
    USD,
    BRL,
    JPY,
    RUB,
    AED,
    SAR,
}

fn group_integer_digits(digits: &str, sep: char) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (digits.len() - idx).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(ch);
    }
    out
}

/// Formats signed integer minor units (`125_050` -> `£1,250.50`, `1 250,50 ₽`, `AED 1,250.50`) deterministically.
pub fn format_money_minor(minor_units: i64, currency: Currency) -> String {
    let negative = minor_units < 0;
    let abs: u128 = minor_units.unsigned_abs() as u128;
    let sign = if negative { "-" } else { "" };

    match currency {
        Currency::JPY => {
            let grouped = group_integer_digits(&abs.to_string(), ',');
            format!("{sign}¥{grouped}")
        }
        Currency::GBP => {
            let major = abs / 100;
            let minor = abs % 100;
            let grouped = group_integer_digits(&major.to_string(), ',');
            format!("{sign}£{grouped}.{minor:02}")
        }
        Currency::USD => {
            let major = abs / 100;
            let minor = abs % 100;
            let grouped = group_integer_digits(&major.to_string(), ',');
            format!("{sign}${grouped}.{minor:02}")
        }
        Currency::EUR => {
            let major = abs / 100;
            let minor = abs % 100;
            let grouped = group_integer_digits(&major.to_string(), ',');
            format!("{sign}€{grouped}.{minor:02}")
        }
        Currency::BRL => {
            let major = abs / 100;
            let minor = abs % 100;
            let grouped = group_integer_digits(&major.to_string(), '.');
            format!("{sign}R$ {grouped},{minor:02}")
        }
        Currency::RUB => {
            let major = abs / 100;
            let minor = abs % 100;
            let grouped = group_integer_digits(&major.to_string(), ' ');
            format!("{sign}{grouped},{minor:02} ₽")
        }
        Currency::AED => {
            let major = abs / 100;
            let minor = abs % 100;
            let grouped = group_integer_digits(&major.to_string(), ',');
            format!("{sign}AED {grouped}.{minor:02}")
        }
        Currency::SAR => {
            let major = abs / 100;
            let minor = abs % 100;
            let grouped = group_integer_digits(&major.to_string(), ',');
            format!("{sign}SAR {grouped}.{minor:02}")
        }
    }
}
