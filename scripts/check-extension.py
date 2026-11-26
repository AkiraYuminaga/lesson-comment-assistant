#!/usr/bin/env python3
"""Validate the Chrome extension: manifest structure, referenced files, and version sync."""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
EXT = ROOT / "extension"
errors: list[str] = []

manifest = json.loads((EXT / "manifest.json").read_text(encoding="utf-8"))

if manifest.get("manifest_version") != 3:
    errors.append("manifest_version must be 3")

for key in ("name", "version", "description"):
    if not manifest.get(key):
        errors.append(f"manifest is missing '{key}'")

# Every file the manifest points at must exist
paths = [manifest.get("background", {}).get("service_worker")]
for cs in manifest.get("content_scripts", []):
    paths += cs.get("js", []) + cs.get("css", [])
    if "<all_urls>" in cs.get("matches", []):
        errors.append("content_scripts must not match <all_urls>")
for war in manifest.get("web_accessible_resources", []):
    paths += war.get("resources", [])
paths.append(manifest.get("options_ui", {}).get("page"))
# Files the service worker registers / serves at runtime
paths += [
    "src/content/site-config.js",
    "src/content/content.js",
    "src/content/panel.html",
    "src/content/panel.css",
]
for p in filter(None, paths):
    if not (EXT / p).is_file():
        errors.append(f"manifest references missing file: {p}")

for key in ("host_permissions", "optional_host_permissions"):
    if "<all_urls>" in manifest.get(key, []):
        errors.append(f"{key} must not include <all_urls>")

# The school-system site is chosen in the options page; never hard-code it in the manifest
if manifest.get("content_scripts"):
    errors.append("content_scripts must stay empty: the site is registered from the options page")

# Extension and backend versions must match (one release = one version)
cargo = (ROOT / "backend" / "Cargo.toml").read_text(encoding="utf-8")
m = re.search(r'^version\s*=\s*"([^"]+)"', cargo, re.M)
backend_version = m.group(1) if m else None
if backend_version != manifest.get("version"):
    errors.append(
        f"version mismatch: extension {manifest.get('version')} vs backend {backend_version}"
    )

# Optional: a release tag passed as argv[1] must match too (e.g. v1.0.0)
if len(sys.argv) > 1:
    tag = sys.argv[1].removeprefix("refs/tags/").removeprefix("v")
    if tag != manifest.get("version"):
        errors.append(f"tag v{tag} does not match version {manifest.get('version')}")

if errors:
    for e in errors:
        print(f"::error::{e}")
    sys.exit(1)
print(f"extension OK (version {manifest['version']})")
