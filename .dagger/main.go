// Dagger module for Hubdash CI/CD pipelines.
//
// Lint, test, and deployment logic is delegated to shared CHARMD modules
// (rust and cf-worker) from the daggerverse. This module provides thin
// wrappers that supply hubdash-specific parameters (worker directory,
// toolchain file, cache volume names, etc.).
package main

import (
	"context"
	"dagger/hubdash/internal/dagger"
)

const pnpmStoreCacheName = "hubdash-pnpm-store"

type Hubdash struct{}

// Lint runs cargo fmt check, cargo check, and cargo clippy.
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

// Test runs the Rust test suite via cargo test.
// Source directory defaults to the root of the repository.
func (m *Hubdash) Test(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
) (string, error) {
	return dag.Rust().DevContainer(dagger.RustDevContainerOpts{
		ToolchainFile: source.File("rust-toolchain.toml"),
		Source:        source,
	}).CargoTest(ctx)
}

// CfWorkerTest builds and tests the hubdash-cf Cloudflare Worker.
// Source directory defaults to the root of the repository.
func (m *Hubdash) CfWorkerTest(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
) (string, error) {
	return dag.CharmdCfWorker().DevContainer(
		"hubdash-cf",
		dagger.CharmdCfWorkerDevContainerOpts{
			Source:          source,
			ToolchainFile:   source.File("rust-toolchain.toml"),
			PnpmCacheVolume: pnpmStoreCacheName,
		},
	).Test(ctx)
}

// DeployCfWorker deploys the hubdash-cf Cloudflare Worker using Wrangler.
// For production, omit prNumber.
// For preview, pass prNumber to upload a new version with alias "pr-<prNumber>",
// yielding a preview URL of the form pr-<N>-hubdash.<subdomain>.workers.dev.
// Source directory defaults to the root of the repository.
func (m *Hubdash) DeployCfWorker(
	ctx context.Context,
	//+defaultPath="/"
	source *dagger.Directory,
	// Cloudflare API token for authentication
	cloudflareApiToken *dagger.Secret,
	// Cloudflare Account ID
	cloudflareAccountId *dagger.Secret,
	// Pull request number. When set, uploads a versioned preview alias "pr-<prNumber>".
	//+optional
	prNumber string,
) (string, error) {
	ctr := dag.CharmdCfWorker().DevContainer(
		"hubdash-cf",
		dagger.CharmdCfWorkerDevContainerOpts{
			Source:          source,
			ToolchainFile:   source.File("rust-toolchain.toml"),
			PnpmCacheVolume: pnpmStoreCacheName,
		},
	)
	if prNumber != "" {
		return ctr.UploadVersion(ctx, cloudflareApiToken, cloudflareAccountId,
			dagger.CharmdCfWorkerUploadVersionOpts{PreviewAlias: "pr-" + prNumber})
	}
	return ctr.Deploy(ctx, cloudflareApiToken, cloudflareAccountId)
}
