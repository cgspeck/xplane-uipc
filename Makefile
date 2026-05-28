# Makefile — Linux development workflow
#
# The plugin targets Windows but most checks can run on Linux:
#   - "portable" crates (uipc-expr, uipc-mapping, etc.) compile and test natively
#   - "windows" crates (xplane_uipc, ipc_host, uipc-debug) can be type-checked
#     via cross-compilation but cannot link or run tests
#
# Use `make` for day-to-day development on Linux.
# Use `cargo xtask dist/deploy` on Windows for release builds.

WIN_TARGET = x86_64-pc-windows-msvc

PORTABLE_CRATES = uipc-expr uipc-mapping expr-calculator xtask
WINDOWS_CRATES  = ipc_host uipc-debug xplane_uipc

.PHONY: all fmt test check clippy clean

## Run all checks: format, lint, cross-compile, and test
all: fmt clippy check test

## Check formatting across the workspace
fmt:
	cargo fmt --all -- --check

## Lint portable crates natively + Windows crates via cross-compilation
clippy:
	@for c in $(PORTABLE_CRATES); do \
		echo "── clippy $$c ──"; \
		cargo clippy -p $$c || exit 1; \
	done
	@for c in $(WINDOWS_CRATES); do \
		echo "── clippy $$c (cross → $(WIN_TARGET)) ──"; \
		cargo clippy --target $(WIN_TARGET) -p $$c || exit 1; \
	done

## Type-check Windows crates via cross-compilation (no linker needed)
check:
	@for c in $(WINDOWS_CRATES); do \
		echo "── check $$c (cross → $(WIN_TARGET)) ──"; \
		cargo check --target $(WIN_TARGET) -p $$c || exit 1; \
	done

## Run tests (portable crates only — Windows crates cannot execute on Linux)
test:
	@for c in $(PORTABLE_CRATES); do \
		echo "── test $$c ──"; \
		cargo test -p $$c || exit 1; \
	done

## Remove build artifacts
clean:
	cargo clean
