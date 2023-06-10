# options
ignore_output = &> /dev/null

.PHONY: run-dev test build

run-dev:
	@CONFIG_FILE_PATH=./config/default.toml cargo run

test:
	@cargo test -- --nocapture --include-ignored

lint:
	@cargo clippy --all-targets --all-features

fix:
	@cargo clippy --fix --bin "jarvis-agent" --tests

build:
	@DOCKER_BUILDKIT=1 docker build --output target .
