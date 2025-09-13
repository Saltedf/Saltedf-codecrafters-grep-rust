use std::collections::HashSet;
use std::env;
use std::io;
use std::process;

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    if pattern.chars().count() == 1 {
        return input_line.contains(pattern);
    } else {
        match pattern {
            r"\d" => return input_line.chars().any(|ch| ch.is_digit(10)),
            r"\w" => {
                return input_line
                    .chars()
                    .any(|ch| ch.is_alphanumeric() || ch == '_')
            }
            _ => {
                if pattern.starts_with('[') && pattern.ends_with(']') {
                    let set: HashSet<char> = pattern[1..pattern.len() - 1].chars().collect();
                    return input_line.chars().any(|ch| set.contains(&ch));
                }
                return false;
            }
        }
    }
    panic!("Unhandled pattern: {}", pattern);
}

// Usage: echo <input_text> | your_program.sh -E <pattern>
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    if env::args().nth(1).unwrap() != "-E" {
        println!("Expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();

    io::stdin().read_line(&mut input_line).unwrap();

    // Uncomment this block to pass the first stage
    if match_pattern(&input_line, &pattern) {
        process::exit(0)
    } else {
        process::exit(1)
    }
}
