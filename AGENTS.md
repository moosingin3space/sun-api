# Overall guidelines

The `sun-api` is written in Rust and is intended to be used as a REST api
for querying information about the sunrise, sunset, and solar noon times at
a given location.

# Outline

- The `solarcalc` crate provides functions for calculating sunrise, sunset,
  and solar noon times. This crate has purely functional implementations of
  these calculations, and can be integrated anywhere.
- The `sunapi` crate uses the `solarcalc` crate to provide a REST API for
  querying information about the sunrise, sunset, and solar noon times at
  a given location. This API uses the `axum` crate to provide its endpoints.
- The `sunapi-cf` crate implements a Cloudflare Worker based on the `sunapi`
  crate.

# Project Infrastructure

This project uses the following supporting tools:

- GitHub Actions for executing continuous integration and deployment.
- Dagger for defining portable CI/CD pipelines and executing tasks.

# Specific guidelines

See @doc/*.md and @specs/**/*.md for more information.

Individual crates have documentation at @**/README.md.
