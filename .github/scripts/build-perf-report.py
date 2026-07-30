#!/usr/bin/env python3

"""Combine flamegraph SVGs (and the critcmp table) into one self-contained HTML file."""

from __future__ import annotations

import argparse
import html
import pathlib
import re
import subprocess
import sys

SVG_RE = re.compile(r"<svg\b.*</svg>", re.DOTALL | re.IGNORECASE)


def extract_svg(path: pathlib.Path) -> str:
    text = path.read_text(encoding="utf-8", errors="replace")
    match = SVG_RE.search(text)
    if not match:
        raise SystemExit(f"no <svg> element in {path}")
    svg = match.group(0)
    # Drop fixed size so CSS can fill the viewport.
    svg = re.sub(r'\s(width|height)="[^"]*"', "", svg, count=2)
    return svg


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out-dir", type=pathlib.Path, required=True)
    parser.add_argument("--base-sha", required=True)
    parser.add_argument("--head-sha", required=True)
    parser.add_argument("--run-url", required=True)
    parser.add_argument("--repo-dir", type=pathlib.Path, required=True)
    args = parser.parse_args()

    table = subprocess.run(
        ["critcmp", "base", "head"],
        check=True,
        cwd=args.repo_dir,
        capture_output=True,
        text=True,
    ).stdout.strip()

    svgs = sorted(args.out_dir.glob("*.svg"))
    if not svgs:
        print(f"no .svg files in {args.out_dir}", file=sys.stderr)
        return 1

    sections: list[str] = []
    for path in svgs:
        sections.append(
            f'<section id="{html.escape(path.stem)}">\n'
            f"<h2>{html.escape(path.name)}</h2>\n"
            f"{extract_svg(path)}\n"
            f"</section>"
        )

    toc = "\n".join(
        f'<li><a href="#{html.escape(path.stem)}">{html.escape(path.name)}</a></li>'
        for path in svgs
    )

    doc = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Performance report</title>
<style>
  html, body {{ margin: 0; color: #111; font-family: system-ui, sans-serif; }}
  .intro {{ padding: 1.5rem; }}
  pre {{ background: #f4f4f4; padding: 1rem; overflow: auto; }}
  nav ul {{ columns: 2; }}
  section {{ border-top: 1px solid #ddd; }}
  section h2 {{ margin: 0; padding: 0.75rem 1.5rem; background: #f4f4f4; }}
  section svg {{ display: block; width: 100%; height: 100vh; }}
</style>
</head>
<body>
<div class="intro">
<h1>Performance report</h1>
<p><code>{html.escape(args.base_sha)}</code> (base) →
<code>{html.escape(args.head_sha)}</code> (head)</p>
<p>Run: <a href="{html.escape(args.run_url)}">{html.escape(args.run_url)}</a></p>
<h2>Benchmarks</h2>
<pre>{html.escape(table)}</pre>
<nav>
<h2>Flamegraphs</h2>
<ul>
{toc}
</ul>
</nav>
</div>
{chr(10).join(sections)}
</body>
</html>
"""

    html_path = args.out_dir / "perf-report.html"
    html_path.write_text(doc, encoding="utf-8")

    # PR comment body: numbers + pointer to the HTML artifact.
    report_md = f"""## Performance report

`{args.base_sha}` (base) → `{args.head_sha}` (head)

```
{table}
```

Flamegraphs: download `perf-report.html` artifact from [this run]({args.run_url}) and open it in a browser.
"""
    (args.out_dir / "report.md").write_text(report_md, encoding="utf-8")
    print(report_md)
    return 0


if __name__ == "__main__":
    sys.exit(main())
