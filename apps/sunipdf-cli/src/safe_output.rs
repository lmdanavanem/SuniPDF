pub fn terminal_text(text: &str) -> String {
    text.chars()
        .filter(|&character| !character.is_control() || matches!(character, '\n' | '\t' | '\u{c}'))
        .collect()
}

pub fn escape_c1(serialized_json: &str) -> String {
    let mut output = String::with_capacity(serialized_json.len());
    for character in serialized_json.chars() {
        if ('\u{7f}'..='\u{9f}').contains(&character) {
            use std::fmt::Write;
            write!(output, "\\u{:04x}", character as u32).expect("string output");
        } else {
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_controls_are_removed_without_losing_layout_or_unicode() {
        assert_eq!(
            terminal_text("A\u{1b}]8;\u{7}B\u{9b}C\n\t\u{c}été中文"),
            "A]8;BC\n\t\u{c}été中文"
        );
        for character in (0..=0x9f)
            .filter_map(char::from_u32)
            .filter(|c| c.is_control())
        {
            let output = terminal_text(&character.to_string());
            assert_eq!(
                !output.is_empty(),
                matches!(character, '\n' | '\t' | '\u{c}')
            );
        }
    }

    #[test]
    fn json_controls_are_escaped_and_round_trip_without_data_loss() {
        let value = serde_json::json!({"title":"\u{1b}\u{7f}\u{80}\u{9b}\u{9f}été"});
        for serialized in [
            serde_json::to_string(&value).unwrap(),
            serde_json::to_string_pretty(&value).unwrap(),
        ] {
            let output = escape_c1(&serialized);
            assert!(
                !output.chars().any(char::is_control)
                    || output.chars().filter(|c| c.is_control()).all(|c| c == '\n')
            );
            assert!(output.contains("\\u009b"));
            assert!(output.contains("\\u007f"));
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&output).unwrap(),
                value
            );
        }
    }
}
