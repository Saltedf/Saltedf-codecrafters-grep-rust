use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum Inst {
    Char(char), // one char
    AnyChar,
    Start,
    End,
    Match,
    Jump(usize),
    Split(usize, usize),
    CharClass { negated: bool, chars: HashSet<char> },
    Digit,
    MetaChar, // \w : alpha digit '_'

    GroupBegin(usize), // (
    GroupEnd(usize),   // )
    Ref(usize),        // '\1'
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
            Inst::CharClass { negated, chars } => {
                if *negated {
                    !chars.contains(ch)
                } else {
                    chars.contains(ch)
                }
            }
            Inst::Digit => ch.is_digit(10),
            Inst::MetaChar => ch.is_alphanumeric() || *ch == '_',
            _ => todo!("尚未实现指令的匹配逻辑"),
        }
    }
}
