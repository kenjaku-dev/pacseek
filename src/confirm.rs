use std::io::{BufRead, Write};

/// Returns true when an interactive confirmation prompt is required:
/// noconfirm flag/env is off AND we have a real TTY to ask on.
pub fn should_confirm(noconfirm: bool, stdin_tty: bool) -> bool {
    !noconfirm && stdin_tty
}

/// Ask `prompt` on `output`, read one line from `input`.
/// Accepts `y`/`yes` (case-insensitive); anything else — including EOF —
/// means no. Never panics on IO errors: failure to ask is a refusal.
pub fn confirm_prompt(prompt: &str, input: &mut dyn BufRead, output: &mut dyn Write) -> bool {
    let _ = writeln!(output, "{} [y/N]", prompt);
    let _ = output.flush();
    let mut line = String::new();
    match input.read_line(&mut line) {
        Ok(_) => matches!(line.trim().to_lowercase().as_str(), "y" | "yes"),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn ask(reply: &str) -> bool {
        let mut input = Cursor::new(reply.as_bytes().to_vec());
        let mut output = Vec::new();
        confirm_prompt("Remove 'foo'?", &mut input, &mut output)
    }

    #[test]
    fn yes_variants_confirm() {
        for reply in ["y\n", "Y\n", "yes\n", "YES\n", "  y  \n"] {
            assert!(ask(reply), "reply {reply:?} should confirm");
        }
    }

    #[test]
    fn anything_else_refuses() {
        for reply in ["n\n", "no\n", "\n", "yess\n", "foo\n", ""] {
            assert!(!ask(reply), "reply {reply:?} should refuse");
        }
    }

    #[test]
    fn should_confirm_matrix() {
        assert!(should_confirm(false, true));
        assert!(!should_confirm(true, true));
        assert!(!should_confirm(false, false));
        assert!(!should_confirm(true, false));
    }
}
