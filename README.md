# Figment FileEnv Provider &thinsp; [![ci.svg]][ci] [![crates.io]][crate] [![docs.rs]][docs]

[crates.io]: https://img.shields.io/crates/v/figment_file_env_provider.svg
[crate]: https://crates.io/crates/figment_file_env_provider
[docs.rs]: https://docs.rs/figment_file_env_provider/badge.svg
[docs]: https://docs.rs/figment_file_env_provider
[ci.svg]: https://github.com/nitnelave/figment_file_env_provider/workflows/CI/badge.svg
[ci]: https://github.com/nitnelave/figment_file_env_provider/actions

[Figment](https://docs.rs/figment/latest/figment/) provider for optionally file-based env config values.

```rust
use serde::Deserialize;
use figment::{Figment, providers::Env};
use figment_file_env_provider::FileEnv;

#[derive(Deserialize)]
struct Config {
  frobnicate: String,
  foo: u64,
}

# figment::Jail::expect_with(|jail| {
# jail.create_file("secret_file", "32")?;
# jail.set_env("APP_FROBNICATE", "with gusto");
# jail.set_env("APP_FOO_FILE", "secret_file");
let config: Config = Figment::new()
    .merge(FileEnv::from_env(Env::prefixed("APP_")))
    .extract()?;
# Ok(())
# });
```

# Overview

This crate contains the `FileEnv` provider for `Figment`, to allow loading
configuration values from either environment variables or files. This is especially useful
for secret management in combination with containers.

For instance, to pass an API key to the configuration, you could use either the environment
variable `API_KEY=abc123deadbeef`, or you could write that API key to a file
`/secrets/api_key` and pass the env variable `API_KEY_FILE=/secrets/api_key`.

See the [documentation][docs] for a detailed usage guide and
more information.

# Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
figment = { version = "0.10", features = ["env"] }
figment_file_env_provider = { version = "0.1" }
```

## License

Figment_file_env_provider is licensed under either of the MIT License ([LICENSE-MIT](LICENSE-MIT)
or http://opensource.org/licenses/MIT).
