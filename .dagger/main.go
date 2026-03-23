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

const npmCacheName = "sun-api-npm"

// Lint runs cargo fmt check, cargo check, and cargo clippy with warnings denied.
// Source directory defaults to the root of the repository.
func (m *SunApi) Lint(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
) (string, error) {
	rust := dag.Rust().DevContainer(dagger.RustDevContainerOpts{
		ToolchainFile: source.File("rust-toolchain.toml"),
		Source:        source,
	})

	checkOut, err := rust.CargoCheck(ctx)
	if err != nil {
		return checkOut, err
	}
	fmtOut, err := rust.CargoFmtCheck(ctx)
	if err != nil {
		return fmtOut, err
	}
	clippyOut, err := rust.CargoClippy(ctx)
	if err != nil {
		return clippyOut, err
	}
	return checkOut + fmtOut + clippyOut, nil
}

// Test runs cargo test.
// Source directory defaults to the root of the repository.
func (m *SunApi) Test(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
) (string, error) {
	rust := dag.Rust().DevContainer(dagger.RustDevContainerOpts{
		ToolchainFile: source.File("rust-toolchain.toml"),
		Source:        source,
	})

	return rust.Container().
		WithExec([]string{"cargo", "test", "--all-targets", "--all-features"}).
		Stdout(ctx)
}

// Build compiles the sunapi binary in release mode and returns it.
// Source directory defaults to the root of the repository.
func (m *SunApi) Build(
	//+defaultPath="/"
	source *dagger.Directory,
) *dagger.File {
	return dag.Rust().DevContainer(dagger.RustDevContainerOpts{
		ToolchainFile: source.File("rust-toolchain.toml"),
		Source:        source,
	}).Container().
		WithExec([]string{"cargo", "build", "--release", "--package", "sunapi", "--bin", "sunapi"}).
		WithExec([]string{"cp", "/src/target/release/sunapi", "/tmp/sunapi"}).
		File("/tmp/sunapi")
}

// BuildContainer builds a minimal Wolfi-based container with the sunapi binary.
// Source directory defaults to the root of the repository.
func (m *SunApi) BuildContainer(
	//+defaultPath="/"
	source *dagger.Directory,
) *dagger.Container {
	binary := m.Build(source)

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

// Publish builds and publishes the container image to a registry.
// Source directory defaults to the root of the repository.
func (m *SunApi) Publish(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
	address string,
) (string, error) {
	return m.BuildContainer(source).Publish(ctx, address)
}

// cfWorkerContainer extends the Rust dev container with CF worker build tools.
// Source is mounted at /src by the Rust DevContainer.
func (m *SunApi) cfWorkerContainer(source *dagger.Directory) *dagger.Container {
	return dag.Rust().DevContainer(dagger.RustDevContainerOpts{
		ToolchainFile:     source.File("rust-toolchain.toml"),
		Source:            source,
		ExtraPackages:     []string{"nodejs-22", "npm", "clang", "wasm-tools", "worker-build"},
		ExtraRepositories: []string{"https://moosingin3space.github.io/wolfi-pkgs"},
		ExtraKeyUrls:      []string{"https://moosingin3space.github.io/wolfi-pkgs/melange.rsa.pub"},
	}).Container().
		WithWorkdir("/src/sunapi-cf").
		WithMountedCache("/src/sunapi-cf/node_modules", dag.CacheVolume(npmCacheName)).
		WithExec([]string{"npm", "install"})
}

// CfWorkerBuild builds the Cloudflare Worker without deploying.
// Source directory defaults to the root of the repository.
func (m *SunApi) CfWorkerBuild(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
) (string, error) {
	return m.cfWorkerContainer(source).
		WithExec([]string{"worker-build", "--release"}).
		Stdout(ctx)
}

// CfWorkerTest runs tests for the Cloudflare Worker.
// Source directory defaults to the root of the repository.
func (m *SunApi) CfWorkerTest(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
) (string, error) {
	return m.cfWorkerContainer(source).
		WithExec([]string{"npx", "wrangler", "deploy", "--dry-run"}).
		WithExec([]string{"npx", "vitest", "run"}).
		Stdout(ctx)
}

// DeployCfWorker deploys the Cloudflare Worker using Wrangler.
// Source directory defaults to the root of the repository.
func (m *SunApi) DeployCfWorker(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
	// Cloudflare API token for authentication
	cloudflareApiToken *dagger.Secret,
	// Cloudflare Account ID
	cloudflareAccountId *dagger.Secret,
) (string, error) {
	return m.cfWorkerContainer(source).
		WithSecretVariable("CLOUDFLARE_API_TOKEN", cloudflareApiToken).
		WithSecretVariable("CLOUDFLARE_ACCOUNT_ID", cloudflareAccountId).
		WithExec([]string{"npx", "wrangler", "deploy"}).
		Stdout(ctx)
}
