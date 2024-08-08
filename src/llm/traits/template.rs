pub type Eos = String;
pub type TemplatedPrompt = String;

pub trait Template {
    fn prompt_template(&self, system_msg: String, user_message: String) -> TemplatedPrompt;
    fn get_eos(&self) -> Eos;
}
