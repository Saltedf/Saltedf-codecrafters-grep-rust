use std::{char, collections::HashSet};
use thiserror::{self, Error};

#[derive(Debug, Error)]
pub enum RegError {
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
    AlphaNumber,
}

pub struct Reg {
    instrs: Vec<Inst>,
}

impl Reg {
    pub fn new(pattern: &str) -> Result<Self, RegError> {
        let instrs = Self::compile(pattern)?;
        Ok(Self { instrs })
    }
    pub fn is_match(&self, text: &str) -> bool {
        match self.instrs.get(0) {
            Some(Inst::Start) => return self.run(1, text),
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
            None => return true,
        }
    }

    fn run(&self, pc: usize, text: &str) -> bool {
        match &self.instrs[pc] {
            Inst::Char(ch) => text.starts_with(*ch) && self.run(pc + 1, &text[ch.len_utf8()..]),
            Inst::AnyChar => todo!(),
            Inst::Start => todo!(),
            Inst::End => text.is_empty() && self.run(pc + 1, text),
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
            Inst::AlphaNumber => text.chars().nth(0).is_some_and(|c| {
                (c.is_alphanumeric() || c == '_') && self.run(pc + 1, &text[c.len_utf8()..])
            }),
        }
    }

    pub fn instrs(&self) -> &Vec<Inst> {
        &self.instrs
    }

    fn compile(pattern: &str) -> Result<Vec<Inst>, RegError> {
        let mut stream = pattern.chars().peekable();
        let mut instrs = Vec::new();
        let mut patch_list = Vec::new();
        // 查看开头是否为 ^
        if let Some('^') = stream.peek() {
            instrs.push(Inst::Start);
            stream.next();
        }

        while let Some(ch) = stream.next() {
            match ch {
                '^' => return Err(RegError::MisplacedAnchor),
                '\\' => match stream.next() {
                    Some('d') => instrs.push(Inst::Digit),
                    Some('w') => instrs.push(Inst::AlphaNumber),
                    _ => return Err(RegError::UnknownEscape),
                },
                '.' | ' ' | 'a'..='z' => {
                    let char_inst = match ch {
                        '.' => Inst::AnyChar,
                        _ => Inst::Char(ch),
                    };
                    if let Some('+') | Some('*') = stream.peek() {
                        let quantifier = stream.next().expect("非空");
                        if quantifier == '+' {
                            instrs.push(char_inst.clone())
                        }
                        patch_list.push(instrs.len());
                        let start = instrs.len();
                        instrs.push(Inst::Split(start + 1, 0));
                        instrs.push(char_inst);
                        instrs.push(Inst::Jump(start));
                        let pc_to_target = instrs.len();
                        if let Some(pc_to_patch) = patch_list.pop() {
                            if let Some(Inst::Split(_, target)) = instrs.get_mut(pc_to_patch) {
                                *target = pc_to_target;
                            }
                        }
                    } else {
                        instrs.push(char_inst);
                    }
                }
                '[' => {
                    let mut chars = HashSet::new();
                    let mut negated = false;
                    if let Some('^') = stream.peek() {
                        negated = true;
                        stream.next();
                    }

                    loop {
                        match stream.next() {
                            Some(']') => {
                                let group_instr = Inst::Group { negated, chars };
                                if let Some('+') | Some('*') = stream.peek() {
                                    let quantifier = stream.next().expect("非空");
                                    if quantifier == '+' {
                                        instrs.push(group_instr.clone());
                                    }
                                    patch_list.push(instrs.len());
                                    let start = instrs.len();
                                    instrs.push(Inst::Split(start + 1, 0));
                                    instrs.push(group_instr);
                                    instrs.push(Inst::Jump(start));
                                    let pc_to_target = instrs.len();
                                    if let Some(pc_to_patch) = patch_list.pop() {
                                        if let Some(Inst::Split(_, target)) =
                                            instrs.get_mut(pc_to_patch)
                                        {
                                            *target = pc_to_target;
                                        }
                                    }
                                } else {
                                    instrs.push(group_instr);
                                }
                                break;
                            }
                            Some(c) => {
                                chars.insert(c);
                            }
                            None => {
                                return Err(RegError::UnclosedCharClass);
                            }
                        }
                    }
                }
                _ => {
                    unimplemented!();
                }
            }
        }

        instrs.push(Inst::Match);
        Ok(instrs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Context, Error};

    #[test]
    fn test_compile_zero_or_more() -> Result<(), Error> {
        let reg = Reg::new("a*bbbb").context("编译模式串出错")?;
        let list = reg.instrs();
        eprintln!("{:?}", list);

        Ok(())
    }
    #[test]
    fn test_compile_group() -> Result<(), Error> {
        let reg = Reg::new(r"zz[^abc]d\d").context("编译模式串出错")?;
        let list = reg.instrs();
        eprintln!("{:?}", list);

        Ok(())
    }

    #[test]
    fn test_run_chars() -> Result<(), Error> {
        let reg = Reg::new("abc").context("编译模式串出错")?;
        let list = reg.instrs();
        eprintln!("{:?}", list);
        let res = reg.is_match("abe");
        assert_eq!(res, true);
        Ok(())
    }
    #[test]
    fn test_negative_group() -> Result<(), Error> {
        let reg = Reg::new("[^abc]pple").context("编译模式串出错")?;
        let list = reg.instrs();
        eprintln!("{:?}", list);
        let res = reg.is_match("applepple");
        assert_eq!(res, true);
        assert_eq!(reg.is_match(r"apple"), false);
        assert_eq!(reg.is_match(r"appleapplepple"), true);
        Ok(())
    }

    #[test]
    fn test_postive_group() -> Result<(), Error> {
        let reg = Reg::new("[abc]pple").context("编译模式串出错")?;
        let list = reg.instrs();
        eprintln!("{:?}", list);
        let res = reg.is_match("epplepple");
        assert_eq!(res, false);
        assert_eq!(reg.is_match(r"apple"), true);
        assert_eq!(reg.is_match(r"pppleapplepple"), true);
        Ok(())
    }

    #[test]
    fn test_match_escaped_char() -> Result<(), Error> {
        let reg = Reg::new(r"\d apple").context("编译模式串出错")?;
        let list = reg.instrs();
        eprintln!("{:?}", list);
        let res = reg.is_match("sally has 3 apples");
        assert_eq!(res, true);
        Ok(())
    }

    #[test]
    fn test_escaped_char_alphanumber() -> Result<(), Error> {
        let reg = Reg::new(r"\d \w\w\ws").context("编译模式串出错")?;
        let list = reg.instrs();
        eprintln!("{:?}", list);
        let res = reg.is_match("sally has 3 dogs");
        assert_eq!(res, true);
        Ok(())
    }

    #[test]
    fn test_escaped_char_underline() -> Result<(), Error> {
        let reg = Reg::new(r"\w").context("编译模式串出错")?;
        let list = reg.instrs();
        eprintln!("{:?}", list);
        eprintln!("{}", '_'.is_alphanumeric());
        let res = reg.is_match("×#÷_%÷×");
        assert_eq!(res, true);
        Ok(())
    }

    #[test]
    fn test_match_zero_or_more() -> Result<(), Error> {
        let reg = Reg::new("a*ab").context("编译模式串出错")?;
        let list = reg.instrs();
        eprintln!("{:?}", list);
        assert_eq!(reg.is_match("aaacb"), false);
        Ok(())
    }
}
