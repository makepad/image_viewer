# Moly Kit Integration

## Introduction

This optional appendix tutorial section aims to explore the integration of
[Moly Kit](https://github.com/moxin-org/moly/tree/main/moly-kit), a crate
containing abstractions, implementations and widgets for embedding LLMs in
Makepad apps.

We will construct from the [last part of the base image viewer tutorial](https://publish.obsidian.md/makepad-docs/Tutorials/Image+Viewer/7+-+Adding+Animations),
to progressively integrate a couple of LLM chats with capabilities to interact
with our previous image viewer.

We will break this lessons into 3 big fully functional parts:
1. Embed a fully functional LLM chat inside the slideshow screen.
2. Give the current image in the slideshow as context to that chat.
3. Add a separate prompt input to the grid screen, with image generation
capabilities.

Lesson 1 and 2 are connected, while lesson 3 uses what we learned to do something
totally different on a different screen.

## Screenshots

![lesson 2](./2%20-%20Current%20Image%20as%20Conversation%20Context/screenshot_002_001.png)
![lesson 3](./3%20-%20Generating%20Images%20to%20the%20Grid/screenshot_003_002.png)


## Requirements

- It is assumed you have the latest version of the base image viewer working.
- You should have access to an OpenAI compatible service with its respective
API key and support for vision and image generation models.

> [!tip]
> 
> If you don't have an OpenAI key, you can technically use any local
> model for lesson 1. For lesson 2 you will need a model with vision support.
> These kinds of models may be accessible by using something like Ollama.
>
> However, if you are going to do lesson 3, you will need an OpenAI compatible
> image generation endpoint. That's trickier to mimic locally.

## Environment variables

You will eventually need to configure the following environment variables.

```shell
export API_URL="https://api.openai.com/v1" # Or compatible
export API_KEY="<SOME API KEY>"
export MODEL_ID="gpt-5-nano"
export IMAGE_MODEL_ID="dall-e-3"
```

>  [!info]
> 
>  The `IMAGE_MODEL_ID` is only needed if you are doing lesson 3.

> [!info]
> 
> You can replace `gpt-5-nano` and `dall-e-3` with the models you prefer.
> These are simply suggested because they don't require having your identity
> verified and they are relatively cheap.

## Useful links

- [The official Moly Kit guide](https://moxin-org.github.io/moly/basics.html)
- [Moly Kit crate documentation](https://moxin-org.github.io/moly/docs/moly_kit)

## Overview

- [1 - Embedding an LLM Chat](./1%20-%20Embedding%20an%20LLM%20Chat/README.md)
- [2 - Current Image as Conversation Context](./2%20-%20Current%20Image%20as%20Conversation%20Context/README.md)
- [3 - Generating Images to the Grid](./3%20-%20Generating%20Images%20to%20the%20Grid/README.md)
