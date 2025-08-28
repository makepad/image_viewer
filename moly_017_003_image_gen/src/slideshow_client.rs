use moly_kit::{OpenAIClient, protocol::*};
use std::sync::{Arc, Mutex};

struct SlideshowClientInner {
    attachment: Option<Attachment>,
    openai_client: OpenAIClient,
}

pub struct SlideshowClient(Arc<Mutex<SlideshowClientInner>>);

impl From<OpenAIClient> for SlideshowClient {
    fn from(openai_client: OpenAIClient) -> Self {
        SlideshowClient(Arc::new(Mutex::new(SlideshowClientInner {
            attachment: None,
            openai_client,
        })))
    }
}

impl Clone for SlideshowClient {
    fn clone(&self) -> Self {
        SlideshowClient(Arc::clone(&self.0))
    }
}

impl BotClient for SlideshowClient {
    fn bots(&self) -> BoxPlatformSendFuture<'static, ClientResult<Vec<Bot>>> {
        self.0.lock().unwrap().openai_client.bots()
    }

    fn send(
        &mut self,
        bot_id: &BotId,
        messages: &[Message],
        tools: &[Tool],
    ) -> BoxPlatformSendStream<'static, ClientResult<MessageContent>> {
        let mut messages = messages.to_vec();

        if let Some(attachment) = &self.0.lock().unwrap().attachment {
            messages.insert(
                0,
                Message {
                    content: MessageContent {
                        attachments: vec![attachment.clone()],
                        ..Default::default()
                    },
                    from: EntityId::User,
                    ..Default::default()
                },
            );
        }

        self.0
            .lock()
            .unwrap()
            .openai_client
            .send(bot_id, &messages, tools)
    }

    fn clone_box(&self) -> Box<dyn BotClient> {
        Box::new(self.clone())
    }
}

impl SlideshowClient {
    pub fn set_attachment(&self, attachment: Option<Attachment>) {
        self.0.lock().unwrap().attachment = attachment;
    }
}
