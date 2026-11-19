# Nothing earbuds for Linux.
#
# Run `make` (or `make help`) to list the targets.
#
# Install locations can be changed on the command line, for example:
#   make install PREFIX=/usr/local
#   make install DESTDIR=/tmp/package-root

PREFIX        ?= $(HOME)/.local
XDG_DATA_HOME ?= $(HOME)/.local/share

BIN_DIR     := $(DESTDIR)$(PREFIX)/bin
APP_DIR     := $(DESTDIR)$(XDG_DATA_HOME)/applications
ICON_DIR    := $(DESTDIR)$(XDG_DATA_HOME)/icons/hicolor/scalable/apps
RELEASE_DIR := target/release
BINARIES    := nothing-ear nothing-ctl

.DEFAULT_GOAL := help
.PHONY: help install uninstall build run fmt test release refresh-menu

install: release ## Build and install the tray app and the CLI for the current user
	install -Dm755 $(RELEASE_DIR)/nothing-ear $(BIN_DIR)/nothing-ear
	install -Dm755 $(RELEASE_DIR)/nothing-ctl $(BIN_DIR)/nothing-ctl
	install -Dm644 packaging/nothing-ear.desktop $(APP_DIR)/nothing-ear.desktop
	install -Dm644 packaging/nothing-ear.svg $(ICON_DIR)/nothing-ear.svg
	@$(MAKE) --no-print-directory refresh-menu
	@case ":$$PATH:" in *":$(PREFIX)/bin:"*) ;; *) echo "Warning: $(PREFIX)/bin is not in your PATH." ;; esac
	@echo "Installed nothing-ear (tray app) and nothing-ctl (command line) in $(BIN_DIR)."

uninstall: ## Remove everything `make install` added
	rm -f $(addprefix $(BIN_DIR)/,$(BINARIES))
	rm -f $(APP_DIR)/nothing-ear.desktop
	rm -f $(ICON_DIR)/nothing-ear.svg
	@$(MAKE) --no-print-directory refresh-menu
	@echo "Uninstalled."

build: ## Build everything in debug mode
	cargo build --workspace

run: ## Start the tray app from the sources (debug logs: RUST_LOG=debug make run)
	cargo run -p nothing-tray

fmt: ## Format the code
	cargo fmt --all

test: ## Run the unit tests
	cargo test --workspace

help: ## List the targets
	@echo "Usage: make <target>"
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z_-]+:.*## / {printf "  %-12s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# --- Internal steps (not listed in `make help`) ------------------------------

# Optimised build used by `make install`.
release:
	cargo build --release --workspace

# Refreshes the application menu cache. Skipped when staging a package
# (DESTDIR set): the cache belongs to the target system.
refresh-menu:
	@if [ -z "$(DESTDIR)" ] && command -v update-desktop-database >/dev/null; then \
		update-desktop-database -q $(APP_DIR) || true; \
	fi
