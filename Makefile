.PHONY: build release clean clear-history

# Settings path: macOS uses Application Support, Linux uses XDG config
UNAME := $(shell uname)
ifeq ($(UNAME), Darwin)
  SETTINGS := $(HOME)/Library/Application Support/winxmerge/settings.json
else
  SETTINGS := $(HOME)/.config/winxmerge/settings.json
endif

build:
	cargo build
	@$(MAKE) -s clear-history

release:
	cargo build --release
	@$(MAKE) -s clear-history

clear-history:
	@if [ -f "$(SETTINGS)" ]; then \
	  jq '.recent_files = [] | .session = []' "$(SETTINGS)" > "$(SETTINGS).tmp" \
	  && mv "$(SETTINGS).tmp" "$(SETTINGS)" \
	  && echo "[winxmerge] comparison history cleared" \
	  || rm -f "$(SETTINGS).tmp"; \
	fi

clean:
	cargo clean
