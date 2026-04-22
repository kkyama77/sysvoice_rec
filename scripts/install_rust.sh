#!/usr/bin/env bash
#
# install_rust.sh
#
# Install rustup, configure the default toolchain, add common components and targets,
# and optionally install some useful cargo utilities.
#
# Usage:
#   ./install_rust.sh [--toolchain <name>] [--components "comp1 comp2"] [--targets "t1 t2"] [--no-cargo-utils] [--help]
#
# Examples:
#   ./install_rust.sh
#   ./install_rust.sh --toolchain stable --components "rustfmt clippy" --targets "wasm32-unknown-unknown"
#
# Notes:
# - This script is intended for POSIX-like environments (Linux, macOS, WSL).
# - On native Windows (cmd/powershell), prefer the rustup installer from:
#     https://win.rustup.rs/
#
set -euo pipefail

# Defaults
TOOLCHAIN="stable"
COMPONENTS=("rustfmt" "clippy")
TARGETS=()
INSTALL_CARGO_UTILS=true
RUSTUP_INIT_URL="https://sh.rustup.rs"

# Helpers
info()    { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
warn()    { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error()   { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }
die()     { error "$@"; exit 1; }

usage() {
  cat <<EOF
Install rustup and configure Rust toolchain & components.

Usage:
  $0 [options]

Options:
  --toolchain <name>        Default toolchain to install (default: stable)
  --components "<list>"     Space-separated components to add (default: "rustfmt clippy")
  --targets "<list>"        Space-separated rust targets to add (example: "wasm32-unknown-unknown")
  --no-cargo-utils          Do not install cargo utilities (cargo-edit)
  --help                    Show this help and exit

Examples:
  $0
  $0 --toolchain nightly --components "rustfmt clippy" --targets "wasm32-unknown-unknown"
EOF
}

# Parse args (simple long-option parser)
while [[ $# -gt 0 ]]; do
  case "$1" in
    --toolchain)
      shift
      [[ $# -gt 0 ]] || die "--toolchain requires an argument"
      TOOLCHAIN="$1"
      shift
      ;;
    --components)
      shift
      [[ $# -gt 0 ]] || die "--components requires an argument"
      read -r -a COMPONENTS <<< "$1"
      shift
      ;;
    --targets)
      shift
      [[ $# -gt 0 ]] || die "--targets requires an argument"
      read -r -a TARGETS <<< "$1"
      shift
      ;;
    --no-cargo-utils)
      INSTALL_CARGO_UTILS=false
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      die "Unknown argument: $1. Use --help for usage."
      ;;
  esac
done

# Detect platform (crude)
OS="$(uname -s || echo Unknown)"
case "$OS" in
  Linux*)   PLATFORM=linux ;;
  Darwin*)  PLATFORM=darwin ;;
  MINGW*|MSYS*|CYGWIN*) PLATFORM=windows ;;
  *) PLATFORM=unknown ;;
esac

info "Detected platform: $OS"

if [[ "$PLATFORM" = "windows" ]]; then
  warn "This script targets POSIX shells. On native Windows you should use the official installer:"
  warn "  https://win.rustup.rs/"
  warn "If you're running MSYS2/MinGW or WSL this script may work; continuing."
fi

# Check for existing rustup
if command -v rustup >/dev/null 2>&1; then
  info "rustup is already installed. Updating rustup and toolchains..."
  rustup self update || warn "rustup self update failed; continuing"
  rustup update || warn "rustup update failed; continuing"
else
  # Ensure curl or wget exists
  if command -v curl >/dev/null 2>&1; then
    DL_CMD="curl -sSfL \"$RUSTUP_INIT_URL\" -o /tmp/rustup-init.sh"
  elif command -v wget >/dev/null 2>&1; then
    DL_CMD="wget -qO /tmp/rustup-init.sh \"$RUSTUP_INIT_URL\""
  else
    die "Neither curl nor wget found. Please install one of them and re-run this script."
  fi

  info "Downloading rustup-init from $RUSTUP_INIT_URL"
  # shellcheck disable=SC2086
  if ! sh -c "$DL_CMD"; then
    die "Failed to download rustup-init script."
  fi

  info "Running rustup-init (non-interactive)"
  # Use -y to accept defaults and --default-toolchain to set toolchain
  # We will source the environment file after installation.
  if ! sh /tmp/rustup-init.sh -y --default-toolchain "$TOOLCHAIN"; then
    die "rustup-init failed."
  fi
fi

# Source cargo env if present
CARGO_ENV="$HOME/.cargo/env"
if [[ -f "$CARGO_ENV" ]]; then
  # shellcheck disable=SC1090
  source "$CARGO_ENV"
  info "Sourced $CARGO_ENV"
else
  warn "$CARGO_ENV not found. You may need to add \$HOME/.cargo/bin to your PATH manually."
  warn "Example: export PATH=\"\$HOME/.cargo/bin:\$PATH\""
fi

# Ensure desired default toolchain
if command -v rustup >/dev/null 2>&1; then
  info "Setting default toolchain to '$TOOLCHAIN'"
  rustup default "$TOOLCHAIN" || warn "Failed to set default toolchain to $TOOLCHAIN"

  # Add components
  if [[ "${#COMPONENTS[@]}" -gt 0 ]]; then
    info "Adding components: ${COMPONENTS[*]}"
    for comp in "${COMPONENTS[@]}"; do
      if rustup component add "$comp" --toolchain "$TOOLCHAIN"; then
        info "Added component: $comp"
      else
        warn "Failed to add component: $comp"
      fi
    done
  fi

  # Add targets
  if [[ "${#TARGETS[@]}" -gt 0 ]]; then
    info "Adding targets: ${TARGETS[*]}"
    for tgt in "${TARGETS[@]}"; do
      if rustup target add "$tgt" --toolchain "$TOOLCHAIN"; then
        info "Added target: $tgt"
      else
        warn "Failed to add target: $tgt"
      fi
    done
  fi

  # Install some useful cargo utilities
  if [[ "$INSTALL_CARGO_UTILS" = true ]]; then
    # cargo-edit provides `cargo add`, `cargo rm`, `cargo upgrade` which are handy.
    if command -v cargo >/dev/null 2>&1; then
      if command -v cargo-add >/dev/null 2>&1 || cargo install --list | grep -q '^cargo-edit '; then
        info "cargo-edit appears to be installed already."
      else
        info "Installing cargo-edit (cargo utilities)"
        if cargo install cargo-edit; then
          info "cargo-edit installed"
        else
          warn "cargo install cargo-edit failed. You can install it later with: cargo install cargo-edit"
        fi
      fi
    else
      warn "cargo not found; skipping cargo utilities installation."
    fi
  else
    info "Skipping cargo utilities installation as requested."
  fi

else
  die "rustup is not available after installation. Please check the installer output."
fi

info "Rust install flow finished."

cat <<EOF

Next steps (if necessary):
  - Make sure \$HOME/.cargo/bin is on your PATH. Add to your shell rc profile if missing:
      export PATH="\$HOME/.cargo/bin:\$PATH"
  - If you use an IDE (VSCode, CLion, etc.) restart it so it sees the new toolchain.
  - To install additional tools:
      rustup component add rls rust-analysis rust-src
  - To remove rustup/toolchains:
      rustup self uninstall

EOF

exit 0
