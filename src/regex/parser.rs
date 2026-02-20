use crate::regex::Inst;
use std::{collections::HashSet, iter::Peekable, str::Chars};
use thiserror::{self, Error};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("未知的转义字符: '\\{0}'")]
    UnknownEscape(char),

    #[error("不完整的转义字符")]
    IncompletedEscape,

    #[error("未闭合的字符类，缺少']'")]
    UnclosedCharClass,

    #[error("未闭合的()，缺少')'")]
    UnclosedGroup,

    #[error("'$'后面不允许出现其它字符")]
    MisplacedAnchor,

    #[error("指令回填错误")]
    PatchError,

    #[error("捕获组序号不匹配")]
    GroupNumMissError,
}

pub struct Parser<'p> {
    chars: Peekable<Chars<'p>>,
    instrs: Vec<Inst>,
    patch_list: Vec<usize>,
    num_stack: Vec<usize>,
    next_group_num: usize,
}

impl<'p> Parser<'p> {
    pub fn new(pattern: &'p str) -> Self {
        Parser {
            chars: pattern.chars().peekable(),
            instrs: Vec::new(),
            patch_list: Vec::new(),
            num_stack: Vec::new(),
            next_group_num: 1,
        }
    }

    pub fn next_group_num(&mut self) -> usize {
        let group_num = self.next_group_num;
        self.next_group_num += 1;
        self.num_stack.push(group_num);
        group_num
    }
    pub fn current_group_num(&mut self) -> Result<usize, ParseError> {
        if let Some(num) = self.num_stack.pop() {
            Ok(num)
        } else {
            return Err(ParseError::GroupNumMissError);
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

    fn patch_top_inst(&mut self, pc_to_target: usize) -> Result<(), ParseError> {
        if let Some(inst_id) = self.patch_list.pop() {
            match self.instrs.get_mut(inst_id) {
                Some(Inst::Split(_, target)) => {
                    *target = pc_to_target;
                }
                Some(Inst::Jump(target)) => {
                    *target = pc_to_target;
                }
                Some(_) | None => return Err(ParseError::PatchError),
            }
        }
        Ok(())
    }
    fn parse_term(&mut self) -> Result<(), ParseError> {
        if let Some('(') = self.chars.peek() {
            self.chars.next(); // 实际消耗 '('
                               // 先插入分组开始的指令
            let num = self.next_group_num();
            self.instrs.push(Inst::GroupBegin(num));

            let mut real_split: bool = false;
            let split_index = self.pc();
            let split = Inst::Split(split_index + 1, 0);
            self.patch_list.push(split_index);
            self.instrs.push(split);
            loop {
                match self.chars.peek() {
                    None => return Err(ParseError::UnclosedGroup),
                    Some('|') => {
                        self.chars.next();
                        real_split = true;
                        let jump = Inst::Jump(0);
                        let jump_index = self.pc();
                        self.instrs.push(jump);
                        self.patch_top_inst(self.pc())?;
                        self.patch_list.push(jump_index);
                    }
                    Some(')') => {
                        self.chars.next();
                        if real_split {
                            self.patch_top_inst(self.pc())?;
                        } else {
                            // 组内无分支指令时, 不该继续回填split的第二地址,
                            // 而要消除开头插入的spilt, 改为jump
                            if let Some(split) = self.instrs.get_mut(split_index) {
                                *split = Inst::Jump(split_index + 1)
                            }
                        }

                        // 插入分组结束的指令
                        let num = self.current_group_num()?;
                        self.instrs.push(Inst::GroupEnd(num));

                        break;
                    }
                    Some(_) => self.parse_term()?,
                }
            }
        } else {
            // parse like: 'a*','a+' or 'a?'
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
        }

        Ok(())
    }

    fn parse_atom(&mut self) -> Result<Vec<Inst>, ParseError> {
        let mut atom_instrs = vec![];

        match self.chars.next() {
            Some('.') => atom_instrs.push(Inst::AnyChar),
            Some(
                ch @ ('a'..='z'
                | 'A'..='Z'
                | '0'..='9'
                | ','
                | '<'
                | '>'
                | ' '
                | '-'
                | '_'
                | '"'
                | '\''),
            ) => {
                atom_instrs.push(Inst::Char(ch));
            }
            Some('\\') => match self.chars.next() {
                Some('d') => atom_instrs.push(Inst::Digit),
                Some('w') => atom_instrs.push(Inst::MetaChar),
                Some('\\') => atom_instrs.push(Inst::Char('\\')),
                Some(d @ '1'..='9') => {
                    // 向前引用 \1 \2
                    atom_instrs.push(Inst::Ref(d.to_digit(10).unwrap() as usize))
                }
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
                            atom_instrs.push(Inst::CharClass {
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
            Some(c) => {
                eprintln!("未支持的字符: {}", c);
                todo!()
            }
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
