# ProgressHub Development Commands

# Download nari-labs/Dia-1.6B using HTTP client - MUST show progress bars and write files
http:
    @echo "DOWNLOADING nari-labs/Dia-1.6B using HTTP client..."
    RUST_LOG=warn timeout 2h cargo run --bin progresshub -- \
        --model nari-labs/Dia-1.6B \
        --force
    @echo "HTTP download completed successfully"

# Download nari-labs/Dia-1.6B using HTTP client with TUI interface
http-tui:
    RUST_LOG=warn timeout 2h cargo run --bin progresshub -- \
        --model nari-labs/Dia-1.6B \
        --tui \
        --force