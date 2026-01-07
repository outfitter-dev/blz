#!/usr/bin/env python3
from __future__ import annotations

import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError as exc:  # pragma: no cover - Python <3.11
    raise SystemExit("Python 3.11+ is required for tomllib support.") from exc


def read_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def workspace_package_names(root: Path, cargo: dict) -> list[str]:
    members = cargo.get("workspace", {}).get("members", [])
    if not isinstance(members, list):
        return []

    names: list[str] = []
    for member in members:
        if not isinstance(member, str):
            continue
        manifest = root / member / "Cargo.toml"
        if not manifest.exists():
            continue
        data = read_toml(manifest)
        name = data.get("package", {}).get("name")
        if isinstance(name, str):
            names.append(name)
    return names


def sync_versions(lock_path: Path, packages: set[str], version: str) -> bool:
    lines = lock_path.read_text(encoding="utf-8").splitlines()
    updated = False
    current_pkg: str | None = None
    output: list[str] = []

    for line in lines:
        if line.startswith("[[package]]"):
            current_pkg = None
        elif line.startswith('name = "'):
            parts = line.split('"')
            if len(parts) >= 3:
                current_pkg = parts[1]

        if current_pkg in packages and line.startswith('version = "'):
            parts = line.split('"')
            if len(parts) >= 3 and parts[1] != version:
                line = f'version = "{version}"'
                updated = True

        output.append(line)

    if updated:
        lock_path.write_text("\n".join(output) + "\n", encoding="utf-8")

    return updated


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    cargo_path = root / "Cargo.toml"
    lock_path = root / "Cargo.lock"

    if not cargo_path.exists():
        print("Cargo.toml not found.")
        return 1
    if not lock_path.exists():
        print("Cargo.lock not found.")
        return 1

    cargo = read_toml(cargo_path)
    version = cargo.get("workspace", {}).get("package", {}).get("version")
    if not isinstance(version, str) or not version:
        print("Cargo.toml workspace.package.version is missing.")
        return 1

    packages = set(workspace_package_names(root, cargo))
    if not packages:
        print("No workspace package names found; skipping.")
        return 0

    updated = sync_versions(lock_path, packages, version)
    if updated:
        print(f"Updated Cargo.lock workspace package versions to {version}.")
    else:
        print("Cargo.lock already matches workspace package version.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
