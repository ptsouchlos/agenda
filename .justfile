# Set variables using shell commands
VERSION := if os_family() == "unix" { `git describe --tags --abbrev=0 2>/dev/null || echo "dev"` } else { `git describe --tags --abbrev=0 2>$null; if ($LASTEXITCODE -ne 0) { echo "dev" }` }
COMMIT := if os_family() == "unix" { `git rev-parse --short HEAD 2>/dev/null || echo "none"` } else { `git rev-parse --short HEAD 2>$null; if ($LASTEXITCODE -ne 0) { echo "none" }` }

set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

default:
    @just -l

[group('dev')]
[doc('Build the project')]
build:
    go build -ldflags "-X main.version={{VERSION}} -X main.commit={{COMMIT}}"

[group('dev')]
[doc('Install the project')]
install:
    go install -ldflags "-X main.version={{VERSION}} -X main.commit={{COMMIT}}" .

[group('dev')]
[doc('Format the code')]
fmt:
    gofmt -w .

[group('dev')]
[doc('Lint the code')]
lint:
    golangci-lint run

[group('dev')]
[doc('Run tests')]
test:
    go test ./...
