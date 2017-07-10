pub fn parse_text(input: &[&str]) -> Result<Vec<String>, String> {
    let input = input.join(" ");
    let mut words = vec![String::new()];
    let mut single_quote = false;
    let mut double_quote = false;
    let mut escape = false;
    for c in input.chars() {
        match c {
            '\'' => {
                if escape || double_quote {
                    words.last_mut().unwrap().push('\'');
                    if escape {
                        escape = false;
                    }
                } else {
                    single_quote = !single_quote;
                }
            }
            '"' => {
                if escape || single_quote {
                    words.last_mut().unwrap().push('\"');
                    if escape {
                        escape = false;
                    }
                } else {
                    double_quote = !double_quote;
                }
            }
            '\\' => {
                if escape {
                    words.last_mut().unwrap().push('\"');
                    escape = false;
                } else {
                    escape = true;
                }
            }
            ' ' => {
                if escape {
                    return Err("Invalid escape character ' '".to_owned());
                }
                if single_quote || double_quote {
                    words.last_mut().unwrap().push(' ');
                    escape = false;
                } else {
                    words.push(String::new());
                }
            }
            x => {
                if escape {
                    return Err(format!("Invalid escape character {}", x));
                }
                words.last_mut().unwrap().push(x);
            }
        }
    }
    if single_quote || double_quote {
        return Err("Unbalanced quotes".to_owned());
    }
    Ok(words)
}
mod test {
    #[test]
    fn parse_works() {
        use parse::parse_text;
        assert_eq!(
            parse_text(&["echo", "\"hello world\""]).unwrap().as_slice(),
            &[String::from("echo"), String::from("hello world")]
        );
    }
}
