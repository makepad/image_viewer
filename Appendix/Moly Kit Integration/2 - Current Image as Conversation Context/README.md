# 2 - Current Image as Conversation Context

## Introduction

In the previous lesson we embedded Moly Kit's `Chat` widget into our slideshow
screen, which once configured, worked automatically to talk to LLM models.

However, to make this a real integration, we would like to let the LLM model
"see" the current image in the slideshow, so we can ask questions about it.

Don't be fooled, even if Moly Kit `Chat` has a default behavior, it doesn't mean
we can't change it when we really need to. To understand how, I recommend reading the official [Integrate and customize behavior](https://moxin-org.github.io/moly/integrate.html)
Moly Kit guide. But to keep knowledge here, let me try to summarize it next.

## Screenshots

![screenshot](./screenshot_002_001.png)

## Theory

### Hooks and `ChatTask`

Every important behavior in `Chat` (like updating messages, sending them,
copying to clipboard, etc.) is identified with an enum called a `ChatTask`.
When `Chat` is about to do something, it emits a `ChatTask`, that when received
back, performs the action for real.

However, `Chat` allows us to "hook" between that "send and receive" flow,
giving us the chance to modify those tasks before they are performed. To do so,
we simply configure a subscriber closure we call the "hook" using the
`set_hook_before` function. The closure will receive a **vector** of tasks that
were originally dispatched together. As we said before, modifying any of the
tasks in the vector will impact the final result once performed. Additionally,
you can inject new tasks into the vector, or simply `clear()` it to cancel all
default behaviors and handle everything on your own.

If you are cancelling (cleaning the vector), then you may be interested in the
`.perform()` and `.dispatch()` methods of `Chat`, which allow you to
programmatically trigger those behaviors yourself. The only difference between
the two is that `perform` "bypasses" the hook, while `dispatch` causes it to be
triggered.

For our purposes in this tutorial, one option would be to use a hook to wait for
`ChatTask::Send`, to insert a message in the chat with the image before it's
sent. But that would not look clean. It would be better if the image could be
"injected silently" without touching the chat history. To do so, let me also
introduce you to making a custom `BotClient`.

### `BotClient` (custom)

`BotClient` is a trait all clients that talk to an LLM in Moly Kit implement. We
saw the `OpenAIClient` before, which was a built-in implementation of that.

But we can also make our own, tailored to our needs. The
[Implement your own client](https://moxin-org.github.io/moly/custom-client.html)
guide in Moly Kit covers this trait and how to implement it, but it's considered
"advanced", and we don't care about many of these details.

What we will want is to make our own `SlideshowClient` that simply wraps the
existing `OpenAIClient`, delegating most of its implementation, but customizing
the `send()` implementation to inject a message with the image attached as
context.

## Steps

### Overview

Okay, now we have the required theory, let's put this into practice. To allow
the `Chat` to "see" our current image we will do something like the following:

1. Implement a wrapper `SlideshowClient` that simply wraps `OpenAIClient`
to add the image as a message while sending it to the LLM.
2. Set the hook that will:
   1. Wait for the `ChatTask::Send` event.
   2. Filter that task (leaving others untouched) so message updates are still
      performed automatically, but we can manually trigger our own "send
      mechanism".
   3. Define our custom send mechanism, where we will read the current image in
      the slideshow (handling the filename and mimetype as well).
   4. Give the read message to the client.
   5. Trigger a `ChatTask::Send` manually.
3. As an extra, we will adjust a little bit the DSL of the `Chat`.

### 1. Implementing the wrapper client

As we mentioned, the purpose of this wrapper client will be to take the current
image, and insert it in the message history that is sent to remote LLMs.

The implementation is simple, so let's show it and explain it inline:

```rust
use moly_kit::{OpenAIClient, protocol::*};
use std::sync::{Arc, Mutex};

// Here, we will hold the attachment to be sent to the LLM and the `OpenAIClient`
// to which we will delegate most of the behavior.
struct SlideshowClientInner {
    // An `Attachment` is how Moly Kit represents all kinds of files that are
    // exchanged with LLMs. This will be our image when set.
    attachment: Option<Attachment>,
    openai_client: OpenAIClient,
}

// This is the public client, which is reference counted so we can give a copy
// of it to `BotContext`, while also preserving it in our `App` for setting the
// attachment.
pub struct SlideshowClient(Arc<Mutex<SlideshowClientInner>>);

// Let's simply define a method to wrap the `OpenAIClient` we previously had
// from the previous tutorial chapter.
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
    // Simply delegate this impl to the `OpenAIClient`
    fn bots(&self) -> BoxPlatformSendFuture<'static, ClientResult<Vec<Bot>>> {
        self.0.lock().unwrap().openai_client.bots()
    }

    // The only method we are truly working with.
    // This method takes a list of messages and sends it to the given bot.
    fn send(
        &mut self,
        bot_id: &BotId,
        messages: &[Message],
        tools: &[Tool],
    ) -> BoxPlatformSendStream<'static, ClientResult<MessageContent>> {
        // Let's turn the immutable slice into a vec we can modify.
        let mut messages = messages.to_vec();

        // Let's insert the image as a message at the beginning (if any).
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

        // Now, simply call the delegated method in `OpenAIClient` with the
        // modified messages list.
        self.0
            .lock()
            .unwrap()
            .openai_client
            .send(bot_id, &messages, tools)
    }

    // Required for cloning across dynamic dispatch.
    fn clone_box(&self) -> Box<dyn BotClient> {
        Box::new(self.clone())
    }
}

impl SlideshowClient {
    // We will use this method to give an `Attachment` from our `App`.
    pub fn set_attachment(&self, attachment: Option<Attachment>) {
        self.0.lock().unwrap().attachment = attachment;
    }
}
```

> [!info] 
>
> Please note we are only really working with the `send()` method, and even so, we
> don't need to understand every type involved thanks to our reliance on the
> already implemented `OpenAIClient`.

We will update our `App` widget to hold the copy of this we mentioned:

```rust
#[derive(Live)]
struct App {
    // ...other fields...
    #[rust]
    slideshow_client: Option<SlideshowClient>,
}
```

And update our `configure_slideshow_chat_context` to simply wrap our already
existing `OpenAIClient` from the previous tutorial chapter:

```rust
fn configure_slideshow_chat_context(&mut self, cx: &mut Cx) {
    // ...existing code...

    // The client we already had from before, unmodified.
    let mut client = OpenAIClient::new(url);
    client.set_key(&key).unwrap();

    // The only code we are inserting, that wraps the client from before and
    // saves a copy of itself to `App`.
    let client = SlideshowClient::from(client);
    self.slideshow_client = Some(client.clone());

    // The context from before, unmodified.
    let mut bot_context = BotContext::from(client);

    // ...existing code...
}
```

> [!info]
>
> Please note we simply inserted 2 lines in the middle of what we already had.

### 2. The hook

We will update our `configure_slideshow_chat` method from the previous tutorial
chapter, to add a `configure_slideshow_chat_before_hook` method call.

```rust
fn configure_slideshow_chat(&mut self, cx: &mut Cx) {
    self.configure_slideshow_chat_context(cx);
    self.configure_slideshow_chat_before_hook(cx);
}
```

And we will implement it as the following:

```rust
fn configure_slideshow_chat_before_hook(&mut self, _cx: &mut Cx) {
    let ui = self.ui_runner();
    let mut chat = self.ui.chat(id!(slideshow.chat));

    // Here, our hook is receiving the (grouped) list of tasks that our `Chat`
    // emits for us when doing something important.
    chat.write().set_hook_before(move |task_group, _chat, _cx| {
        let before_len = task_group.len();

        // We delete any `ChatTask::Send` from the group so, whatever
        // happens, it will not cause an automatic send, but other behaviors
        // are still performed automatically.
        task_group.retain(|task| *task != ChatTask::Send);

        // If there was a `ChatTask::Send`, let's handle the send ourselves by
        // calling a `perform_chat_send` method (we will define it next).
        if task_group.len() != before_len {
            // `defer` in Makepad's `UiRunner` will be executed later at
            // `handle_event`, so other tasks not erased from the vector will be
            // already applied by then.
            ui.defer(move |me, cx, _scope| {
                me.perform_chat_send(cx);
            });
        }
    });
}
```

Then, we will need to implement `perform_chat_send`. This is where the
integration is truly completed. It will:

- Get the current image.
- Try to infer its mime type (required to build the `Attachment`).
- Extract the filename.
- Read the file bytes to memory.
- Construct the `Attachment` and set it in our `SlideshowClient` we stored in
`App`.
- Trigger `ChatTask::Send` to allow the normal send flow to happen.

The code:

```rust
fn perform_chat_send(&mut self, cx: &mut Cx) {
    let Some(client) = self.slideshow_client.as_mut() else {
        return;
    };

    // Get the current image.
    let path =
        self.state.image_paths[self.state.current_image_idx].as_path();

    // Try to infer the mime type by just looking at the extension. This is a
    // naive implementation. To do this in a serious app, you may want to use a
    // crate like `mime_guess`, or one that sniffs the real type from the
    // binary content. But this is enough for our tutorial use cases.
    let extension = path.extension().and_then(|e| e.to_str());
    let mime = extension.map(|e| match e {
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "png" => "image/png".to_string(),
        e => format!("image/{e}"),
    });

    // Extract the filename.
    let filename = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or_default();

    let mut chat = self.ui.chat(id!(slideshow.chat));

    // Try reading the file's content synchronously from the filesystem.
    match std::fs::read(path) {
        Ok(bytes) => {
            // Build the attachment from the information we collected.
            let attachment =
                Attachment::from_bytes(filename.to_string(), mime, &bytes);
            
            // Set the attachment in our client.
            client.set_attachment(Some(attachment));

            // Trigger the natural send mechanism of `Chat` that we aborted
            // earlier.
            chat.write().perform(cx, &[ChatTask::Send]);
        }
        Err(e) => {
            // Just some nice error reporting, but could be a simple "print" if
            // you want.
            chat.read()
                .messages_ref()
                .write()
                .messages
                .push(Message::app_error(e));
        }
    }
}
```

### 3. UI details

This is optional but, the "attach file" button on the left side of the prompt
input of the chat is not something important for our app. We can hide it by
overriding the Makepad DSL to hide the left side of the prompt input like this:

```rust
chat = <Chat> {
    // ...other overrides...

    prompt = {
        persistent = {
            center = {
                left = {
                    visible: false
                }

                // ...other overrides...
            }
        }
    }
}
```

## What we did

Now, we have a chat in slideshow that can "see" our current image! Try asking
it some questions like "what do you see?" to test it.

## What's Next

What we did until now is already complete. The [next lesson](../3%20-%20Generating%20Images%20to%20the%20Grid/README.md)
will use the knowledge we gained to create, configure and integrate a new
separate "chat", that will be put in the image grid screen to generate new
images at runtime!