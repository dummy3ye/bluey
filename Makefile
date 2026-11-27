PREFIX ?= /usr/local
BIN    := bluey

.PHONY: build release run fmt lint test check install uninstall dist clean

build:
	cargo build

release:
	cargo build --release --locked

run:
	cargo run

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test

check:
	cargo fmt --all -- --check
	$(MAKE) lint
	$(MAKE) test

install: release
	install -Dm755 target/release/$(BIN) $(DESTDIR)$(PREFIX)/bin/$(BIN)
	install -Dm644 packaging/bluey.desktop $(DESTDIR)$(PREFIX)/share/applications/bluey.desktop

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/$(BIN)
	rm -f $(DESTDIR)$(PREFIX)/share/applications/bluey.desktop

dist: release
	mkdir -p dist
	tar -C target/release -czf dist/$(BIN)-linux-x86_64.tar.gz $(BIN)

clean:
	cargo clean
