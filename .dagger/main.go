// Dagger module for Hubdash CI/CD pipelines
//
// This module provides linting, testing, building, and deployment functions
// for the hubdash project, including the Cloudflare Worker in hubdash-cf/.
package main

import (
	"context"
	"dagger/hubdash/internal/dagger"
)

const pnpmStoreCacheName = "hubdash-pnpm-store"

type Hubdash struct{}

// Runs cargo fmt check, cargo check, and cargo clippy with warnings denied.
// Source directory defaults to the root of the repository.
func (m *Hubdash) Lint(
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

// Runs cargo test.
// Source directory defaults to the root of the repository.
func (m *Hubdash) Test(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
) (string, error) {
	rust := dag.Rust().DevContainer(dagger.RustDevContainerOpts{
		ToolchainFile: source.File("rust-toolchain.toml"),
		Source:        source,
	})

	return rust.Container().
		WithExec([]string{"cargo", "test"}).
		Stdout(ctx)
}

// cfWorkerContainer extends the Rust dev container with CF worker build tools.
// Source is mounted at /src by the Rust DevContainer.
func (m *Hubdash) cfWorkerContainer(source *dagger.Directory) *dagger.Container {
	return dag.Rust().DevContainer(dagger.RustDevContainerOpts{
		ToolchainFile:     source.File("rust-toolchain.toml"),
		Source:            source,
		ExtraPackages:     []string{"nodejs-22", "npm", "clang", "wasm-tools", "worker-build"},
		ExtraRepositories: []string{"https://moosingin3space.github.io/wolfi-pkgs"},
		ExtraKeyUrls:      []string{"https://moosingin3space.github.io/wolfi-pkgs/melange.rsa.pub"},
	}).Container().
		WithExec([]string{"npm", "install", "-g", "pnpm@10.30.3"}).
		WithWorkdir("/src/hubdash-cf").
		WithMountedCache("/root/.local/share/pnpm/store", dag.CacheVolume(pnpmStoreCacheName)).
		WithEnvVariable("CI", "true").
		WithExec([]string{"pnpm", "install", "--frozen-lockfile"})
}

// CfWorkerBuild builds the Cloudflare Worker without deploying.
// Source directory defaults to the root of the repository.
func (m *Hubdash) CfWorkerBuild(
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
func (m *Hubdash) CfWorkerTest(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
) (string, error) {
	return m.cfWorkerContainer(source).
		WithExec([]string{"pnpm", "wrangler", "deploy", "--dry-run"}).
		WithExec([]string{"pnpm", "test"}).
		Stdout(ctx)
}

// DeployCfWorker deploys the Cloudflare Worker using Wrangler.
// Source directory defaults to the root of the repository.
func (m *Hubdash) DeployCfWorker(
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
		WithExec([]string{"pnpm", "run", "deploy"}).
		Stdout(ctx)
}
