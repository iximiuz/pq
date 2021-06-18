.PHONY:
build:
	cargo build

.PHONY:
test-all:
	cargo test

.PHONY:
test-e2e:
	cargo test --test e2e -- --nocapture 
