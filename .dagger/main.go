// Dagger module for sun-api CI/CD pipelines
//
// This module provides linting, building, and deployment functions for the
// sun-api project using Wolfi-based containers for minimal, secure builds.
package main

import (
	"context"
	"dagger/sun-api/internal/dagger"
)

type SunApi struct{}

// Cache volume names
const (
	cargoCacheName  = "sun-api-cargo"
	rustupCacheName = "sun-api-rustup"
	targetCacheName = "sun-api-target"
	npmCacheName    = "sun-api-npm"
)

// withRustToolchain adds the Rust toolchain to a container with caching
func (m *SunApi) withRustToolchain(ctr *dagger.Container) *dagger.Container {
	return ctr.
		WithMountedCache("/root/.cargo", dag.CacheVolume(cargoCacheName)).
		WithMountedCache("/root/.rustup", dag.CacheVolume(rustupCacheName)).
		WithEnvVariable("CARGO_HOME", "/root/.cargo").
		WithEnvVariable("RUSTUP_HOME", "/root/.rustup").
		WithExec([]string{"rustup-init", "-y", "--default-toolchain", "stable"}).
		WithEnvVariable("PATH", "/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
}

// withSource mounts the source directory and sets the workdir with target caching
func (m *SunApi) withSource(ctr *dagger.Container, source *dagger.Directory) *dagger.Container {
	return ctr.
		WithMountedDirectory("/src", source).
		WithMountedCache("/src/target", dag.CacheVolume(targetCacheName)).
		WithWorkdir("/src")
}

// rustContainer returns a Wolfi container with Rust toolchain installed
func (m *SunApi) rustContainer() *dagger.Container {
	return m.withRustToolchain(dag.Wolfi().Container(dagger.WolfiContainerOpts{
		Packages: []string{
			"rustup",
			"build-base",
			"openssl-dev",
			"pkgconf",
			"curl",
		},
	}))
}

// Lint runs cargo fmt check, cargo check, and cargo clippy with warnings denied
func (m *SunApi) Lint(ctx context.Context, source *dagger.Directory) (string, error) {
	return m.withSource(m.rustContainer(), source).
		WithExec([]string{"rustup", "component", "add", "rustfmt", "clippy"}).
		WithExec([]string{"cargo", "fmt", "--all", "--check"}).
		WithExec([]string{"cargo", "check", "--all-targets", "--all-features"}).
		WithExec([]string{"cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"}).
		Stdout(ctx)
}

// Fmt runs cargo fmt check
func (m *SunApi) Fmt(ctx context.Context, source *dagger.Directory) (string, error) {
	return m.withSource(m.rustContainer(), source).
		WithExec([]string{"rustup", "component", "add", "rustfmt"}).
		WithExec([]string{"cargo", "fmt", "--all", "--check"}).
		Stdout(ctx)
}

// Check runs cargo check
func (m *SunApi) Check(ctx context.Context, source *dagger.Directory) (string, error) {
	return m.withSource(m.rustContainer(), source).
		WithExec([]string{"cargo", "check", "--all-targets", "--all-features"}).
		Stdout(ctx)
}

// Clippy runs cargo clippy with warnings denied
func (m *SunApi) Clippy(ctx context.Context, source *dagger.Directory) (string, error) {
	return m.withSource(m.rustContainer(), source).
		WithExec([]string{"rustup", "component", "add", "clippy"}).
		WithExec([]string{"cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"}).
		Stdout(ctx)
}

// Test runs cargo test
func (m *SunApi) Test(ctx context.Context, source *dagger.Directory) (string, error) {
	return m.withSource(m.rustContainer(), source).
		WithExec([]string{"cargo", "test", "--all-targets", "--all-features"}).
		Stdout(ctx)
}

// Build compiles the sunapi binary in release mode and returns it
func (m *SunApi) Build(ctx context.Context, source *dagger.Directory) *dagger.File {
	return m.withSource(m.rustContainer(), source).
		WithExec([]string{"cargo", "build", "--release", "--package", "sunapi", "--bin", "sunapi"}).
		WithExec([]string{"cp", "/src/target/release/sunapi", "/tmp/sunapi"}).
		File("/tmp/sunapi")
}

// BuildContainer builds a minimal Wolfi-based container with the sunapi binary
func (m *SunApi) BuildContainer(ctx context.Context, source *dagger.Directory) *dagger.Container {
	binary := m.Build(ctx, source)

	return dag.Wolfi().Container(dagger.WolfiContainerOpts{
		Packages: []string{
			"ca-certificates-bundle",
			"libgcc",
		},
	}).
		WithFile("/usr/local/bin/sunapi", binary).
		WithEntrypoint([]string{"/usr/local/bin/sunapi"}).
		WithExposedPort(3000)
}

// Publish builds and publishes the container image to a registry
func (m *SunApi) Publish(ctx context.Context, source *dagger.Directory, address string) (string, error) {
	return m.BuildContainer(ctx, source).Publish(ctx, address)
}

// wranglerContainer returns a Wolfi container with Wrangler, Rust (for worker-build), and npm installed
func (m *SunApi) wranglerContainer() *dagger.Container {
	return m.withRustToolchain(dag.Wolfi().Container(dagger.WolfiContainerOpts{
		Packages: []string{
			"nodejs",
			"npm",
			"rustup",
			"build-base",
			"clang",
			"wasm-tools",
			"curl",
		},
	})).
		WithExec([]string{"rustup", "target", "add", "wasm32-unknown-unknown"}).
		WithExec([]string{"cargo", "install", "worker-build"})
}

// withCfWorkerSource mounts source and sets workdir to the CF worker directory with npm caching
func (m *SunApi) withCfWorkerSource(ctr *dagger.Container, source *dagger.Directory) *dagger.Container {
	return m.withSource(ctr, source).
		WithWorkdir("/src/sunapi-cf").
		WithMountedCache("/src/sunapi-cf/node_modules", dag.CacheVolume(npmCacheName)).
		WithExec([]string{"npm", "install"})
}

// DeployCfWorker deploys the Cloudflare Worker using Wrangler
func (m *SunApi) DeployCfWorker(
	ctx context.Context,
	source *dagger.Directory,
	// Cloudflare API token for authentication
	cloudflareApiToken *dagger.Secret,
	// Cloudflare Account ID
	cloudflareAccountId *dagger.Secret,
) (string, error) {
	return m.withCfWorkerSource(m.wranglerContainer(), source).
		WithSecretVariable("CLOUDFLARE_API_TOKEN", cloudflareApiToken).
		WithSecretVariable("CLOUDFLARE_ACCOUNT_ID", cloudflareAccountId).
		WithExec([]string{"npx", "wrangler", "deploy"}).
		Stdout(ctx)
}

// CfWorkerBuild builds the Cloudflare Worker without deploying
func (m *SunApi) CfWorkerBuild(ctx context.Context, source *dagger.Directory) (string, error) {
	return m.withCfWorkerSource(m.wranglerContainer(), source).
		WithExec([]string{"worker-build", "--release"}).
		Stdout(ctx)
}

// CfWorkerTest runs tests for the Cloudflare Worker
func (m *SunApi) CfWorkerTest(ctx context.Context, source *dagger.Directory) (string, error) {
	return m.withCfWorkerSource(m.wranglerContainer(), source).
		WithExec([]string{"npx", "wrangler", "deploy", "--dry-run"}).
		WithExec([]string{"npx", "vitest", "run"}).
		Stdout(ctx)
}
