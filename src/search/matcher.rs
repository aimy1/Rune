use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

pub struct Matcher {
    inner: SkimMatcherV2,
}

impl Matcher {
    pub fn new() -> Self {
        Self {
            inner: SkimMatcherV2::default().ignore_case(),
        }
    }

    pub fn fuzzy_match(&self, text: &str, query: &str) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }
        self.inner.fuzzy_match(text, query)
    }
}
