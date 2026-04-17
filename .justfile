set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

default:
    @just -l

[group('dev')]
[doc('Build the project')]
build config="debug":
    cargo build {{ if config == "release" { "--release" } else { "" } }}

[group('dev')]
[doc('Install the project')]
install:
    cargo install --path .

[group('dev')]
[doc('Format the code')]
fmt:
    cargo fmt --all

[group('dev')]
[doc('Lint the code')]
lint:
    cargo clippy -- -D warnings

[group('dev')]
[doc('Run tests')]
test config="debug":
    cargo test {{ if config == "release" { "--release" } else { "" } }}
