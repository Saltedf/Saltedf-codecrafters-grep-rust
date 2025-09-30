mod parser;

use anyhow::Error;
use std::{char, collections::HashSet};
use thiserror::{self, Error};

use crate::regex::parser::Parser;

#[derive(Debug, Error)]
pub enum RegexError {
    #[error("'^'只能出现在字符串开头")]
    MisplacedAnchor,

    #[error("非法的转义字符")]
    UnknownEscape,

    #[error("未闭合的分组")]
    UnclosedCharClass,
}

#[derive(Debug, Clone)]
enum Inst {
    Char(char), // one char
    AnyChar,
    Start,
    End,
    Match,
    Jump(usize),
    Split(usize, usize),
    Group { negated: bool, chars: HashSet<char> },
    Digit,
    MetaChar, // \w : alpha digit '_'
}

impl Inst {
    pub fn is_match(&self, ch: &char) -> bool {
        match self {
            Inst::Char(c) => *c == *ch,
            Inst::AnyChar => true,
            Inst::Start => false,
            Inst::End => false,
            Inst::Match => true,
            Inst::Jump(_) => false,
            Inst::Split(_, _) => false,
            Inst::Group { negated, chars } => {
                if *negated {
                    !chars.contains(ch)
                } else {
                    chars.contains(ch)
                }
            }
            Inst::Digit => ch.is_digit(10),
            Inst::MetaChar => ch.is_alphanumeric() || *ch == '_',
        }
    }
}

pub struct Regex {
    instrs: Vec<Inst>,
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let instrs = Parser::new(pattern).compile()?;
        Ok(Self { instrs })
    }

    pub fn is_match(&self, text: &str) -> bool {
        match self.instrs.get(0) {
            Some(Inst::Start) => self.run(1, text),
            Some(_) => {
                let mut text_cursor = text;
                while !text_cursor.is_empty() {
                    if self.run(0, text_cursor) {
                        return true;
                    }
                    let mut chars = text_cursor.chars();
                    chars.next();
                    text_cursor = chars.as_str();
                }
                false
            }
            None => true,
        }
    }

    fn run(&self, pc: usize, text: &str) -> bool {
        match &self.instrs[pc] {
            Inst::Char(ch) => text.starts_with(*ch) && self.run(pc + 1, &text[ch.len_utf8()..]),
            Inst::AnyChar => {
                let mut cursor = text.chars();
                cursor.next();
                (!text.is_empty()) && self.run(pc + 1, cursor.as_str())
            }
            Inst::Start => todo!(),
            Inst::End => text.is_empty(),
            Inst::Match => true,
            Inst::Jump(target_pc) => self.run(*target_pc, text),
            Inst::Split(b1, b2) => self.run(*b1, text) || self.run(*b2, text),
            Inst::Group { negated, chars } => text.chars().nth(0).is_some_and(|c| {
                let res = if !negated {
                    chars.contains(&c)
                } else {
                    !chars.contains(&c)
                };
                res && self.run(pc + 1, &text[c.len_utf8()..])
            }),
            Inst::Digit => text
                .chars()
                .nth(0)
                .is_some_and(|c| c.is_digit(10) && self.run(pc + 1, &text[c.len_utf8()..])),
            Inst::MetaChar => text.chars().nth(0).is_some_and(|c| {
                (c.is_alphanumeric() || c == '_') && self.run(pc + 1, &text[c.len_utf8()..])
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Context, Error};

    #[test]
    fn test_compile_zero_or_more() -> Result<(), Error> {
        let reg = Regex::new("a*bbbb").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!("{:?}", list);

        Ok(())
    }
    #[test]
    fn test_compile_group() -> Result<(), Error> {
        let reg = Regex::new(r"zz[^abc]d\d").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!("{:?}", list);

        Ok(())
    }

    #[test]
    fn test_run_chars() -> Result<(), Error> {
        let reg = Regex::new("abc").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!("{:?}", list);
        let res = reg.is_match("abc");
        assert_eq!(res, true);
        Ok(())
    }
    #[test]
    fn test_negative_group() -> Result<(), Error> {
        let reg = Regex::new("[^abc]pple").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!("{:?}", list);
        let res = reg.is_match("applepple");
        assert_eq!(res, true);
        assert_eq!(reg.is_match(r"apple"), false);
        assert_eq!(reg.is_match(r"appleapplepple"), true);
        Ok(())
    }

    #[test]
    fn test_postive_group() -> Result<(), Error> {
        let reg = Regex::new("[abc]pple").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!("{:?}", list);
        let res = reg.is_match("epplepple");
        assert_eq!(res, false);
        assert_eq!(reg.is_match(r"apple"), true);
        assert_eq!(reg.is_match(r"pppleapplepple"), true);
        Ok(())
    }

    #[test]
    fn test_match_escaped_char() -> Result<(), Error> {
        let reg = Regex::new(r"\d apple").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!("{:?}", list);
        let res = reg.is_match("sally has 3 apples");
        assert_eq!(res, true);
        Ok(())
    }

    #[test]
    fn test_escaped_char_alphanumber() -> Result<(), Error> {
        let reg = Regex::new(r"\d \w\w\ws").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!("{:?}", list);
        let res = reg.is_match("sally has 3 dogs");
        assert_eq!(res, true);
        Ok(())
    }

    #[test]
    fn test_escaped_char_underline() -> Result<(), Error> {
        let reg = Regex::new(r"\w").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!("{:?}", list);
        eprintln!("{}", '_'.is_alphanumeric());
        let res = reg.is_match("×#÷_%÷×");
        assert_eq!(res, true);
        Ok(())
    }

    #[test]
    fn test_match_zero_or_more() -> Result<(), Error> {
        let reg = Regex::new("a*ab").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!("{:?}", list);
        assert_eq!(reg.is_match("aaacb"), false);
        Ok(())
    }

    #[test]
    fn test_match_anchor() -> Result<(), Error> {
        let reg = Regex::new("a*ab$").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!("{:?}", list);
        assert_eq!(reg.is_match("aaabb"), false);
        Ok(())
    }

    #[test]
    fn test_match_wildcard() -> Result<(), Error> {
        let reg = Regex::new(r"g.+gol").context("编译模式串出错")?;
        let list = &reg.instrs;
         eprintln!(">> {:?}", list);
         assert_eq!(reg.is_match("goøö0Ogol"), true);
        Ok(())
    }
}
