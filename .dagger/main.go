// Dagger module for sun-api CI/CD pipelines.
//
// Lint, test, and deployment logic is delegated to shared CHARMD modules
// (rust and cf-worker) from the daggerverse. This module provides thin
// wrappers that supply sun-api-specific parameters, plus the binary container
// build and publish functions that are unique to this service.
package main

import (
	"context"
	"dagger/sun-api/internal/dagger"
)

const pnpmStoreCacheName = "sun-api-pnpm-store"

type SunApi struct{}

// Lint runs cargo fmt check, cargo check, and cargo clippy.
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

// Test runs the Rust test suite via cargo test.
// Source directory defaults to the root of the repository.
func (m *SunApi) Test(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
) (string, error) {
	return dag.Rust().DevContainer(dagger.RustDevContainerOpts{
		ToolchainFile: source.File("rust-toolchain.toml"),
		Source:        source,
	}).CargoTest(ctx, dagger.RustCargoTestOpts{
		ExtraArgs: []string{"--all-targets", "--all-features"},
	})
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

// CfWorkerTest builds and tests the sunapi-cf Cloudflare Worker.
// Source directory defaults to the root of the repository.
func (m *SunApi) CfWorkerTest(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
) (string, error) {
	return dag.CharmdCfWorker().DevContainer(
		"sunapi-cf",
		dagger.CharmdCfWorkerDevContainerOpts{
			Source:          source,
			ToolchainFile:   source.File("rust-toolchain.toml"),
			PnpmCacheVolume: pnpmStoreCacheName,
		},
	).Test(ctx)
}

// DeployCfWorker deploys the sunapi-cf Cloudflare Worker using Wrangler.
// Source directory defaults to the root of the repository.
func (m *SunApi) DeployCfWorker(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
	// Cloudflare API token for authentication
	cloudflareApiToken *dagger.Secret,
	// Cloudflare account ID
	cloudflareAccountId *dagger.Secret,
) (string, error) {
	return dag.CharmdCfWorker().DevContainer(
		"sunapi-cf",
		dagger.CharmdCfWorkerDevContainerOpts{
			Source:          source,
			ToolchainFile:   source.File("rust-toolchain.toml"),
			PnpmCacheVolume: pnpmStoreCacheName,
		},
	).Deploy(ctx, cloudflareApiToken, cloudflareAccountId)
}
