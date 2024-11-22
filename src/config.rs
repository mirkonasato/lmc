use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, ensure, Context, Result};
use argh::FromArgs;
use home::home_dir;
use serde::Deserialize;

/// LMC - Large Model Client: interact with LLM APIs from the command line
#[derive(Debug, FromArgs)]
pub struct Args {
    /// base URL, e.g. "http://localhost:11434/v1" for Ollama
    #[argh(option, short = 'u')]
    pub api_url: Option<String>,

    /// secret key, if the API requires authentication
    #[argh(option, short = 'k')]
    pub api_key: Option<String>,

    /// model name, e.g. "gemma2:9b"
    #[argh(option, short = 'm')]
    pub model: Option<String>,

    /// initial instructions for the assistant
    #[argh(option, short = 's')]
    pub system_prompt: Option<String>,

    /// parameter passed directly to the API
    #[argh(option, short = 't')]
    pub temperature: Option<f32>,

    /// configuration file; default: "$HOME/.lmc/config.toml"
    #[argh(option, short = 'c')]
    pub config: Option<String>,

    /// configuration profile; default: "default"
    #[argh(option, short = 'p')]
    pub profile: Option<String>,

    /// disable response streaming
    #[argh(switch)]
    pub no_streaming: Option<bool>,

    /// display the version
    #[argh(switch, short = 'v', long = "version")]
    pub print_version: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Config {
    pub api_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
}

impl Config {
    fn from_profile(profile: &Profile) -> Result<Self> {
        ensure!(profile.api_url.is_some(), "No \"api_url\" provided");
        ensure!(profile.model.is_some(), "No \"model\" provided");
        Ok(Self {
            api_key: profile.api_key.to_owned(),
            api_url: profile.api_url.to_owned().unwrap(),
            model: profile.model.to_owned().unwrap(),
            system_prompt: profile.system_prompt.to_owned(),
            temperature: profile.temperature.to_owned(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Profile {
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub extends: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
}

impl Profile {
    pub fn new() -> Self {
        Self {
            api_key: None,
            api_url: None,
            extends: None,
            model: None,
            system_prompt: None,
            temperature: None,
        }
    }
    fn merge_with(mut self, other: &Self) -> Self {
        if let Some(api_key) = &other.api_key {
            self.api_key = Some(api_key.to_owned());
        }
        if let Some(api_url) = &other.api_url {
            self.api_url = Some(api_url.to_owned());
        }
        if let Some(model) = &other.model {
            self.model = Some(model.to_owned());
        }
        if let Some(extends) = &other.extends {
            self.extends = Some(extends.to_owned());
        }
        if let Some(system_prompt) = &other.system_prompt {
            self.system_prompt = Some(system_prompt.to_owned());
        }
        if let Some(temperature) = &other.temperature {
            self.temperature = Some(temperature.to_owned());
        }
        self
    }
    fn override_with_args(mut self, args: &Args) -> Self {
        if let Some(api_key) = &args.api_key {
            self.api_key = Some(api_key.to_owned());
        }
        if let Some(api_url) = &args.api_url {
            self.api_url = Some(api_url.to_owned());
        }
        if let Some(model) = &args.model {
            self.model = Some(model.to_owned());
        }
        if let Some(system_prompt) = &args.system_prompt {
            self.system_prompt = Some(system_prompt.to_owned());
        }
        if let Some(temperature) = &args.temperature {
            self.temperature = Some(temperature.to_owned());
        }
        self
    }
}

pub fn get_config(args: &Args) -> Result<Config> {
    let profiles = parse_config_file(&args.config)?;
    let selected = resolve_profile(&profiles, &args.profile)?;
    let overriden = selected.clone().override_with_args(args);
    let config = Config::from_profile(&overriden)?;
    Ok(config)
}

fn parse_config_file(file: &Option<String>) -> Result<HashMap<String, Profile>> {
    let path = match file {
        Some(value) => PathBuf::from(value),
        None => {
            let dir = home_dir().context("Could not detect HOME directory")?;
            dir.join(".lmc").join("config.toml")
        }
    };
    if fs::exists(&path)? {
        let source = fs::read_to_string(&path)?;
        let profiles: HashMap<String, Profile> = toml::from_str(&source)?;
        Ok(profiles)
    } else {
        match file {
            Some(value) => Err(anyhow!("Configuration file not found: \"{}\"", value)),
            None => Ok(HashMap::new()),
        }
    }
}

fn resolve_profile(
    profiles: &HashMap<String, Profile>,
    profile_arg: &Option<String>,
) -> Result<Profile> {
    let name = profile_arg.to_owned().unwrap_or(String::from("default"));
    if let Some(selected) = profiles.get(&name) {
        match selected.extends {
            None => Ok(selected.to_owned()),
            Some(_) => {
                let merged = flatten_profile_hierarchy(profiles, selected)?;
                Ok(merged)
            }
        }
    } else {
        match profile_arg {
            None => Ok(Profile::new()),
            Some(name) => Err(anyhow!("No such configuration profile: \"{}\"", name)),
        }
    }
}

fn flatten_profile_hierarchy(
    profiles: &HashMap<String, Profile>,
    selected: &Profile,
) -> Result<Profile> {
    let mut current = selected;
    let mut hierarchy: Vec<&Profile> = vec![selected];
    while let Some(name) = &current.extends {
        let parent = profiles
            .get(name)
            .with_context(|| format!("No such configuration profile: \"{}\"", name))?;
        ensure!(
            !hierarchy.contains(&parent),
            "Circular \"extends\" references: \"{}\"",
            name
        );
        hierarchy.push(parent);
        current = parent;
    }
    hierarchy.reverse();
    let mut merged = Profile::new();
    for node in hierarchy {
        merged = merged.merge_with(node);
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn minimal_default_config() -> Result<()> {
        let config_file = write_temp_config(
            r#"
[default]
api_url = "http://localhost:11434/v1"
model = "gemma2:9b"
"#,
        )?;

        let args = args_with_config(&config_file)?;

        let config = get_config(&args)?;
        assert_eq!(
            config,
            Config {
                api_key: None,
                api_url: String::from("http://localhost:11434/v1"),
                model: String::from("gemma2:9b"),
                system_prompt: None,
                temperature: None,
            }
        );
        Ok(())
    }

    #[test]
    fn hierarchical_profiles() -> Result<()> {
        let config_file = write_temp_config(
            r#"
[groq]
api_url = "https://api.groq.com/openai/v1"
api_key = "gsk_abc123"
model = "llama-3.1-8b-instant"

[llama3-70b]
extends = "groq"
model = "llama-3.1-70b-versatile"
system_prompt = "You are a helpful assistant."

[poet]
extends = "llama3-70b"
system_prompt = "You are a poet, and will answer any question in rhyme."
temperature = 1.5
"#,
        )?;

        let mut args = args_with_config(&config_file)?;
        args.profile = Some(String::from("poet"));

        let config = get_config(&args)?;
        assert_eq!(
            config,
            Config {
                api_url: String::from("https://api.groq.com/openai/v1"),
                api_key: Some(String::from("gsk_abc123")),
                model: String::from("llama-3.1-70b-versatile"),
                system_prompt: Some(String::from(
                    "You are a poet, and will answer any question in rhyme."
                )),
                temperature: Some(1.5),
            }
        );
        Ok(())
    }

    #[test]
    fn args_take_precedence() -> Result<()> {
        let config_file = write_temp_config(
            r#"
[default]
api_url = "http://localhost:11434/v1"
model = "gemma2:9b"
system_prompt = "You are a helpful assistant."
"#,
        )?;

        let mut args = args_with_config(&config_file)?;
        args.model = Some(String::from("llama3.1:8b"));
        args.system_prompt = Some(String::from("Summarise the text provided as input."));

        let config = get_config(&args)?;
        assert_eq!(
            config,
            Config {
                api_url: String::from("http://localhost:11434/v1"),
                api_key: None,
                model: String::from("llama3.1:8b"),
                system_prompt: Some(String::from("Summarise the text provided as input.")),
                temperature: None,
            }
        );
        Ok(())
    }

    #[test]
    fn missing_selected_profile() -> Result<()> {
        let config_file = write_temp_config(
            r#"
[profile-1]
extends = "profile-3"
api_url = "http://localhost:11434/v1"

[profile-2]
extends = "profile-1"
model = "gemma2:9b"
system_prompt = "You are a helpful assistant."

[profile-3]
extends = "profile-2"
model = "llama3.1:8b"
"#,
        )?;

        let mut args = args_with_config(&config_file)?;
        args.profile = Some(String::from("profile-3"));

        let result = get_config(&args);
        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "Circular \"extends\" references: \"profile-3\""
        );
        Ok(())
    }

    #[test]
    fn circular_references() -> Result<()> {
        let config_file = write_temp_config(
            r#"
[default]
api_url = "http://localhost:11434/v1"
model = "gemma2:9b"
"#,
        )?;

        let mut args = args_with_config(&config_file)?;
        args.profile = Some(String::from("superhuman"));

        let result = get_config(&args);
        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "No such configuration profile: \"superhuman\""
        );
        Ok(())
    }

    fn write_temp_config(source: &str) -> Result<NamedTempFile> {
        let mut config_file = NamedTempFile::new()?;
        config_file.write_all(source.as_bytes())?;
        Ok(config_file)
    }

    fn new_args() -> Args {
        Args {
            api_key: None,
            api_url: None,
            config: None,
            model: None,
            no_streaming: None,
            profile: None,
            system_prompt: None,
            temperature: None,
            print_version: false,
        }
    }

    fn args_with_config(config_file: &NamedTempFile) -> Result<Args> {
        let mut args = new_args();
        let config_filename = config_file.path().to_str().context("Path.to_str")?.into();
        args.config = Some(config_filename);
        Ok(args)
    }
}
