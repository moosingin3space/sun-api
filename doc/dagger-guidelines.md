# Dagger CI/CD Guidelines

This project uses [Dagger](https://dagger.io) for portable CI/CD pipelines. All pipelines are defined in `.dagger/main.go` using the Go SDK and use Wolfi-based containers for minimal, secure builds.

## Available Dagger Functions

### Linting

| Function | Description |
|----------|-------------|
| `lint`   | Runs all linting checks (fmt, check, clippy) |
| `fmt`    | Runs `cargo fmt --check` |
| `check`  | Runs `cargo check` |
| `clippy` | Runs `cargo clippy` with warnings denied |

```bash
# Run all linting
dagger call lint --source=.

# Run individual checks
dagger call fmt --source=.
dagger call check --source=.
dagger call clippy --source=.
```

### Testing

```bash
dagger call test --source=.
```

### Building

| Function | Description |
|----------|-------------|
| `build` | Builds the sunapi binary (returns a file) |
| `build-container` | Builds a minimal Wolfi container with the binary |
| `publish` | Builds and publishes the container to a registry |

```bash
# Build the binary
dagger call build --source=. export --path=./sunapi

# Build the container
dagger call build-container --source=.

# Publish to a registry
dagger call publish --source=. --address=ghcr.io/myrepo/sunapi:latest
```

### Cloudflare Worker Deployment

| Function | Description |
|----------|-------------|
| `cf-worker-build` | Builds the Cloudflare Worker without deploying |
| `deploy-cf-worker` | Deploys the Cloudflare Worker using Wrangler |

```bash
# Build the worker
dagger call cf-worker-build --source=.

# Deploy (requires Cloudflare API token)
dagger call deploy-cf-worker \
  --source=. \
  --cloudflare-api-token=env:CLOUDFLARE_API_TOKEN \
  --cloudflare-account-id=YOUR_ACCOUNT_ID
```

## GitHub Actions Workflows

### PR Workflow (`.github/workflows/pr.yml`)

Runs on every pull request to `main`:
- **Lint**: Runs all linting checks
- **Test**: Runs cargo test
- **Build**: Builds the binary and container
- **Build CF Worker**: Verifies the Cloudflare Worker builds

### Main Branch Workflow (`.github/workflows/main.yml`)

Runs on every push to `main`:
- **Lint**: Runs all linting checks
- **Test**: Runs cargo test
- **Build and Publish**: Builds and pushes the container to GHCR (requires lint/test to pass)
- **Deploy CF Worker**: Deploys to Cloudflare (requires lint/test to pass, runs in `production` environment)

## Required Secrets

Configure these in your GitHub repository settings:

| Secret | Description |
|--------|-------------|
| `CLOUDFLARE_API_TOKEN` | Cloudflare API token with Workers deployment permissions |
| `CLOUDFLARE_ACCOUNT_ID` | Your Cloudflare account ID |

## Container Images

The `build-container` function creates a minimal Wolfi-based container with:
- `ca-certificates-bundle` for HTTPS support
- `libgcc` for Rust runtime
- The `sunapi` binary as entrypoint
- Port 3000 exposed

## Development

To modify the Dagger module:

```bash
cd .dagger

# Regenerate after changes
dagger develop

# List available functions
dagger functions
```
