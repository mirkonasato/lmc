use anyhow::{anyhow, bail, Result};
use rustyline::error::ReadlineError;
use rustyline::history::MemHistory;
use rustyline::{DefaultEditor, Editor};

pub struct Console {
    editor: Editor<(), MemHistory>,
    continuation: bool,
}

impl Console {
    pub fn new() -> Result<Self> {
        Ok(Self {
            editor: DefaultEditor::new()?,
            continuation: false,
        })
    }

    pub fn read_piped_input(&mut self) -> Result<String> {
        let mut buffer = String::new();
        loop {
            match self.editor.readline("") {
                Ok(line) => buffer.push_str(&line),
                Err(ReadlineError::Eof) => break,
                Err(error) => {
                    bail!("Failed to read input: {}", error);
                }
            }
        }
        Ok(buffer)
    }

    pub fn read_interactive_input(&mut self) -> Result<Option<String>> {
        self.continuation = false;
        let mut buffer = String::new();
        loop {
            let prompt = if self.continuation { "... " } else { ">>> " };
            match self.editor.readline(prompt) {
                Ok(line) => {
                    if line.ends_with("\\") {
                        buffer.push_str(&line[0..(line.len() - 1)]);
                        buffer.push('\n');
                        self.continuation = true;
                    } else {
                        buffer.push_str(&line);
                        break;
                    }
                }
                Err(ReadlineError::Eof) => {
                    return Ok(None);
                }
                Err(error) => {
                    return Err(anyhow!("Failed to read input: {}", error));
                }
            }
        }
        Ok(Some(buffer))
    }
}
