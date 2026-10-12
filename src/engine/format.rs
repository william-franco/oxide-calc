const SIGNIFICANT_DIGITS: i32 = 12;
const SCI_UPPER: f64 = 1e12;
const SCI_LOWER: f64 = 1e-9;

/// Formats a result with up to 12 significant digits, switching to scientific notation
/// (`1.5e-12`, `2.5e20`) at the extremes. The output is valid calculator input again.
pub fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return String::from("—");
    }
    if value == 0.0 {
        return String::from("0");
    }

    let magnitude = value.abs();
    if !(SCI_LOWER..SCI_UPPER).contains(&magnitude) {
        let text = format!("{:.*e}", (SIGNIFICANT_DIGITS - 3) as usize, value);
        return match text.split_once('e') {
            Some((mantissa, exponent)) => format!("{}e{}", trim_zeros(mantissa), exponent),
            None => text,
        };
    }

    let integer_digits = magnitude.log10().floor() as i32 + 1;
    let decimals = (SIGNIFICANT_DIGITS - integer_digits).clamp(0, 15) as usize;
    let text = trim_zeros(&format!("{value:.decimals$}")).to_string();
    if text == "-0" { String::from("0") } else { text }
}

fn trim_zeros(text: &str) -> &str {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text
    }
}
