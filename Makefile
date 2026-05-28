WIN_TARGET = x86_64-pc-windows-msvc

# Crates that compile and test natively on Linux
PORTABLE_CRATES = uipc-expr uipc-mapping expr-calculator xtask

# Crates that require Windows (cross-check only, no linking/testing)
WINDOWS_CRATES = ipc_host uipc-debug xplane_uipc

.PHONY: all check test fmt clippy check-windows clippy-windows clean

## Default: format, lint, and test everything possible on this host
all: fmt clippy test

## Format all workspace crates
fmt:
	cargo fmt --all -- --check

## Clippy portable crates (native)
clippy:
	@for c in $(PORTABLE_CRATES); do \
		echo "── clippy $$c ──"; \
		cargo clippy -p $$c || exit 1; \
	done

## Compile-check Windows crates via cross-compilation (no linker needed)
check-windows:
	@for c in $(WINDOWS_CRATES); do \
		echo "── check $$c ($(WIN_TARGET)) ──"; \
		cargo check --target $(WIN_TARGET) -p $$c || exit 1; \
	done

## Clippy Windows crates via cross-compilation
clippy-windows:
	@for c in $(WINDOWS_CRATES); do \
		echo "── clippy $$c ($(WIN_TARGET)) ──"; \
		cargo clippy --target $(WIN_TARGET) -p $$c || exit 1; \
	done

## Compile-check all crates possible (native + cross)
check: check-windows
	@for c in $(PORTABLE_CRATES); do \
		echo "── check $$c ──"; \
		cargo check -p $$c || exit 1; \
	done

## Run tests for portable crates (only these can run on Linux)
test:
	@for c in $(PORTABLE_CRATES); do \
		echo "── test $$c ──"; \
		cargo test -p $$c || exit 1; \
	done

## Clean build artifacts
clean:
	cargo clean
