use crate::regex::Inst;
use std::{
    collections::HashSet,
    iter::Peekable,
    ops::{Add, Sub},
    str::Chars,
    usize,
};

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

    #[error("非法的量词数字")]
    InvalidQuantifier(String),
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
        let instrs = self.parse_expr()?;
        self.instrs.extend(instrs);
        self.instrs.push(Inst::Match);
        Ok(self.instrs)
    }

    fn parse_expr(&mut self) -> Result<Vec<Inst>, ParseError> {
        let mut instrs = vec![];
        while let Some(_) = self.chars.peek() {
            instrs.extend(self.parse_term()?);
        }
        Ok(instrs)
    }

    fn parse_term(&mut self) -> Result<Vec<Inst>, ParseError> {
        let block = if let Some('(') = self.chars.peek() {
            self.chars.next(); // 实际消耗 '('

            // 先插入分组开始的指令
            let num = self.next_group_num();
            let mut group_instrs = vec![];
            group_instrs.push(Inst::GroupBegin(num));

            let mut real_split: bool = false;
            let mut branch1 = vec![];
            let mut branch2 = vec![];

            loop {
                match self.chars.peek() {
                    None => return Err(ParseError::UnclosedGroup),
                    Some('|') => {
                        self.chars.next();
                        real_split = true;
                    }
                    Some(')') => {
                        self.chars.next();

                        if real_split {
                            let jump = Self::emit_jump_forward(&branch2);
                            branch1.push(jump);
                            let split_code = Self::emit_split_code(branch1, branch2);
                            group_instrs.extend(split_code);
                        } else {
                            group_instrs.extend(branch1);
                        }
                        // 插入分组结束的指令
                        let num = self.current_group_num()?;
                        group_instrs.push(Inst::GroupEnd(num));
                        break;
                    }
                    Some(_) => {
                        let res = self.parse_term()?;
                        if real_split {
                            branch2.extend(res);
                        } else {
                            branch1.extend(res);
                        }
                    }
                }
            }

            group_instrs
        } else {
            // parse like: 'a*','a+' or 'a?'
            self.parse_atom()?
        };

        let block = match self.chars.peek() {
            Some('*') => {
                self.chars.next();
                Self::emit_zero_or_more_code(block)
            }
            Some('+') => {
                self.chars.next();
                Self::emit_one_or_more_code(block)
            }
            Some('?') => {
                self.chars.next();
                Self::emit_zero_or_one_code(block)
            }
            Some('{') => {
                self.chars.next();

                let mut digit_buffer = vec![];
                let mut min = 1;
                let mut max = 1;
                loop {
                    match self.chars.peek() {
                        Some(d @ ('0'..='9' | ',')) => {
                            digit_buffer.push(*d);
                            self.chars.next();
                        }
                        Some('}') => {
                            self.chars.next();
                            let digit_text: String = digit_buffer.iter().collect();
                            let mut digits: Vec<&str> = digit_text.split(",").collect();

                            eprintln!(">>> {:?}", digits);
                            if let Some(&d1) = digits.get(0) {
                                eprintln!("-------> {d1}");
                                min = d1.parse::<usize>().map_err(|err| {
                                    ParseError::InvalidQuantifier(format!(
                                        "解析数字失败 '{}': {}",
                                        d1, err
                                    ))
                                })?;
                            } else {
                                return Err(ParseError::InvalidQuantifier(format!(
                                    "非法的量词格式"
                                )));
                            }

                            match digits.get(1) {
                                None => max = min,
                                Some(&d2) => {
                                    if d2.is_empty() {
                                        max = usize::MAX;
                                        eprintln!("-------> {max}");
                                    } else {
                                        max = d2.parse::<usize>().map_err(|err| {
                                            ParseError::InvalidQuantifier(format!(
                                                "解析数字失败 '{}': {}",
                                                d2, err
                                            ))
                                        })?;
                                    }
                                }
                            }

                            break;
                        }
                        Some(_) | None => {
                            return Err(ParseError::InvalidQuantifier(format!("解析量词失败")))
                        }
                    }
                }

                let mut repeat_block = vec![];

                if min > max {
                    return Err(ParseError::InvalidQuantifier(format!("解析量词失败")));
                }

                for _ in 1..=min {
                    repeat_block.extend_from_slice(&block);
                }
                if max == usize::MAX {
                    // >= min
                    repeat_block.extend(Self::emit_zero_or_more_code(block))
                } else if max != min {
                    // repeat min - max times
                    let at_most_once = Self::emit_zero_or_one_code(block);
                    for _ in 1..=(max - min) {
                        repeat_block.extend_from_slice(&at_most_once);
                    }
                }

                repeat_block
            }
            Some(_) | None => block,
        };

        Ok(block)
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
                | ':'
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
            Some('^') => atom_instrs.push(Inst::Start),
            Some('$') => {
                atom_instrs.push(Inst::End);
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

    fn calc_jump_offset<T>(base_pc: T, target_pc: T) -> isize
    where
        T: Add<Output = T> + Sub<Output = T> + Into<isize>,
    {
        let base: isize = base_pc.into();
        let target: isize = target_pc.into();
        target - (base)
    }

    fn emit_jump_forward(insts: &Vec<Inst>) -> Inst {
        let target = insts.len() + 1;
        let base = 0;
        let offset = Self::calc_jump_offset(base as isize, target as isize);
        Inst::Jump(offset)
    }

    fn emit_jump_backward(insts: &Vec<Inst>) -> Inst {
        let target = -1;
        let base = insts.len();
        let offset = Self::calc_jump_offset(base as isize, target as isize);
        Inst::Jump(offset)
    }

    fn emit_split_code(branch1: Vec<Inst>, branch2: Vec<Inst>) -> Vec<Inst> {
        let split_index = 0;
        let branch1_len = branch1.len();
        let branch2_index = split_index + branch1_len + 1;

        let split_code = Inst::Split(
            1,
            Self::calc_jump_offset(split_index as isize, branch2_index as isize),
        );

        let mut target_instrs: Vec<Inst> = Vec::new();
        target_instrs.push(split_code);
        target_instrs.extend(branch1.into_iter());
        target_instrs.extend(branch2.into_iter());

        target_instrs
    }

    fn emit_zero_or_more_code(block: Vec<Inst>) -> Vec<Inst> {
        let mut branch1 = block;
        let branch2 = vec![];
        branch1.push(Self::emit_jump_backward(&branch1));
        Self::emit_split_code(branch1, branch2)
    }

    fn emit_one_or_more_code(block: Vec<Inst>) -> Vec<Inst> {
        let mut res = vec![];
        res.extend_from_slice(&block);
        res.extend(Self::emit_zero_or_more_code(block));
        res
    }

    fn emit_zero_or_one_code(block: Vec<Inst>) -> Vec<Inst> {
        let branch1 = block;
        let branch2 = vec![];
        Self::emit_split_code(branch1, branch2)
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

    #[test]
    fn test_split() {
        eprintln!("empty? {}", "".is_empty());

        let text = "2,";
        let res: Vec<&str> = text.split(",").collect();
        eprintln!("{:?}", res);
        let text = "2";
        let res: Vec<&str> = text.split(",").collect();
        eprintln!("{:?}", res);
    }
}
