#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! Figment provider for optionally file-based env config values.
//!
//! ```rust
//! use serde::Deserialize;
//! use figment::{Figment, providers::Env};
//! use figment_file_env_provider::FileEnv;
//!
//! #[derive(Deserialize)]
//! struct Config {
//!   frobnicate: String,
//!   foo: u64,
//! }
//!
//! # figment::Jail::expect_with(|jail| {
//! # jail.create_file("secret_file", "32")?;
//! # jail.set_env("APP_FROBNICATE", "with gusto");
//! # jail.set_env("APP_FOO_FILE", "secret_file");
//! let config: Config = Figment::new()
//!     .merge(FileEnv::from_env(Env::prefixed("APP_")))
//!     .extract()?;
//! # Ok(())
//! # });
//! ```
//!
//! # Overview
//!
//! This crate contains the [`FileEnv`] provider for [`figment::Figment`], to allow loading
//! configuration values from either environment variables or files. This is especially useful
//! for secret management in combination with containers.
//!
//! For instance, to pass an API key to the configuration, you could use either the environment
//! variable `API_KEY=abc123deadbeef`, or you could write that API key to a file
//! `/secrets/api_key` and pass the env variable `API_KEY_FILE=/secrets/api_key`.
//!
//! # Recommendations
//!
//! ## Namespacing and restricting the variables read
//!
//! The provider will try to read any environment variable that ends with `_FILE` (or the custom
//! suffix} and will error if the file cannot be read. As such, it is usually necessary to have a
//! namespace for the environment variables in the form of a unique prefix, to avoid conficts or
//! unexpected interactions: `FileEnv::from_env(Env::prefixed("MY_APP_"))` (see
//! [`figment::providers::Env::prefixed`]).
//!
//! Since it is built on top of [`figment::providers::Env`], you can set it up so that only some
//! variables accept the `_FILE` variant and the rest are read normally with [`FileEnv::only`]:
//!
//! ```rust
//! # use serde::Deserialize;
//! # use figment::{Figment, providers::Env};
//! # use figment_file_env_provider::FileEnv;
//! #
//! # #[derive(Deserialize)]
//! # struct Config {
//! #   frobnicate: String,
//! #   foo: u64,
//! # }
//! #
//! # figment::Jail::expect_with(|jail| {
//! # jail.create_file("secret_file", "32")?;
//! # jail.set_env("APP_FROBNICATE", "with gusto");
//! # jail.set_env("APP_FOO_FILE", "secret_file");
//! let file_keys = ["foo", "bar"];
//! let env = Env::prefixed("APP_");
//! // Call `.only` on the FileEnv, not the Env.
//! let config: Config = Figment::new()
//!     .merge(FileEnv::from_env(env.clone()).only(&file_keys))
//!     .merge(env.ignore(&file_keys))
//!     .extract()?;
//! # Ok(())
//! # });
//! ```
//!
//! ## Changing the suffix
//!
//! You can also specify the suffix to use. For instance, to use "_PATH" instead of "_FILE":
//!
//! ```rust
//! # use serde::Deserialize;
//! # use figment::{Figment, providers::Env};
//! # use figment_file_env_provider::FileEnv;
//! #
//! # #[derive(Deserialize)]
//! # struct Config {
//! #   frobnicate: String,
//! #   foo: u64,
//! # }
//! #
//! # figment::Jail::expect_with(|jail| {
//! # jail.create_file("secret_file", "32")?;
//! # jail.set_env("APP_FROBNICATE", "with gusto");
//! # jail.set_env("APP_FOO_PATH", "secret_file");
//! let config: Config = Figment::new()
//!     .merge(FileEnv::from_env(Env::prefixed("APP_")).with_suffix("_PATH"))
//!     .extract()?;
//! # Ok(())
//! # });
//! ```

use figment::{error::Kind, value::Dict};
pub use figment::{providers::Env, Provider};
use std::collections::HashSet;

/// Provider that reads config values from the environment or from files pointed to by the
/// environment.
///
/// The config value `foo` will be read either from the env variable `FOO` or from the file
/// pointed to by `FOO_FILE`.
///
/// ```rust
/// # use serde::Deserialize;
/// # use figment::{Figment, providers::Env};
/// # use figment_file_env_provider::FileEnv;
/// #
/// #[derive(Deserialize)]
/// struct Config {
///   foo: String,
///   bar: String,
/// }
///
/// # figment::Jail::expect_with(|jail| {
/// # jail.create_file("secret_file", "bar_value")?;
/// # jail.set_env("APP_FOO", "foo_value");
/// # jail.set_env("APP_BAR_FILE", "secret_file");
/// // ENV:
/// // - `APP_FOO=foo_value`
/// // - `APP_BAR_FILE=./secret_file`
/// //
/// // Contents of the file `"./secret_file"`: `"bar_value"`
/// let config: Config = Figment::new()
///     .merge(FileEnv::from_env(Env::prefixed("APP_")))
///     .extract()?;
/// assert_eq!(config.foo, "foo_value");
/// assert_eq!(config.bar, "bar_value");
/// # Ok(())
/// # });
/// ```
#[derive(Clone)]
pub struct FileEnv {
    env: Env,
    suffix: String,
}

/// A [`FileEnv`] that cannot have its suffix changed anymore. See [`FileEnv::with_suffix`].
#[derive(Clone)]
pub struct FileEnvWithRestrictions {
    file_env: FileEnv,
}

impl FileEnv {
    /// Build from a [`figment::providers::Env`]. Any restriction or transformation applied
    /// to the `env` will propagate to the resulting `FileEnv`.
    ///
    /// In particular, it is recommended to use [`figment::providers::Env::prefixed`].
    ///
    /// Note that [`figment::providers::Env::only`] and [`figment::providers::Env::ignore`]
    /// should not be used. Use [`FileEnv::only`] and [`FileEnv::ignore`] instead.
    ///
    /// ```rust
    /// use figment_file_env_provider::{Env, FileEnv};
    /// let file_env = FileEnv::from_env(Env::prefixed("MY_APP_"));
    /// ```
    pub fn from_env(env: Env) -> Self {
        Self {
            env,
            suffix: "_file".to_string(),
        }
    }

    /// Change the suffix used to detect env variables that point to files ("_FILE" by
    /// default).
    ///
    /// ```rust
    /// # use serde::Deserialize;
    /// # use figment::{Figment, providers::Env};
    /// # use figment_file_env_provider::FileEnv;
    /// #
    /// # #[derive(Deserialize)]
    /// # struct Config {
    /// #   foo: u64,
    /// # }
    /// #
    /// # figment::Jail::expect_with(|jail| {
    /// # jail.create_file("secret_file", "32")?;
    /// # jail.set_env("APP_FOO_PATH", "secret_file");
    /// // ENV: "APP_FOO_PATH=./secret_file"
    /// // Contents of "./secret_file": "32"
    /// let config: Config = Figment::new()
    ///     .merge(FileEnv::from_env(Env::prefixed("APP_")).with_suffix("_PATH"))
    ///     .extract()?;
    /// assert_eq!(config.foo, 32);
    /// # Ok(())
    /// # });
    /// ```
    ///
    /// Note that the suffix cannot be changed after calling [`FileEnv::only`] or
    /// [`FileEnv::ignore`].
    pub fn with_suffix(self, suffix: &str) -> Self {
        Self {
            suffix: suffix.to_lowercase(),
            ..self
        }
    }

    /// Restrict the provider to process only the given list of keys (and their "_FILE"
    /// counterparts).
    ///
    /// IMPORTANT: This should be used instead of [`figment::providers::Env::only`] otherwise
    /// the "_FILE" variants won't be supported.
    ///
    /// ```rust
    /// use figment_file_env_provider::{Env, FileEnv};
    /// // This provider will look at the variables FOO, FOO_FILE, BAR and BAR_FILE.
    /// let file_env = FileEnv::from_env(Env::prefixed("MY_APP_")).only(&["foo", "bar"]);
    /// ```
    pub fn only(self, keys: &[&str]) -> FileEnvWithRestrictions {
        FileEnvWithRestrictions { file_env: self }.only(keys)
    }

    /// Restrict the provider to ignore the given list of keys (and their "_FILE"
    /// counterparts).
    ///
    /// IMPORTANT: This should be used instead of [`figment::providers::Env::ignore`] otherwise
    /// the "_FILE" variants won't be ignored.
    ///
    /// ```rust
    /// use figment_file_env_provider::{Env, FileEnv};
    /// // This provider will not look at the variables FOO, FOO_FILE, BAR and BAR_FILE.
    /// let file_env = FileEnv::from_env(Env::prefixed("MY_APP_")).ignore(&["foo", "bar"]);
    /// ```
    pub fn ignore(self, keys: &[&str]) -> FileEnvWithRestrictions {
        FileEnvWithRestrictions { file_env: self }.ignore(keys)
    }
}

impl FileEnvWithRestrictions {
    /// See [`FileEnv::only`].
    pub fn only(self, keys: &[&str]) -> Self {
        let keys: Vec<String> = keys
            .iter()
            .map(|s| s.to_string())
            .chain(keys.iter().map(|s| s.to_string() + &self.file_env.suffix))
            .collect();
        FileEnvWithRestrictions {
            file_env: FileEnv {
                env: self
                    .file_env
                    .env
                    .filter(move |key| keys.iter().any(|k| k.as_str() == key)),
                ..self.file_env
            },
        }
    }

    /// See [`FileEnv::ignore`].
    pub fn ignore(self, keys: &[&str]) -> Self {
        let keys: Vec<String> = keys
            .iter()
            .map(|s| s.to_string())
            .chain(keys.iter().map(|s| s.to_string() + &self.file_env.suffix))
            .collect();
        FileEnvWithRestrictions {
            file_env: FileEnv {
                env: self
                    .file_env
                    .env
                    .filter(move |key| !keys.iter().any(|k| k.as_str() == key)),
                ..self.file_env
            },
        }
    }
}

impl Provider for FileEnvWithRestrictions {
    fn metadata(&self) -> figment::Metadata {
        self.file_env.metadata()
    }

    fn data(&self) -> Result<figment::value::Map<figment::Profile, Dict>, figment::Error> {
        self.file_env.data()
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
                if let Some(stripped_key) = key.as_str().strip_suffix(&self.suffix) {
                    let contents = std::fs::read_to_string(&file_name).map_err(|e| {
                        Kind::Message(format!(
                            "Could not open `{}` from env variable `{}`: {:#}",
                            &file_name, &key, e
                        ))
                    })?;
                    dict.insert(
                        stripped_key.to_string(),
                        contents.parse().expect("infallible"),
                    );
                    seen_file_keys.insert(key.to_string());
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
    fn basic_env() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FIGMENT_TEST_FOO", "bar");
            jail.set_env("FIGMENT_TEST_BAZ", "put");

            let config = figment::Figment::new()
                .merge(FileEnv::from_env(Env::prefixed("FIGMENT_TEST_")))
                .extract::<Config>()?;

            assert_eq!(config.foo, "bar");
            Ok(())
        });
    }

    #[test]
    fn basic_file() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FIGMENT_TEST_FOO_FILE", "secret");
            jail.create_file("secret", "bar")?;

            let config = figment::Figment::new()
                .merge(FileEnv::from_env(Env::prefixed("FIGMENT_TEST_")))
                .extract::<Config>()?;

            assert_eq!(config.foo, "bar");
            Ok(())
        });
    }

    #[test]
    fn basic_both() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FIGMENT_TEST_FOO_FILE", "secret");
            jail.set_env("FIGMENT_TEST_FOO", "env");
            jail.create_file("secret", "file")?;

            let config = figment::Figment::new()
                .merge(FileEnv::from_env(Env::prefixed("FIGMENT_TEST_")))
                .extract::<Config>()?;

            assert_eq!(config.foo, "env");
            Ok(())
        });
    }

    #[test]
    fn with_suffix() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FIGMENT_TEST_FOO_path", "secret");
            jail.create_file("secret", "bar")?;

            let config = figment::Figment::new()
                .merge(FileEnv::from_env(Env::prefixed("FIGMENT_TEST_")).with_suffix("_PATH"))
                .extract::<Config>()?;

            assert_eq!(config.foo, "bar");
            Ok(())
        });
    }

    #[test]
    fn missing_file() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FIGMENT_TEST_BAR_FILE", "secret");
            jail.set_env("FIGMENT_TEST_FOO", "bar");

            let config = figment::Figment::new()
                .merge(FileEnv::from_env(Env::prefixed("FIGMENT_TEST_")))
                .extract::<Config>();

            assert!(config.is_err());
            Ok(())
        });
    }

    #[test]
    fn only() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FIGMENT_TEST_BAR_FILE", "secret");
            jail.set_env("FIGMENT_TEST_FOO", "bar");

            let config = figment::Figment::new()
                .merge(FileEnv::from_env(Env::prefixed("FIGMENT_TEST_")).only(&["foo"]))
                .extract::<Config>()?;

            assert_eq!(config.foo, "bar");
            Ok(())
        });
    }

    #[test]
    fn ignore() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FIGMENT_TEST_BAR_FILE", "secret");
            jail.set_env("FIGMENT_TEST_FOO", "bar");

            let config = figment::Figment::new()
                .merge(FileEnv::from_env(Env::prefixed("FIGMENT_TEST_")).ignore(&["bar"]))
                .extract::<Config>()?;

            assert_eq!(config.foo, "bar");
            Ok(())
        });
    }

    #[test]
    fn only_ignore() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("FIGMENT_TEST_BAR_FILE", "secret");
            jail.set_env("FIGMENT_TEST_BAZ_FILE", "secret");
            jail.set_env("FIGMENT_TEST_FOO", "bar");

            let config = figment::Figment::new()
                .merge(
                    FileEnv::from_env(Env::prefixed("FIGMENT_TEST_"))
                        .ignore(&["bar"])
                        .only(&["foo"]),
                )
                .extract::<Config>()?;

            assert_eq!(config.foo, "bar");
            Ok(())
        });
    }
}
