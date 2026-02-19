pub struct MatchResult {
    is_match: bool,
    capatured: Vec<(usize, usize)>,
}

impl MatchResult {
    pub fn new(is_match: bool, capatured: Vec<(usize, usize)>) -> Self {
        Self {
            is_match,
            capatured,
        }
    }
}
