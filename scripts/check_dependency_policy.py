#!/usr/bin/env python3
"""Moonships GA dependency/source/license policy gate.

This intentionally avoids making legal conclusions. It enforces that every
third-party Rust package declares license metadata (or a license file), and that
non-registry Git dependencies are pinned to a concrete commit.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from collections import Counter


def main() -> int:
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--format-version", "1", "--locked"],
            text=True,
        )
    )

    missing_license: list[str] = []
    unpinned_git: list[str] = []
    licenses: Counter[str] = Counter()
    third_party = 0

    for pkg in metadata["packages"]:
        source = pkg.get("source")
        if source is None:
            # Workspace/path package.
            continue

        third_party += 1
        license_expr = (pkg.get("license") or "").strip()
        license_file = pkg.get("license_file")
        if not license_expr and not license_file:
            missing_license.append(f"{pkg['name']} {pkg['version']}")
        else:
            licenses[license_expr or "LICENSE-FILE"] += 1

        if source.startswith("git+"):
            # Cargo source IDs pin the resolved commit after '#'.
            if not re.search(r"#[0-9a-fA-F]{7,64}$", source):
                unpinned_git.append(f"{pkg['name']} {pkg['version']} -> {source}")

    print(f"third_party_packages={third_party}")
    print("license_summary:")
    for license_name, count in sorted(licenses.items()):
        print(f"  {count:4d}  {license_name}")

    if missing_license:
        print("\nERROR: dependencies without declared license metadata:", file=sys.stderr)
        for item in missing_license:
            print(f"  - {item}", file=sys.stderr)

    if unpinned_git:
        print("\nERROR: Git dependencies not pinned to a resolved commit:", file=sys.stderr)
        for item in unpinned_git:
            print(f"  - {item}", file=sys.stderr)

    if missing_license or unpinned_git:
        return 1

    print("\nDependency provenance/license metadata policy: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
