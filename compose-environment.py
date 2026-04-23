#!/usr/bin/env python3
"""Compose per-component `environment.yaml` files into a single master env.

Conda doesn't natively compose environment files, so this script does it:
reads the listed `environment.yaml` files, merges channels (preserving
first-seen order) and dependencies (deduplicated), and writes a single
`master-environ.yaml`.

By default it reads:
  - lib/muudd/environment.yaml
  - lib/muudd/lib/kith-covenant/environment.yaml
  - lib/muudd/lib/autonomous_trust/environment.yaml

Conflicts (same package, different version specs — e.g. python==3.10.* vs
python==3.12.*) are *reported to stderr and exit 1*, because the caller
has to decide which version to pick.

Usage:
    ./compose-environment.py                         # default set
    ./compose-environment.py -o /tmp/my-env.yaml     # custom output
    ./compose-environment.py --name my-master-env    # custom env name
    ./compose-environment.py env1.yaml env2.yaml     # explicit file set
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    sys.stderr.write(
        "pyyaml not found. Install into your conda env (pyyaml is a default) or:\n"
        "  pip install pyyaml\n"
    )
    sys.exit(2)


# Relative to this script's directory (= lib/muudd).
DEFAULT_SOURCES = [
    "environment.yaml",
    "lib/kith-covenant/environment.yaml",
    "lib/autonomous_trust/environment.yaml",
]

# Match the leading alphanumeric package name, e.g. "numpy>=1.20" -> "numpy".
# conda also allows paths and urls, which we leave as-is.
_NAME_RE = re.compile(r"^([A-Za-z0-9_.\-]+)")


def package_name(spec: str) -> str:
    m = _NAME_RE.match(spec.strip())
    return m.group(1) if m else spec.strip()


def load_env(path: Path) -> dict:
    with path.open() as f:
        data = yaml.safe_load(f)
    return data or {}


def merge_ordered_unique(sequences):
    """Union preserving first-seen order across the input sequences."""
    out = []
    for seq in sequences:
        for item in seq:
            if item not in out:
                out.append(item)
    return out


def merge_dependencies(envs):
    """Merge the conda `dependencies:` (and nested `pip:`) lists across envs.

    envs is a list of (source_path, parsed_yaml) tuples.
    Returns (merged_list, ok). When ok is False, conflicts were printed to stderr.
    """
    conda_seen: dict[str, list] = {}
    pip_seen: dict[str, list] = {}

    for source, env in envs:
        for item in env.get("dependencies") or []:
            if isinstance(item, dict) and "pip" in item:
                for pip_spec in item.get("pip") or []:
                    if not isinstance(pip_spec, str):
                        sys.stderr.write(
                            f"warning: non-string pip entry in {source}: {pip_spec!r}\n"
                        )
                        continue
                    pip_seen.setdefault(package_name(pip_spec), []).append(
                        (pip_spec, source)
                    )
            elif isinstance(item, str):
                conda_seen.setdefault(package_name(item), []).append((item, source))
            else:
                sys.stderr.write(
                    f"warning: unexpected dependency entry in {source}: {item!r}\n"
                )

    ok = True
    merged_conda: list[str] = []
    for name in sorted(conda_seen):
        specs = {spec for spec, _ in conda_seen[name]}
        if len(specs) == 1:
            merged_conda.append(conda_seen[name][0][0])
        else:
            ok = False
            sys.stderr.write(f"conflict (conda): {name}\n")
            for spec, source in conda_seen[name]:
                sys.stderr.write(f"    {spec!r}  (from {source})\n")

    merged_pip: list[str] = []
    for name in sorted(pip_seen):
        specs = {spec for spec, _ in pip_seen[name]}
        if len(specs) == 1:
            merged_pip.append(pip_seen[name][0][0])
        else:
            ok = False
            sys.stderr.write(f"conflict (pip): {name}\n")
            for spec, source in pip_seen[name]:
                sys.stderr.write(f"    {spec!r}  (from {source})\n")

    merged: list = list(merged_conda)
    if merged_pip:
        merged.append({"pip": merged_pip})
    return merged, ok


def compose(sources, name):
    envs = [(p, load_env(p)) for p in sources]
    channels = merge_ordered_unique(env.get("channels") or [] for _, env in envs)
    deps, ok = merge_dependencies(envs)
    master = {"name": name}
    if channels:
        master["channels"] = channels
    master["dependencies"] = deps
    return master, ok


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Compose multiple environment.yaml files into one."
    )
    parser.add_argument(
        "files",
        nargs="*",
        help="environment.yaml files to merge (default: muudd + kith-covenant + autonomous_trust)",
    )
    parser.add_argument(
        "-o",
        "--output",
        default="master-environ.yaml",
        help="output path (default: master-environ.yaml in cwd)",
    )
    parser.add_argument(
        "--name",
        default="sustainable-space-master",
        help="name of the merged env (default: sustainable-space-master)",
    )
    args = parser.parse_args()

    script_dir = Path(__file__).resolve().parent
    if args.files:
        sources = [Path(f).resolve() for f in args.files]
    else:
        sources = [(script_dir / s).resolve() for s in DEFAULT_SOURCES]

    missing = [str(p) for p in sources if not p.exists()]
    if missing:
        sys.stderr.write("error: missing environment file(s):\n")
        for m in missing:
            sys.stderr.write(f"  {m}\n")
        return 1

    master, ok = compose(sources, args.name)

    if not ok:
        sys.stderr.write(
            "\ncomposition aborted: resolve the conflicts above (edit one of the source files) and re-run.\n"
        )
        return 1

    output = Path(args.output)
    if not output.is_absolute():
        output = Path.cwd() / output

    with output.open("w") as f:
        yaml.safe_dump(master, f, sort_keys=False, default_flow_style=False)

    n_conda = sum(1 for d in master["dependencies"] if isinstance(d, str))
    n_pip = 0
    for d in master["dependencies"]:
        if isinstance(d, dict) and "pip" in d:
            n_pip = len(d["pip"])
    print(
        f"wrote {output}  "
        f"({n_conda} conda + {n_pip} pip deps from {len(sources)} source files)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
