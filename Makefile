PLUGIN := planeai-plugin-kiro-usage
DIST := dist/$(PLUGIN)
MANIFEST := package/planeai-plugin.json
UI := build/ui/entry.js
ICON := package/ui/assets/kiro-icon.svg
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

ifeq ($(UNAME_S),Darwin)
  ifeq ($(UNAME_M),arm64)
    PLATFORM := macos-arm64
  else
    PLATFORM := unsupported-macos
  endif
else
  PLATFORM := unsupported
endif

.PHONY: build-ui test package verify-package clean

build-ui:
	node scripts/build-ui.mjs

test: build-ui
	cargo test --locked
	node --test tests/*.test.mjs

package: build-ui
	@case "$(PLATFORM)" in unsupported-macos) echo "macOS x64 is unsupported; use Apple Silicon" >&2; exit 2;; unsupported) echo "Local packaging is supported only on macOS Apple Silicon in v1" >&2; exit 2;; esac
	cargo build --release --locked
	rm -rf $(DIST)
	mkdir -p $(DIST)/bin/$(PLATFORM) $(DIST)/ui/assets
	cp $(MANIFEST) $(DIST)/planeai-plugin.json
	cp $(UI) $(DIST)/ui/entry.js
	cp $(ICON) $(DIST)/ui/assets/kiro-icon.svg
	cp target/release/$(PLUGIN) $(DIST)/bin/$(PLATFORM)/$(PLUGIN)
	chmod +x $(DIST)/bin/$(PLATFORM)/$(PLUGIN)
	@echo "Staged $(DIST) for $(PLATFORM)"

verify-package: package
	node scripts/verify-package-handshake.mjs $(DIST) $(PLATFORM)

clean:
	rm -rf build dist
