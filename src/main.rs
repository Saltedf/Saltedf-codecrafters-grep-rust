mod reg;

use anyhow::Error;
use reg::*;
use std::char;
use std::collections::HashSet;
use std::env;
use std::io;
use std::iter::Peekable;
use std::process;
use std::str::Chars;

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    if pattern.chars().count() == 1 {
        return input_line.contains(pattern);
    } else {
        match pattern {
            r"\d" => return input_line.chars().any(|ch| ch.is_digit(10)), // any  foreach 用法
            r"\w" => {
                return input_line
                    .chars()
                    .any(|ch| ch.is_alphanumeric() || ch == '_')
            }
            _ => {
                if pattern.starts_with('[') && pattern.ends_with(']') {
                    // str 自身就有contains方法  collect自动创造容器
                    let group = &pattern[1..pattern.len() - 1];
                    if pattern.starts_with("[^") {
                        return input_line.chars().any(|ch| !group.contains(ch));
                    } else {
                        return input_line.chars().any(|ch| group.contains(ch));
                    }
                }
                return false;
            }
        }
    }
    panic!("Unhandled pattern: {}", pattern);
}

// Usage: echo <input_text> | your_program.sh -E <pattern>
fn main() -> Result<(), Error> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    // eprintln!("Logs from your program will appear here!");

    if env::args().nth(1).unwrap() != "-E" {
        println!("Expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();

    io::stdin().read_line(&mut input_line).unwrap();

    let re = Reg::new(&pattern)?;
    // Uncomment this block to pass the first stage
    if re.is_match(input_line.as_str()) {
        println!("{input_line}");
        process::exit(0)
    } else {
        process::exit(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*; // 导入父模块的所有内容
}
