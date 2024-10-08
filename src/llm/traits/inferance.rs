use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use tokio::sync::mpsc::Sender;
use crate::config::{REPEAT_LAST_N, REPEAT_PENALTY, SEED, SPLIT_PROPMT, TEMPERATURE, TOP_K, TOP_P};
use crate::llm::models::core::token::Token;
use crate::llm::models::core::tokenizer::TokenOutputStream;
use crate::ws::messages::message::{IncommingMessage, OutgoingMessage};
use crate::ws::messages::message_type::{IncommingMessageBody, OutgoingMessageBody, OutgoingMessageType};
use crate::ws::messages::variants::incomming::submit_prompt::SubmitPrompt;
use crate::ws::messages::variants::outgoing::response_prompt::ResponsePrompt;
use crate::ws::messages::variants::outgoing::response_prompt_token::ResponsePromptToken;

use super::template::Template;
use super::tokenize::Tokenize;


pub trait Infer: Tokenize + Template {
    fn get_max_sample_len(&self) -> usize;
    fn get_max_sequence_len(&self) -> usize;
    fn get_device(&self) -> &Device;
    fn get_model_name(&self) -> String;
    fn forward(&mut self, input: &Tensor, position: usize) -> Result<Tensor>;
    
    /// Sets up a logits processor based on predefined settings and sampling strategy.
    ///
    /// # Returns
    /// An instance of `LogitsProcessor` configured with a specific sampling strategy.
    fn setup_logit_procesing(&self) -> LogitsProcessor {
        let sampling = self.setup_sampling();
        LogitsProcessor::from_sampling(SEED, sampling)
    }

    /// Configures the sampling strategy based on predefined temperature and probability settings.
    ///
    /// # Returns
    /// A `Sampling` variant configured according to the global temperature, TOP_K, and TOP_P settings.
    fn setup_sampling(&self) -> Sampling {
        if TEMPERATURE <= 0. {
            Sampling::ArgMax
        } else {
            match (TOP_K, TOP_P) {
                (None, None) => Sampling::All { temperature: TEMPERATURE },
                (Some(k), None) => Sampling::TopK { k, temperature: TEMPERATURE },
                (None, Some(p)) => Sampling::TopP { p, temperature: TEMPERATURE },
                (Some(k), Some(p)) => Sampling::TopKThenTopP { k, p,temperature:  TEMPERATURE },
            }
        }
    }

    fn infer(&mut self, prompt_tokens: &Vec<Token>, sender: Sender<OutgoingMessage>, task_id: String) -> Result<String> {
        let mut response_chunks = vec![];
        let mut tos = TokenOutputStream::new(self.tokenizer().clone());
        let to_sample = self.get_max_sample_len().saturating_sub(1);
        let prompt_tokens = if prompt_tokens.len() + to_sample > self.get_max_sequence_len() {
            let to_remove = prompt_tokens.len() + to_sample - self.get_max_sequence_len();
            prompt_tokens[prompt_tokens.len().saturating_sub(to_remove)..].to_vec()
        } else {
            prompt_tokens.to_vec()
        };
        let mut all_tokens = vec![];
        let mut logits_processor = self.setup_logit_procesing();
        let mut next_token = if !SPLIT_PROPMT {
            // Generate response in a single batch if not splitting.
            let input = Tensor::new(prompt_tokens.as_slice(), self.get_device())?.unsqueeze(0)?;
            let logits = self.forward(&input, 0)?;
            let logits = logits.squeeze(0)?;
            logits_processor.sample(&logits)? 
        } else {
            // Generate response token by token if splitting.
            let mut next_token = 0;
            for (pos, token) in prompt_tokens.iter().enumerate() {
                let input = Tensor::new(&[*token], self.get_device())?.unsqueeze(0)?;
                let logits = self.forward(&input, pos)?;
                let logits = logits.squeeze(0)?;
                next_token = logits_processor.sample(&logits)?
            }
            next_token
        };
        all_tokens.push(next_token);
        
        // Collect chunks of the generated response.
        if let Some(token) = tos.next_token(next_token)? {
            response_chunks.push(token.clone());
            self.send_token(task_id.clone(), token, sender.clone());
        }


        // Continue generating tokens until the sample length is reached or an end-of-sentence token is encountered.
        let eos_token = *tos
            .tokenizer()
            .get_vocab(true)
            .get(&self.get_eos())
            .unwrap();

        let mut sampled = 0;
        for index in 0..to_sample {
            let input = Tensor::new(&[next_token], self.get_device())?.unsqueeze(0)?;
            let logits = self.forward(&input, prompt_tokens.len() + index)?;
            let logits = logits.squeeze(0)?;
            let logits = if REPEAT_PENALTY == 1. {
                logits
            } else {
                let start_at = all_tokens.len().saturating_sub(REPEAT_LAST_N);
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    REPEAT_PENALTY,
                    &all_tokens[start_at..],
                )?
            };
            next_token = logits_processor.sample(&logits)?;
            all_tokens.push(next_token);
            if let Some(token) = tos.next_token(next_token)? {
                response_chunks.push(token.clone());
                self.send_token(task_id.clone(), token, sender.clone());
            }
            sampled += 1;
            if next_token == eos_token {
                break;
            };
        }
        Ok(response_chunks.join(""))
    }
   

    fn send_token(&self, task_id: String, token: String, sender: Sender<OutgoingMessage>) {
        let model_name = self.get_model_name();
        tokio::spawn(async move {
            let message = OutgoingMessage {
                message_type: OutgoingMessageType::ResponsePromptToken,
                task_id,
                body: OutgoingMessageBody::ResponsePromptToken(ResponsePromptToken {
                    model: model_name,
                    token,
                }),
            };
            if let Err(e) = sender.send(message).await {
                eprintln!("Failed to send token message: {}", e);
            }
        });
    }

    fn send_response(&self, task_id: String, task: SubmitPrompt, response: String, sender: Sender<OutgoingMessage>) {
        // Spawn a new async task to send the message.
        let model_name = self.get_model_name();
        tokio::spawn(async move {
            let message = OutgoingMessage {
                message_type: OutgoingMessageType::ResponsePrompt,
                task_id,
                body: OutgoingMessageBody::ResponsePrompt(ResponsePrompt {
                    model: model_name,
                    system_mesage: task.system_mesage,
                    mode: task.mode,
                    response,
                    tokenizer_time: 0,
                    inference_time: 0,
                    tokens_processed: 0,
                    tokens_generated: 0,
                }),
            };
            if let Err(e) = sender.send(message).await {
                eprintln!("Failed to send response message: {}", e);
            }
        });
    }
}


