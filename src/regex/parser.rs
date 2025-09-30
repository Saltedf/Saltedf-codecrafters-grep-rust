use std::{collections::HashSet, iter::Peekable, str::Chars};

use thiserror::{self, Error};

use crate::regex::Inst;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("未知的转义字符: '\\{0}'")]
    UnknownEscape(char),

    #[error("不完整的转义字符")]
    IncompletedEscape,

    #[error("未闭合的字符类，缺少']'")]
    UnclosedCharClass,

    #[error("'$'后面不允许出现其它字符")]
    MisplacedAnchor,
}

pub struct Parser<'p> {
    chars: Peekable<Chars<'p>>,
    instrs: Vec<Inst>,
    patch_list: Vec<usize>,
}

impl<'p> Parser<'p> {
    pub fn new(pattern: &'p str) -> Self {
        Parser {
            chars: pattern.chars().peekable(),
            instrs: Vec::new(),
            patch_list: Vec::new(),
        }
    }

    pub fn compile(mut self) -> Result<Vec<Inst>, ParseError> {
        self.parse_expr()?;
        self.instrs.push(Inst::Match);
        Ok(self.instrs)
    }

    fn parse_expr(&mut self) -> Result<(), ParseError> {
        while let Some(_) = self.chars.peek() {
            self.parse_term()?;
        }
        Ok(())
    }

    fn parse_term(&mut self) -> Result<(), ParseError> {
        let atom = self.parse_atom()?;
        match self.chars.peek() {
            Some('*') => {
                self.chars.next();
                self.generate_zero_or_more_code(atom)?;
            }
            Some('+') => {
                self.chars.next();
                self.generate_one_or_more_code(atom)?;
            }
            Some('?') => {
                self.chars.next();
                self.generate_zero_or_one_code(atom)?;
            }
            Some('{') => todo!(),
            Some(_) | None => self.instrs.extend(atom.into_iter()),
        }
        Ok(())
    }

    fn parse_atom(&mut self) -> Result<Vec<Inst>, ParseError> {
        let mut atom_instrs = vec![];

        match self.chars.next() {
            Some('.') => atom_instrs.push(Inst::AnyChar),
            Some(ch @ ('a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '-' | '_')) => {
                atom_instrs.push(Inst::Char(ch));
            }
            Some('\\') => match self.chars.next() {
                Some('d') => atom_instrs.push(Inst::Digit),
                Some('w') => atom_instrs.push(Inst::MetaChar),
                Some(e) => return Err(ParseError::UnknownEscape(e)),
                None => return Err(ParseError::IncompletedEscape),
            },
            Some('[') => {
                let mut set = HashSet::new();
                let mut negated = false;
                if let Some('^') = self.chars.peek() {
                    negated = true;
                    self.chars.next();
                }
                loop {
                    match self.chars.next() {
                        Some(']') => {
                            atom_instrs.push(Inst::Group {
                                negated,
                                chars: set,
                            });
                            break;
                        }
                        Some('\\') => match self.chars.next() {
                            Some('d') => set.extend(('0'..='9').into_iter()),
                            Some('w') => {
                                set.insert('_');
                                set.extend(('0'..='9').into_iter());
                                set.extend(('a'..='z').into_iter());
                                set.extend(('A'..='Z').into_iter());
                            }
                            Some(c) => return Err(ParseError::UnknownEscape(c)),
                            None => return Err(ParseError::IncompletedEscape),
                        },
                        Some(start) if start.is_ascii_alphanumeric() => {
                            if self.chars.next_if_eq(&'-').is_none() {
                                set.insert(start);
                            } else if let Some(end) =
                                self.chars.next_if(|c| c.is_ascii_alphanumeric())
                            {
                                set.extend(Self::translate_range(start, end));
                            } else {
                                set.insert('-');
                            }
                        }
                        Some(ch) => {
                            set.insert(ch);
                        }
                        None => return Err(ParseError::UnclosedCharClass),
                    }
                }
            }
            Some('^') => self.instrs.push(Inst::Start),
            Some('$') => {
                self.instrs.push(Inst::End);
                if self.chars.peek().is_some() {
                    return Err(ParseError::MisplacedAnchor);
                }
            }
            Some(_) => todo!(),
            None => todo!(),
        }

        Ok(atom_instrs)
    }

    fn pc(&self) -> usize {
        self.instrs.len()
    }
    fn generate_zero_or_more_code(&mut self, atom: Vec<Inst>) -> Result<(), ParseError> {
        let start = self.pc();
        let split_code = Inst::Split(start + 1, 0);
        self.instrs.push(split_code);
        self.patch_list.push(start);
        self.instrs.extend(atom.into_iter());
        let jump_code = Inst::Jump(start);
        self.instrs.push(jump_code);
        let pc_to_target = self.pc();
        if let Some(patch) = self.patch_list.pop() {
            if let Some(Inst::Split(_, target)) = self.instrs.get_mut(patch) {
                *target = pc_to_target;
            }
        }

        Ok(())
    }

    fn generate_one_or_more_code(&mut self, atom: Vec<Inst>) -> Result<(), ParseError> {
        self.instrs.extend(atom.clone().into_iter());
        self.generate_zero_or_more_code(atom)?;
        Ok(())
    }

    fn generate_zero_or_one_code(&mut self, atom: Vec<Inst>) -> Result<(), ParseError> {
        let start = self.pc();
        let split_code = Inst::Split(start + 1, 0);
        self.instrs.push(split_code);
        self.patch_list.push(start);
        self.instrs.extend(atom.into_iter());
        let pc_to_target = self.pc();
        if let Some(index) = self.patch_list.pop() {
            if let Some(Inst::Split(_, target)) = self.instrs.get_mut(index) {
                *target = pc_to_target;
            }
        }
        Ok(())
    }

    fn translate_range(start: char, end: char) -> impl IntoIterator<Item = char> {
        let is_range = (start as usize <= end as usize)
            && (start.is_ascii_digit() && end.is_ascii_digit()
                || start.is_ascii_lowercase() && end.is_ascii_lowercase()
                || start.is_ascii_uppercase() && end.is_ascii_uppercase());

        if is_range {
            return (start..=end).collect::<Vec<char>>().into_iter();
        } else {
            return vec![start, '-', end].into_iter();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splice() {
        let a = '0';
        let z = 'z';
        let aa = a as usize;
        let zz = z as usize;
        eprintln!("{aa} --> {zz}");
        let v: Vec<char> = (a..=z).into_iter().collect();
        eprintln!("{:?}", v);
    }
}
