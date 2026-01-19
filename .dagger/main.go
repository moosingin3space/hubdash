// A generated module for Hubdash functions
//
// This module has been generated via dagger init and serves as a reference to
// basic module structure as you get started with Dagger.
//
// Two functions have been pre-created. You can modify, delete, or add to them,
// as needed. They demonstrate usage of arguments and return types using simple
// echo and grep commands. The functions can be called from the dagger CLI or
// from one of the SDKs.
//
// The first line in this comment block is a short description line and the
// rest is a long description with more detail on the module's purpose or usage,
// if appropriate. All modules should have a short description.

package main

import (
	"context"
	"dagger/hubdash/internal/dagger"
)

type Hubdash struct{}

// Runs cargo check and cargo fmt.
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
	combinedOut := checkOut + fmtOut
	return combinedOut, err
}
