use super::{inferance::Infer, loading::Load};



trait LLM: Load + Infer {
    fn prompt(&self, prompt_task: String) -> String;
}