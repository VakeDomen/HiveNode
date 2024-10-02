use super::tags::Tags;


pub struct Poller {
    models: Vec<String>,
    index: usize,
    default: String,
}

impl From<Tags> for Poller {
    fn from(tags: Tags) -> Self {
        let mut models = vec![];
        let default = "/".into();
        for model in tags.models.into_iter() {
            let name = model.name;
            if name.contains(":latest") {
                models.push(name.clone().replace(":latest", ""));
            }
            models.push(name);
        }
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