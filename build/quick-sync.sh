#!/usr/bin/env bash
# Windows(F:) 편집본 → ~/aegisrescue 빠른 동기화
set -euo pipefail
WIN_SRC="${1:-/mnt/f/ofntkd/aegisrescue}"
WSL_HOME="${HOME}/aegisrescue"
[[ -d "${WIN_SRC}" ]] || { echo "missing ${WIN_SRC}" >&2; exit 1; }
mkdir -p "${WSL_HOME}/build"
rsync -a "${WIN_SRC}/core/" "${WSL_HOME}/core/"
rsync -a "${WIN_SRC}/tools/" "${WSL_HOME}/tools/"
rsync -a "${WIN_SRC}/data/" "${WSL_HOME}/data/"
cp -f "${WIN_SRC}/build/wsl-build.sh" "${WSL_HOME}/build/wsl-build.sh"
chmod +x "${WSL_HOME}/build/wsl-build.sh"
for f in Cargo.toml Cargo.lock rust-toolchain.toml; do
  [[ -f "${WIN_SRC}/${f}" ]] && cp -f "${WIN_SRC}/${f}" "${WSL_HOME}/${f}"
done
echo "synced ${WIN_SRC} -> ${WSL_HOME}"
