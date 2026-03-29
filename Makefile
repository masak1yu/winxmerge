.PHONY: build release clean clear-history

build:
	cargo build
	./target/debug/winxmerge --clear-history

release:
	cargo build --release
	./target/release/winxmerge --clear-history

clear-history:
	@if [ -f "./target/release/winxmerge" ]; then \
	  ./target/release/winxmerge --clear-history; \
	elif [ -f "./target/debug/winxmerge" ]; then \
	  ./target/debug/winxmerge --clear-history; \
	else \
	  echo "[winxmerge] binary not found — run 'make build' first"; \
	fi

clean:
	cargo clean
