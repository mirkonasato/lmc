# LMC

**Language Model Client**

`lmc` is a simple command line tool to chat with an AI assistant, or more generally interact with an LLM API, directly from a terminal.

It should work with any [OpenAI-compatible API](https://platform.openai.com/docs/api-reference/chat), either local/self-hosted like [llama-server](https://github.com/ggerganov/llama.cpp/blob/master/examples/server/README.md) or [Ollama](https://ollama.com/), or cloud-based like OpenAI, Groq, etc.

You can use it either to chat interactively or chain it with other commands for scripting.

![lmc](https://github.com/user-attachments/assets/41129606-047e-41c6-88a2-48580759309e)

## Installation

`lmc` is written in Rust and comes as a single executable file. You can download binaries for Linux, macOS, and Windows from the [latest release](https://github.com/mirkonasato/lmc/releases/latest) page, or build it from source yourself using [Cargo](https://doc.rust-lang.org/cargo/commands/cargo-install.html).

## Usage

At a minimum you need to specific which `api_url` and `model` to use. E.g. to call a local Ollama server:

```sh
lmc --api-url "http://localhost:11434/v1" --model "gemma2:2b"
```

This will start an interactive chat:

```
[i] Chatting with "gemma2:2b" at "http://localhost:11434/v1"
>>> _
```

For cloud providers you'll also need a valid `api_key`, e.g. for Groq

```sh
lmc --api-url "https://api.groq.com/openai/v1" \
  --api-key "gsk_abcdef123456" \
  --model "gemma2-9b-it"
```

## Configuration

You'll typically want to predefine your assistants in a configuration file. By default `lmc` looks for a `$HOME/.lmc/config.toml`, which is a [TOML](https://toml.io/en/) file defining one or more _profiles_, i.e. groups of settings.

Here's a minimal example:

```toml
[default]
api_url = "http://localhost:11434/v1"
model = "gemma2:2b"
```

The `default` profile is used by (yep) default, i.e. if you run `lmc` without any arguments:

```
% lmc
[i] Chatting with "gemma2:2b" at "http://localhost:11434/v1"
>>> _
```

Multiple profiles can be used for different providers and/or models etc. Example:

```toml
[default]
api_url = "http://localhost:11434/v1"
model = "gemma2:2b"

[groq]
api_url = "https://api.groq.com/openai/v1"
api_key = "gsk_abcdef123456"
model = "gemma2-9b-it"
```

Use the `-p`/`--profile` argument to select a specific profile:

```
% lmc -p groq
[i] Chatting with "gemma2-9b-it" at "https://api.groq.com/openai/v1"
>>> _
```

A profile can also **extend** another profile, inheriting all its settings but adding or overriding some values. This is a flexible way to configure multiple assistants, based on different models and providers. Example:

```toml
[default]
api_url = "http://localhost:11434/v1"
model = "gemma2:2b"

[llama-3]
extends = "default"
model = "llama3.1:8b"
system_prompt = "You are a helpful assistant."

[spanish-translator]
extends = "llama-3"
system_prompt = "Your task is to translate any input text into Spanish."
```

In the above `spanish-translator` will inherit the `model` from `llama-3` and the `api_url` indirectly from `default`, while overriding the `system_prompt`.

You can also override any configuration setting at execution time by passing the corresponding command line argument.

## Interactive Mode

Chatting interactively supports line editing, courtesy of [RustyLine](https://github.com/kkawakam/rustyline).

By default each input line is sent as a separate message upon pressing `Enter`, however pasted text can include multiple lines. End a line with `\` to enter multiple lines manually.

The following prompts are treated as special _commands_:

* `/quit` or `/q`: exits the interactive loop. `Ctrl+D` also works.
* `/retry` or `/r`: resends the last prompt. Useful e.g. to generate multiple AI responses to the same query for creative purposes.

More commands might be added in future versions.

## Non-Interactive Mode

`lmc` also accepts a user prompt via standard input. This way you can pipe in the output of another command.

For example, you could summarise a PDF document with
```
% pdftotext Report.pdf - \
  | lmc --system-prompt 'Summarise the text provided as input'
```

([pdftotext](https://manpages.debian.org/experimental/poppler-utils/pdftotext.1.en.html) is a command provided by `poppler-utils`.)

Unlike in interactive mode, in this case `lmc` will exit immeditately after the first response, allowing you to do further processing on the output.

## Related Projects

* [LLM](https://github.com/simonw/llm) by Simon Willison: a Python project with more features, including logging all prompts and responses to a SQLite database
