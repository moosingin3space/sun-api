# GitHub Actions Guidelines

This document provides guidelines for writing and maintaining GitHub Actions workflows in this project.

## Workflow Validation

All GitHub Actions workflows must pass validation with [zizmor](https://github.com/woodruffw/zizmor), a security-focused linter for GitHub Actions workflows.

### Running zizmor

```bash
zizmor .github/workflows/
```

Run this command before committing any workflow changes to ensure they pass all security audits.

## Security Requirements

### Pin Actions to SHA

All action references must be pinned to their full commit SHA, not tags:

```yaml
# ✅ Good - pinned to SHA with version comment
- uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5 # v4

# ❌ Bad - using tag reference
- uses: actions/checkout@v4
```

### Minimize Permissions

Set minimal permissions at workflow and job levels:

```yaml
# Workflow level - deny all by default
permissions: {}

jobs:
  build:
    permissions:
      contents: read  # Only what's needed
```

### Disable Credential Persistence

Always set `persist-credentials: false` on checkout steps:

```yaml
- uses: actions/checkout@<sha>
  with:
    persist-credentials: false
```

## Dagger Integration

This project uses Dagger for CI/CD pipelines. See [dagger-guidelines.md](dagger-guidelines.md) for details on the Dagger module.

### Dagger Action Configuration

Use the `dagger/dagger-for-github` action with:
- Pinned SHA reference
- Explicit version
- Cloud token for tracing

```yaml
- name: Install Dagger
  uses: dagger/dagger-for-github@<sha> # v8.x
  with:
    version: "0.19.7"
    cloud-token: ${{ secrets.DAGGER_CLOUD_TOKEN }}
```

## Required Secrets

| Secret | Description |
|--------|-------------|
| `DAGGER_CLOUD_TOKEN` | Dagger Cloud token for traces |
| `CLOUDFLARE_API_TOKEN` | Cloudflare API token for worker deployment |
| `CLOUDFLARE_ACCOUNT_ID` | Cloudflare account ID |
