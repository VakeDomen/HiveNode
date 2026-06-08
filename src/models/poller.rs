#[derive(Debug, Clone)]
pub struct Poller {
    models: Vec<String>,
    index: usize,
    default: String,
}

impl Poller {
    pub fn get_models_target(&self) -> String {
        if self.models.is_empty() {
            return "/".into();
        }
        self.models.join(";")
    }
}

impl From<Vec<String>> for Poller {
    fn from(models: Vec<String>) -> Self {
        let default = "/".into();
        Self {
            models,
            index: 0,
            default,
        }
    }
}

impl Iterator for Poller {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.models.is_empty() {
            return Some(self.default.clone());
        }
        let next = self.models[self.index].clone();
        self.index = (self.index + 1) % self.models.len();
        Some(next)
    }
}
