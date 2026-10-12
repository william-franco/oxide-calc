//! UI-agnostic calculator state. The Slint bridge only forwards user intents here and renders the result.

use crate::engine::{AngleMode, CalcError, evaluate, format_number};

pub const HISTORY_LIMIT: usize = 100;

const BINARY_OPS: [char; 5] = ['+', '−', '×', '÷', '^'];
const FUNCTION_TOKENS: [&str; 10] = ["asin(", "acos(", "atan(", "sin(", "cos(", "tan(", "ln(", "log(", "√(", "∛("];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub expression: String,
    pub result: String,
}

#[derive(Debug, Default)]
pub struct CalcViewModel {
    expression: String,
    preview: String,
    error: Option<CalcError>,
    just_evaluated: bool,
    angle: AngleMode,
    history: Vec<HistoryEntry>,
}

impl CalcViewModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }

    /// Live result while typing, or the error message after a failed `=`.
    pub fn secondary_text(&self) -> String {
        match self.error {
            Some(error) => error.to_string(),
            None => self.preview.clone(),
        }
    }

    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn angle(&self) -> AngleMode {
        self.angle
    }

    /// Newest first.
    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    /// Appends a token produced by a key: digits, `.`, operators (`+ − × ÷ ^`), postfix (`! %`),
    /// parentheses, constants (`π e`), functions (`sin(` …) or composite tokens such as `^2`.
    pub fn input(&mut self, token: &str) {
        let Some(first) = token.chars().next() else {
            return;
        };
        self.error = None;

        if self.just_evaluated {
            self.just_evaluated = false;
            if starts_operand(first) {
                self.expression.clear();
            }
        }

        match first {
            '.' | ',' => self.push_decimal_point(),
            c if BINARY_OPS.contains(&c) => self.push_operator(c, &token[c.len_utf8()..]),
            '!' | '%' => {
                if ends_with_operand(&self.expression) {
                    self.expression.push_str(token);
                }
            }
            ')' => {
                if ends_with_operand(&self.expression) && open_parens(&self.expression) > 0 {
                    self.expression.push(')');
                }
            }
            _ => {
                // Composite tokens such as `10^` must not merge into the previous number.
                if first.is_ascii_digit() && token.len() > 1 && ends_with_operand(&self.expression) {
                    self.expression.push('×');
                }
                self.expression.push_str(token);
            }
        }
        self.refresh_preview();
    }

    /// Maps physical keyboard text to calculator actions. Returns `false` for unhandled keys.
    pub fn handle_key(&mut self, text: &str) -> bool {
        match text {
            "\n" | "\r" | "=" => self.equals(),
            "\u{8}" => self.backspace(),
            "\u{1b}" | "\u{7f}" => self.clear(),
            "*" | "x" | "X" => self.input("×"),
            "/" | ":" => self.input("÷"),
            "-" => self.input("−"),
            "," | "." => self.input("."),
            "s" => self.input("sin("),
            "c" => self.input("cos("),
            "t" => self.input("tan("),
            "l" => self.input("ln("),
            "g" => self.input("log("),
            "r" => self.input("√("),
            "p" => self.input("π"),
            "e" => self.input("e"),
            "+" | "^" | "%" | "!" | "(" | ")" | "π" => self.input(text),
            _ if text.len() == 1 && text.as_bytes()[0].is_ascii_digit() => self.input(text),
            _ => return false,
        }
        true
    }

    pub fn backspace(&mut self) {
        self.error = None;
        self.just_evaluated = false;
        match FUNCTION_TOKENS.iter().find(|f| self.expression.ends_with(*f)) {
            Some(func) => self.expression.truncate(self.expression.len() - func.len()),
            None => {
                self.expression.pop();
            }
        }
        self.refresh_preview();
    }

    pub fn clear(&mut self) {
        self.expression.clear();
        self.preview.clear();
        self.error = None;
        self.just_evaluated = false;
    }

    /// Negates the last number, or the whole expression when it doesn't end with a number.
    pub fn toggle_sign(&mut self) {
        self.error = None;
        self.just_evaluated = false;
        let expr = &mut self.expression;

        let number_start = expr
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_ascii_digit() || *c == '.')
            .last()
            .map(|(i, _)| i);

        match number_start {
            Some(start) => {
                let before = expr[..start].chars().next_back();
                let before_before = expr[..start].chars().rev().nth(1);
                match before {
                    Some('−') if before_before.is_none_or(|c| c == '(' || BINARY_OPS.contains(&c)) => {
                        expr.replace_range(start - '−'.len_utf8()..start, "");
                    }
                    Some('−') => expr.replace_range(start - '−'.len_utf8()..start, "+"),
                    Some('+') => expr.replace_range(start - 1..start, "−"),
                    _ => expr.insert(start, '−'),
                }
            }
            None if expr.is_empty() => expr.push('−'),
            None => {
                if let Some(inner) = expr.strip_prefix("−(").and_then(|rest| rest.strip_suffix(')'))
                    && balanced(inner)
                {
                    *expr = inner.to_string();
                } else {
                    *expr = format!("−({expr})");
                }
            }
        }
        self.refresh_preview();
    }

    pub fn equals(&mut self) {
        if self.expression.is_empty() || self.just_evaluated {
            return;
        }
        match evaluate(&self.expression, self.angle) {
            Ok(value) => {
                let result = display_number(value);
                let mut expression = std::mem::take(&mut self.expression);
                expression.push_str(&")".repeat(open_parens(&expression)));
                self.history.insert(0, HistoryEntry { expression, result: result.clone() });
                self.history.truncate(HISTORY_LIMIT);
                self.expression = result;
                self.preview.clear();
                self.error = None;
                self.just_evaluated = true;
            }
            Err(error) => {
                self.error = Some(error);
                self.preview.clear();
            }
        }
    }

    /// Loads a past expression so it can be edited or re-evaluated.
    pub fn reuse_history(&mut self, index: usize) {
        if let Some(entry) = self.history.get(index) {
            self.expression = entry.expression.clone();
            self.error = None;
            self.just_evaluated = false;
            self.refresh_preview();
        }
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn toggle_angle(&mut self) {
        self.angle = match self.angle {
            AngleMode::Degrees => AngleMode::Radians,
            AngleMode::Radians => AngleMode::Degrees,
        };
        if !self.just_evaluated {
            self.refresh_preview();
        }
    }

    fn push_decimal_point(&mut self) {
        let current_number: String = self
            .expression
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if current_number.contains('.') {
            return;
        }
        if current_number.is_empty() {
            self.expression.push('0');
        }
        self.expression.push('.');
    }

    fn push_operator(&mut self, op: char, suffix: &str) {
        let last = self.expression.chars().next_back();
        match last {
            None if op == '−' => self.expression.push(op),
            None => {
                self.expression.push('0');
                self.expression.push(op);
            }
            Some('(') if op == '−' => self.expression.push(op),
            Some('(') => return,
            Some(prev) if BINARY_OPS.contains(&prev) => {
                if op == '−' && prev != '−' && prev != '+' {
                    self.expression.push(op);
                } else {
                    while self.expression.chars().next_back().is_some_and(|c| BINARY_OPS.contains(&c)) {
                        self.expression.pop();
                    }
                    if self.expression.is_empty() || self.expression.ends_with('(') {
                        if op == '−' {
                            self.expression.push(op);
                        }
                        return;
                    }
                    self.expression.push(op);
                }
            }
            Some('.') => {
                self.expression.pop();
                self.expression.push(op);
            }
            Some(_) => self.expression.push(op),
        }
        self.expression.push_str(suffix);
    }

    fn refresh_preview(&mut self) {
        self.preview.clear();
        let unsigned = self.expression.strip_prefix('−').unwrap_or(&self.expression);
        let is_plain_number = unsigned.chars().all(|c| c.is_ascii_digit() || c == '.');
        if is_plain_number {
            return;
        }
        if let Ok(value) = evaluate(&self.expression, self.angle) {
            self.preview = display_number(value);
        }
    }
}

fn display_number(value: f64) -> String {
    format_number(value).replace('-', "−")
}

fn starts_operand(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '.' | ',' | '(' | 'π' | 'e' | '√' | '∛') || c.is_ascii_lowercase()
}

fn ends_with_operand(expr: &str) -> bool {
    expr.chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_digit() || matches!(c, ')' | 'π' | 'e' | '!' | '%'))
}

fn open_parens(expr: &str) -> usize {
    let opened = expr.chars().filter(|c| *c == '(').count();
    let closed = expr.chars().filter(|c| *c == ')').count();
    opened.saturating_sub(closed)
}

fn balanced(expr: &str) -> bool {
    let mut depth = 0_i32;
    for c in expr.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(tokens: &[&str]) -> CalcViewModel {
        let mut vm = CalcViewModel::new();
        for token in tokens {
            vm.input(token);
        }
        vm
    }

    #[test]
    fn live_preview_and_equals() {
        let mut vm = typed(&["1", "2", "+", "3"]);
        assert_eq!(vm.expression(), "12+3");
        assert_eq!(vm.secondary_text(), "15");
        vm.equals();
        assert_eq!(vm.expression(), "15");
        assert_eq!(vm.history()[0], HistoryEntry { expression: "12+3".into(), result: "15".into() });
    }

    #[test]
    fn operator_replacement_and_negative_operands() {
        let vm = typed(&["5", "+", "×"]);
        assert_eq!(vm.expression(), "5×");
        let vm = typed(&["5", "×", "−", "2"]);
        assert_eq!(vm.expression(), "5×−2");
        assert_eq!(vm.secondary_text(), "−10");
        let vm = typed(&["5", "×", "−", "+"]);
        assert_eq!(vm.expression(), "5+");
        let vm = typed(&["×"]);
        assert_eq!(vm.expression(), "0×");
    }

    #[test]
    fn decimal_point_rules() {
        let vm = typed(&[".", "5", ".", "2"]);
        assert_eq!(vm.expression(), "0.52");
    }

    #[test]
    fn continues_or_restarts_after_result() {
        let mut vm = typed(&["2", "+", "2"]);
        vm.equals();
        vm.input("×");
        vm.input("3");
        assert_eq!(vm.expression(), "4×3");

        let mut vm = typed(&["2", "+", "2"]);
        vm.equals();
        vm.input("7");
        assert_eq!(vm.expression(), "7");
    }

    #[test]
    fn backspace_removes_function_tokens() {
        let mut vm = typed(&["2", "+", "sin("]);
        vm.backspace();
        assert_eq!(vm.expression(), "2+");
        vm.backspace();
        assert_eq!(vm.expression(), "2");
    }

    #[test]
    fn toggle_sign() {
        let mut vm = typed(&["5"]);
        vm.toggle_sign();
        assert_eq!(vm.expression(), "−5");
        vm.toggle_sign();
        assert_eq!(vm.expression(), "5");

        let mut vm = typed(&["3", "+", "4"]);
        vm.toggle_sign();
        assert_eq!(vm.expression(), "3−4");
        vm.toggle_sign();
        assert_eq!(vm.expression(), "3+4");

        let mut vm = typed(&["(", "1", "+", "2", ")"]);
        vm.toggle_sign();
        assert_eq!(vm.expression(), "−((1+2))");
        vm.toggle_sign();
        assert_eq!(vm.expression(), "(1+2)");
    }

    #[test]
    fn errors_are_reported_and_cleared() {
        let mut vm = typed(&["1", "÷", "0"]);
        vm.equals();
        assert!(vm.has_error());
        assert_eq!(vm.secondary_text(), "Divisão por zero");
        assert_eq!(vm.expression(), "1÷0");
        assert!(vm.history().is_empty());
        vm.backspace();
        assert!(!vm.has_error());
    }

    #[test]
    fn history_reuse_clear_and_limit() {
        let mut vm = CalcViewModel::new();
        for i in 0..(HISTORY_LIMIT + 5) {
            vm.clear();
            vm.input(&i.to_string());
            vm.input("+");
            vm.input("1");
            vm.equals();
        }
        assert_eq!(vm.history().len(), HISTORY_LIMIT);
        vm.reuse_history(0);
        assert_eq!(vm.expression(), format!("{}+1", HISTORY_LIMIT + 4));
        vm.reuse_history(1_000);
        vm.clear_history();
        assert!(vm.history().is_empty());
    }

    #[test]
    fn closes_parentheses_in_history_and_respects_angle() {
        let mut vm = typed(&["sin(", "9", "0"]);
        assert_eq!(vm.secondary_text(), "1");
        vm.toggle_angle();
        assert_eq!(vm.angle(), AngleMode::Radians);
        vm.equals();
        assert_eq!(vm.history()[0].expression, "sin(90)");
    }

    #[test]
    fn composite_digit_tokens_insert_multiplication() {
        let vm = typed(&["5", "10^", "2"]);
        assert_eq!(vm.expression(), "5×10^2");
        assert_eq!(vm.secondary_text(), "500");
    }

    #[test]
    fn keyboard_mapping() {
        let mut vm = CalcViewModel::new();
        for key in ["2", "*", "(", "3", "-", "1", ")"] {
            assert!(vm.handle_key(key));
        }
        assert_eq!(vm.expression(), "2×(3−1)");
        assert!(vm.handle_key("\n"));
        assert_eq!(vm.expression(), "4");
        assert!(!vm.handle_key("q"));
        assert!(!vm.handle_key("\u{f700}"));
        assert!(vm.handle_key("\u{1b}"));
        assert_eq!(vm.expression(), "");
    }

    #[test]
    fn ignores_invalid_postfix_and_parens() {
        let vm = typed(&["!", ")", "%"]);
        assert_eq!(vm.expression(), "");
        let vm = typed(&["2", ")"]);
        assert_eq!(vm.expression(), "2");
    }
}
