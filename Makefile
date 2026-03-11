.PHONY: dev build clean install npm-install lint fmt

dev:
	cargo tauri dev --config '{"productName":"Conjure Dev","identifier":"com.davidalecrim.conjure-dev"}'

build:
	cargo tauri build

npm-install:
	npm install

install:
	cp -Rf src-tauri/target/release/bundle/macos/Conjure.app /Applications/Conjure.app

clean:
	rm -rf src-tauri/target node_modules dist
	tccutil reset Accessibility com.davidalecrim.conjure
	tccutil reset Accessibility com.davidalecrim.conjure-dev

lint:
	cd src-tauri && cargo clippy -- -D warnings
	npx tsc --noEmit

fmt:
	cd src-tauri && cargo fmt
	npx prettier --write src/
