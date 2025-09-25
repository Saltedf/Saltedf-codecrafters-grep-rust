use std::char;
use std::collections::HashSet;
use std::env;
use std::io;
use std::iter::Peekable;
use std::process;
use std::str::Chars;

pub struct CompiledPattern {
    instrs: Vec<Inst>,
}

struct Inst {
    matcher: CharClass,
    quantifier: Quantifier,
}

#[derive(Debug, PartialEq)]
enum CharClass {
    Literal(char),
    Digit,
    Word,
    PostiveGroup(HashSet<char>),
    NegativeGroup(HashSet<char>),
}

#[derive(Debug, PartialEq)]
enum Quantifier {
    One,
    ZeroOrMore,
    OneOrMore,
}

impl CompiledPattern {
    fn parse_charclass(stream: &mut Peekable<Chars>) -> Option<CharClass> {
        match stream.peek().copied()? {
            '\\' => {
                stream.next();
                match stream.next() {
                    Some('w') => Some(CharClass::Word),
                    Some('d') => Some(CharClass::Digit),
                    _ => panic!("非法的转义字符！"),
                }
            }
            '[' => {
                stream.next();
                let mut group = HashSet::new();
                let is_negitive = stream.next_if(|c| *c == '^').is_some();
                loop {
                    if let Some(ch) = stream.next_if(|c| *c != ']') {
                        group.insert(ch);
                    } else {
                        stream.next();
                        break;
                    }
                }
                let res = if is_negitive {
                    CharClass::NegativeGroup(group)
                } else {
                    CharClass::PostiveGroup(group)
                };
                Some(res)
            }
            ch => {
                stream.next();
                Some(CharClass::Literal(ch))
            }
        }
    }

    pub fn new(pattern: &str) -> Self {
        let instrs = Vec::new();
        let mut stream = pattern.chars().peekable();

        loop {
            if let Some(c) = Self::parse_charclass(&mut stream) {
                let mut quantifier = if let Some(q) = Self::parse_quantifier(&mut stream) {
                    q
                } else {
                    Quantifier::One
                };
            } else {
                break;
            }
        }
        Self { instrs }
    }

    pub fn instrs(&self) -> &Vec<Inst> {
        return &self.instrs;
    }

    fn parse_quantifier(stream: &mut Peekable<Chars>) -> Option<Quantifier> {
        let quantifier = match stream.next_if(|ch| *ch == '+' || *ch == '*') {
            Some('*') => Quantifier::ZeroOrMore,
            Some('+') => Quantifier::OneOrMore,
            _ => Quantifier::One,
        };
        Some(quantifier)
    }
}

pub struct Parser<'a> {
    input: Peekable<Chars<'a>>,
    text: &'a str,
}

impl<'a> Parser<'a> {
    pub fn new(input_line: &'a str, pattern: &'a str) -> Self {
        Self {
            input: input_line.chars().peekable(),
            text: input_line,
        }
    }
    pub fn peek(&mut self) -> Option<char> {
        self.input.peek().copied()
    }

    pub fn match_pattern(&mut self, pattern: &str) -> ! {
        let compiled_pattern = CompiledPattern::new(pattern);
        let list = compiled_pattern.instrs;

        panic!();
    }
    pub fn match_regex(&mut self, pattern: &[Inst]) -> bool {
        if &self.text[1..2] == "^" {
            return self.match_here(&pattern[1..]);
        } else {
            loop {
                if self.text.is_empty() {
                    return false;
                } else if self.match_here(pattern) {
                    return true;
                } else {
                }
            }
        }
    }

    fn match_here(&mut self, pattern: &[Inst]) -> bool {
        todo!()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*; // 导入父模块的所有内容

    #[test]
    fn test_parse_literal_char() {
        let mut stream = "abc".chars().peekable();
        // 测试解析第一个字符 'a'
        assert_eq!(
            CompiledPattern::parse_charclass(&mut stream),
            Some(CharClass::Literal('a'))
        );
        // 验证函数只消费了一个字符，剩下 "bc"
        assert_eq!(stream.next(), Some('b'));
        assert_eq!(stream.next(), Some('c'));
        assert_eq!(stream.next(), None);
    }

    #[test]
    fn test_parse_quantifier() {
        let mut stream = "a+bc".chars().peekable();
        // 测试解析第一个字符 'a'
        assert_eq!(
            CompiledPattern::parse_charclass(&mut stream),
            Some(CharClass::Literal('a'))
        );

        assert_eq!(
            CompiledPattern::parse_quantifier(&mut stream),
            Some(Quantifier::OneOrMore)
        );
        // b
        assert_eq!(
            CompiledPattern::parse_charclass(&mut stream),
            Some(CharClass::Literal('b'))
        );
        assert_eq!(
            CompiledPattern::parse_quantifier(&mut stream),
            Some(Quantifier::One)
        );
        // c
        assert_eq!(
            CompiledPattern::parse_charclass(&mut stream),
            Some(CharClass::Literal('c'))
        );
        assert_eq!(
            CompiledPattern::parse_quantifier(&mut stream),
            Some(Quantifier::One)
        );

        assert_eq!(CompiledPattern::parse_charclass(&mut stream), None);
    }

    #[test]
    fn test_parse_escape() {
        let mut stream = r"a+\w\d".chars().peekable();
        // 测试解析第一个字符 'a'
        assert_eq!(
            CompiledPattern::parse_charclass(&mut stream),
            Some(CharClass::Literal('a'))
        );

        assert_eq!(
            CompiledPattern::parse_quantifier(&mut stream),
            Some(Quantifier::OneOrMore)
        );
        // \w
        assert_eq!(
            CompiledPattern::parse_charclass(&mut stream),
            Some(CharClass::Word)
        );
        assert_eq!(
            CompiledPattern::parse_quantifier(&mut stream),
            Some(Quantifier::One)
        );
        // \d
        assert_eq!(
            CompiledPattern::parse_charclass(&mut stream),
            Some(CharClass::Digit)
        );
        assert_eq!(
            CompiledPattern::parse_quantifier(&mut stream),
            Some(Quantifier::One)
        );

        assert_eq!(CompiledPattern::parse_charclass(&mut stream), None);
    }

    #[test]
    fn test_parse_group() {
        let mut stream = r"[abc]".chars().peekable();
        // 测试解析第一个字符 'a'
        assert_eq!(
            CompiledPattern::parse_charclass(&mut stream),
            Some(CharClass::PostiveGroup(HashSet::from(['a', 'b', 'c'])))
        );
        assert_eq!(
            CompiledPattern::parse_quantifier(&mut stream),
            Some(Quantifier::One)
        );

        assert_eq!(CompiledPattern::parse_charclass(&mut stream), None);
    }
}
