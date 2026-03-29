.PHONY: build release clean clear-history

SETTINGS := $(HOME)/.config/winxmerge/settings.json

build:
	cargo build
	@$(MAKE) -s clear-history

release:
	cargo build --release
	@$(MAKE) -s clear-history

clear-history:
	@if [ -f "$(SETTINGS)" ]; then \
	  python3 -c " \
import json, sys; \
p = '$(SETTINGS)'; \
d = json.load(open(p)); \
d['recent_files'] = []; \
d['session'] = []; \
json.dump(d, open(p, 'w'), indent=2, ensure_ascii=False); \
print('[winxmerge] comparison history cleared'); \
	  " 2>/dev/null || true; \
	fi

clean:
	cargo clean
