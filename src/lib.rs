use figment::{error::Kind, providers::Env, value::Dict, Provider};
use std::{collections::HashSet, path::Path};

pub struct FileEnv {
    env: Env,
}

impl FileEnv {
    pub fn new(env: Env) -> Self {
        Self { env }
    }
}

impl Provider for FileEnv {
    fn metadata(&self) -> figment::Metadata {
        self.env.metadata()
    }

    fn data(
        &self,
    ) -> Result<figment::value::Map<figment::Profile, figment::value::Dict>, figment::Error> {
        let mut dict = Dict::new();
        let seen_file_keys = {
            let mut seen_file_keys = HashSet::<String>::new();
            for (key, file_name) in self.env.iter() {
                match key.as_str().strip_suffix("_file") {
                    None => continue,
                    Some(stripped_key) => {
                        if !Path::exists(Path::new(&file_name)) {
                            continue;
                        }
                        let contents = std::fs::read_to_string(file_name)
                            .map_err(|e| Kind::Message(e.to_string()))?;
                        dict.insert(
                            stripped_key.to_string(),
                            contents.parse().expect("infallible"),
                        );
                        seen_file_keys.insert(key.to_string());
                    }
                }
            }
            seen_file_keys
        };

        for (key, value) in self.env.iter() {
            if seen_file_keys.contains(key.as_str()) {
                continue;
            }
            dict.insert(key.to_string(), value.parse().expect("infallible"));
        }

        Ok(self.env.profile.collect(dict))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Config {
        foo: String,
    }

    #[test]
    fn with_env() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FOO", "bar");
            jail.set_env("BAZ", "put");

            let config = figment::Figment::new()
                .merge(FileEnv::new(Env::raw()))
                .extract::<Config>()?;

            assert_eq!(config.foo, "bar");
            Ok(())
        });
    }

    #[test]
    fn with_file() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FOO_FILE", "secret");
            jail.create_file("secret", "bar")?;

            let config = figment::Figment::new()
                .merge(FileEnv::new(Env::raw()))
                .extract::<Config>()?;

            assert_eq!(config.foo, "bar");
            Ok(())
        });
    }

    #[test]
    fn with_both() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FOO_FILE", "secret");
            jail.set_env("FOO", "env");
            jail.create_file("secret", "file")?;

            let config = figment::Figment::new()
                .merge(FileEnv::new(Env::raw()))
                .extract::<Config>()?;

            assert_eq!(config.foo, "env");
            Ok(())
        });
    }
}
