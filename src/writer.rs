use std::fmt::Write as FmtWrite;
use std::io::{stdout, Error, Result, Stdout, Write};
use std::primitive::str;

use crate::highlight::highlight_markdown;

pub struct StreamWriter {
    original: String,
    highlighted: String,
    stdout: Stdout,
    written: usize,
}

impl StreamWriter {
    pub fn new() -> Self {
        Self {
            original: String::new(),
            highlighted: String::new(),
            stdout: stdout(),
            written: 0,
        }
    }

    pub fn add_token(&mut self, token: &str) -> Result<()> {
        self.original
            .write_str(token)
            .map_err(|e| Error::other(e))?;
        if token.ends_with('\n') {
            self.highlight_and_write(false)
        } else {
            Ok(())
        }
    }

    pub fn complete(&mut self) -> Result<String> {
        if !self.original.ends_with('\n') {
            self.original
                .write_char('\n')
                .map_err(|e| Error::other(e))?;
        }
        self.highlight_and_write(true)?;
        Ok(self.original.clone())
    }

    fn highlight_and_write(&mut self, until_end: bool) -> Result<()> {
        self.highlighted = highlight_markdown(&self.original).map_err(|e| Error::other(e))?;
        if until_end {
            let delta = &self.highlighted[self.written..];
            self.stdout.write_all(delta.as_bytes())?;
            self.stdout.flush()?;
        } else {
            let previous_line = &self.highlighted[0..self.highlighted.len() - 1].rfind('\n');
            if let Some(position) = previous_line {
                let delta = &self.highlighted[self.written..position.to_owned()];
                self.stdout.write_all(delta.as_bytes())?;
                self.stdout.flush()?;
                self.written += delta.len();
            }
        }
        Ok(())
    }
}
