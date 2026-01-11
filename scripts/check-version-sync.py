#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError as exc:  # pragma: no cover - Python <3.11
    raise SystemExit("Python 3.11+ is required for tomllib support.") from exc


def read_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


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


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    package_path = root / "package.json"
    cargo_path = root / "Cargo.toml"
    manifest_path = root / ".release-please-manifest.json"
    plugin_path = root / ".claude-plugin" / "plugin.json"
    lock_path = root / "Cargo.lock"

    errors: list[str] = []

    try:
        package = read_json(package_path)
    except FileNotFoundError:
        errors.append("package.json not found.")
        package = {}

    try:
        cargo = read_toml(cargo_path)
    except FileNotFoundError:
        errors.append("Cargo.toml not found.")
        cargo = {}

    try:
        manifest = read_json(manifest_path)
    except FileNotFoundError:
        errors.append(".release-please-manifest.json not found.")
        manifest = {}

    try:
        plugin = read_json(plugin_path)
    except FileNotFoundError:
        errors.append(".claude-plugin/plugin.json not found.")
        plugin = {}

    try:
        lock = read_toml(lock_path)
    except FileNotFoundError:
        errors.append("Cargo.lock not found.")
        lock = {}

    package_version = package.get("version")
    cargo_version = (
        cargo.get("workspace", {}).get("package", {}).get("version")
        if isinstance(cargo, dict)
        else None
    )
    manifest_version = manifest.get(".") if isinstance(manifest, dict) else None
    plugin_version = plugin.get("version") if isinstance(plugin, dict) else None

    if not package_version:
        errors.append("package.json version is missing.")
    if not cargo_version:
        errors.append("Cargo.toml workspace.package.version is missing.")
    if not plugin_version:
        errors.append(".claude-plugin/plugin.json version is missing.")

    if package_version and cargo_version and package_version != cargo_version:
        errors.append(
            f"package.json version ({package_version}) does not match "
            f"Cargo.toml workspace.package.version ({cargo_version})."
        )

    if manifest_version and cargo_version and manifest_version != cargo_version:
        errors.append(
            f".release-please-manifest.json version ({manifest_version}) does not match "
            f"Cargo.toml workspace.package.version ({cargo_version})."
        )

    if plugin_version and cargo_version and plugin_version != cargo_version:
        errors.append(
            f".claude-plugin/plugin.json version ({plugin_version}) does not match "
            f"Cargo.toml workspace.package.version ({cargo_version})."
        )

    workspace_deps = cargo.get("workspace", {}).get("dependencies", {})
    if isinstance(workspace_deps, dict) and cargo_version:
        for dep in ("blz-core", "blz-mcp", "blz-cli", "blz-registry-build"):
            dep_entry = workspace_deps.get(dep)
            if isinstance(dep_entry, dict):
                dep_version = dep_entry.get("version")
                if dep_version and dep_version != cargo_version:
                    errors.append(
                        f"Cargo.toml workspace dependency {dep} "
                        f"version ({dep_version}) does not match workspace.package.version "
                        f"({cargo_version})."
                    )

    if isinstance(lock, dict) and cargo_version and cargo:
        packages = {pkg.get("name"): pkg.get("version") for pkg in lock.get("package", []) if isinstance(pkg, dict)}
        for name in workspace_package_names(root, cargo):
            lock_version = packages.get(name)
            if lock_version and lock_version != cargo_version:
                errors.append(
                    f"Cargo.lock package {name} version ({lock_version}) does not match "
                    f"workspace.package.version ({cargo_version})."
                )

    if errors:
        print("Version sync check failed:")
        for error in errors:
            print(f"- {error}")
        print("\nAlign versions across Cargo.toml, package.json, and manifest before merging.")
        return 1

    print("Version sync check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
