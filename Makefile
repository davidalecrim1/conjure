.PHONY: dev build clean install lint fmt

dev:
	cargo tauri dev

build:
	cargo tauri build

install:
	npm install

clean:
	rm -rf src-tauri/target node_modules dist

lint:
	cd src-tauri && cargo clippy -- -D warnings
	npx tsc --noEmit

fmt:
	cd src-tauri && cargo fmt
	npx prettier --write src/
