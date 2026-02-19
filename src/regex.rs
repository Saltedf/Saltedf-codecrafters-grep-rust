mod input;
mod ir;
mod parser;
mod result;
mod vm;

use crate::regex::{input::Text, ir::Inst, parser::Parser, vm::VM};

use anyhow::Error;
use std::{
    char,
    collections::{HashMap, HashSet},
};
use thiserror::{self, Error};

#[derive(Debug, Error)]
pub enum RegexError {
    #[error("'^'只能出现在字符串开头")]
    MisplacedAnchor,

    #[error("非法的转义字符")]
    UnknownEscape,

    #[error("未闭合的分组")]
    UnclosedCharClass,
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
        let input = Text::new(text);
        // let mut vm = VM::new(&self.instrs);

        match self.instrs.get(0) {
            Some(Inst::Start) => {
                let mut vm = VM::new(&self.instrs);
                vm.run(1, &input, 0)
            }
            Some(_) => {
                let mut text_cursor = 0;
                while !input.is_end(text_cursor) {
                    let mut vm = VM::new(&self.instrs);
                    if vm.run(0, &input, text_cursor) {
                        return true;
                    }
                    text_cursor = input.next_cursor_unsafe(text_cursor);
                }
                false
            }
            None => true,
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

    #[test]
    fn test_match_alternation() -> Result<(), Error> {
        let reg = Regex::new(r"((aaa|bbb)|ddd)").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!(">> {:?}", list);
        assert_eq!(reg.is_match("bbb"), true);
        Ok(())
    }

    #[test]
    fn test_match_alternation2() -> Result<(), Error> {
        let reg = Regex::new(r"^I see \d+ (cat|dog)s?$").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!(">> {:?}", list);
        assert_eq!(reg.is_match("I see 42 dogs"), true);
        Ok(())
    }
    #[test]
    fn test_capturing_groups() -> Result<(), Error> {
        let reg = Regex::new(r"(cat) and \1").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!(">> {:?}", list);
        assert_eq!(reg.is_match("cat and dog"), false);
        Ok(())
    }

    #[test]
    fn test_capturing_groups2() -> Result<(), Error> {
        let reg = Regex::new(r"^([act]+) is \1, not [^xyz]+$").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!(">> {:?}", list);
        assert_eq!(reg.is_match("cat is cat, not dog"), false);
        Ok(())
    }

    #[test]
    fn test_nested_capturing_groups() -> Result<(), Error> {
        let reg = Regex::new(r"('(cat) and \2') is the same as \1").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!(">> {:?}", list);
        assert_eq!(
            reg.is_match("'cat and cat' is the same as 'cat and cat'"),
            true
        );
        Ok(())
    }

    #[test]
    fn test_multiple_backreferences() -> Result<(), Error> {
        let reg = Regex::new(r"(\d+) (\w+) squares and \1 \2 circles").context("编译模式串出错")?;
        let list = &reg.instrs;
        eprintln!(">> {:?}", list);
        assert_eq!(reg.is_match("3 red squares and 3 red circles"), true);
        Ok(())
    }
}
