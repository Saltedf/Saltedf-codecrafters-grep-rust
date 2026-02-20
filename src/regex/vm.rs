use crate::regex::{input::Text, Inst};

pub struct VM<'r> {
    instrs: &'r Vec<Inst>,
    // context: Vec<usize>,
    capatured: Vec<(usize, usize)>,
}

impl<'r> VM<'r> {
    pub fn new(instrs: &'r Vec<Inst>) -> Self {
        Self {
            instrs,
            //context: Vec::new(),
            capatured: Vec::new(),
        }
    }
    pub fn save_context(&mut self, cursor: usize) {
        self.capatured.push((cursor, 0));
    }

    pub fn fill_back(&mut self, group_num: usize, end: usize) {
        if let Some((_, e)) = self.capatured.get_mut(group_num - 1) {
            *e = end;
        }
    }
    pub fn restore_context(&mut self, group_num: usize) {
        if let Some((_, end)) = self.capatured.get_mut(group_num - 1) {
            *end = 0
        }
    }
    pub fn jump_by(pc: usize, offset: isize) -> usize {
        ((pc as isize) + offset) as usize
    }

    pub fn run(&mut self, pc: usize, text: &Text, cursor: usize) -> bool {
        match &self.instrs[pc] {
            Inst::Char(ch) => {
                text.matchwith_at(cursor, ch)
                    && self.run(pc + 1, text, text.next_cursor_unsafe(cursor))
            }

            Inst::AnyChar => {
                text.char_at(cursor).is_some()
                    && self.run(pc + 1, text, text.next_cursor_unsafe(cursor))
            }

            Inst::Start => cursor == 0,
            Inst::End => text.char_at(cursor).is_none(),
            Inst::Match => true,
            Inst::Jump(offset) => self.run(Self::jump_by(pc, *offset), text, cursor),
            Inst::Split(offset1, offset2) => {
                self.run(Self::jump_by(pc, *offset1), text, cursor)
                    || self.run(Self::jump_by(pc, *offset2), text, cursor)
            }
            Inst::CharClass { negated, chars } => text.char_at(cursor).is_some_and(|c| {
                let res = if !negated {
                    chars.contains(&c)
                } else {
                    !chars.contains(&c)
                };
                res && self.run(pc + 1, text, text.next_cursor_unsafe(cursor))
            }),
            Inst::Digit => text.char_at(cursor).is_some_and(|c| {
                c.is_digit(10) && self.run(pc + 1, text, text.next_cursor_unsafe(cursor))
            }),
            Inst::MetaChar => text.char_at(cursor).is_some_and(|c| {
                (c.is_alphanumeric() || c == '_')
                    && self.run(pc + 1, text, text.next_cursor_unsafe(cursor))
            }),
            Inst::GroupBegin(num) => {
                self.save_context(cursor);
                self.run(pc + 1, text, cursor)
            }
            Inst::GroupEnd(num) => {
                self.fill_back(*num, cursor);
                // eprintln!("end_pos: {}", cursor);
                if self.run(pc + 1, text, cursor) {
                    true
                } else {
                    self.restore_context(*num);
                    false
                }
            }
            Inst::Ref(num) => {
                if let Some((start, end)) = self.capatured.get(num - 1) {
                    let rest = &text.text()[cursor..];
                    let capatured_group = &text.text()[*start..*end];
                    if rest.starts_with(capatured_group) {
                        self.run(pc + 1, text, cursor + capatured_group.chars().count())
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => todo!("没有实现的指令"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::regex::{input::Text, vm::VM, Inst};

    #[test]
    fn test_backref() {
        let instrs: Vec<Inst> = vec![
            Inst::GroupBegin(1),
            Inst::Char('f'),
            Inst::Char('o'),
            Inst::Char('o'),
            Inst::GroupBegin(2),
            Inst::Char('m'),
            Inst::Char('n'),
            Inst::GroupEnd(2),
            Inst::GroupEnd(1),
            Inst::Char('b'),
            Inst::Char('a'),
            Inst::Char('r'),
            Inst::Ref(2),
            Inst::Digit,
            Inst::Match,
        ];

        let input = "foomnbarmn8";
        let text = Text::new(input);

        let mut vm = VM::new(&instrs);
        assert_eq!(vm.run(0, &text, 0), true);
    }

    #[test]
    fn test_backref2() {
        let instrs: Vec<Inst> = vec![
            Inst::GroupBegin(1),
            Inst::Split(2, 6),
            Inst::Char('f'), // 2
            Inst::Char('o'),
            Inst::Char('o'),
            Inst::Jump(9),
            Inst::Char('b'), // 6
            Inst::Char('a'),
            Inst::Char('r'),
            Inst::GroupEnd(1),
            Inst::Char('-'),
            Inst::Ref(1),
            Inst::Digit,
            Inst::Match,
        ];

        let input1 = "foo-foo8";
        let text1 = Text::new(input1);
        let mut vm1 = VM::new(&instrs);
        assert_eq!(vm1.run(0, &text1, 0), true);

        let input2 = "bar-bar8";
        let text2 = Text::new(input2);
        let mut vm2 = VM::new(&instrs);
        assert_eq!(vm2.run(0, &text2, 0), true);

        let input = "foo-bar8";
        let text = Text::new(input);
        let mut vm = VM::new(&instrs);
        assert_eq!(vm.run(0, &text, 0), false);
    }

    #[test]
    fn test_vec_index() {
        let mut v = Vec::<usize>::new();
        for i in 0..9 {
            v.push(i);
        }
        if let Some(num) = v.get_mut(8) {
            eprintln!("{:#?}", num);
            *num = 100;
        }

        eprintln!("{:?}", v[8]);
        eprintln!("{:?}", v);
    }
}
