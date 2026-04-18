.PHONY: fmt test clippy ci coverage sq sq-ci sonar run

SONAR_SCANNER ?= sonar-scanner
SONAR_ARGS ?= -Dsonar.qualitygate.wait=true

fmt:
	cargo fmt --check

test:
	cargo test --all-targets

clippy:
	cargo clippy --all-targets -- -D warnings

ci: fmt test clippy

coverage:
	@cargo llvm-cov --version >/dev/null 2>&1 || { \
		echo "cargo-llvm-cov is required. Install with: cargo install cargo-llvm-cov"; \
		exit 1; \
	}
	@rustup component list --installed | grep -q '^llvm-tools-' || { \
		echo "llvm-tools-preview is required. Install with: rustup component add llvm-tools-preview"; \
		exit 1; \
	}
	mkdir -p target/coverage
	rustup run stable cargo llvm-cov --all-targets --workspace --cobertura --output-path target/coverage/coverage.xml

sq-ci: coverage

sq:
	@test -n "$$SONAR_TOKEN" || { \
		echo "SONAR_TOKEN must be set in the environment."; \
		exit 1; \
	}
	@command -v $(SONAR_SCANNER) >/dev/null 2>&1 || { \
		echo "$(SONAR_SCANNER) is required for local SonarQube scans."; \
		exit 1; \
	}
	@$(MAKE) coverage
	$(SONAR_SCANNER) $(SONAR_ARGS)

sonar: sq

run:
	cargo run
