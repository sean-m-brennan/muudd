#!/usr/bin/env python3
#  Copyright 2026 Sean M. Brennan and contributors
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

"""Extract fenced mermaid blocks from the narrative chain and render each to PDF."""
import json
import os
import re
import subprocess
import sys

if len(sys.argv) != 3:
    sys.exit('usage: mermaid.py <muudd repository> <output directory>')
ROOT = os.path.realpath(sys.argv[1])
OUTDIR = sys.argv[2]
# mermaid-cli is installed in the build's work directory, so npx is run
# from there rather than from wherever this script was invoked
NPM_PREFIX = os.environ.get('NPM_PREFIX') or os.path.dirname(
    os.path.realpath(OUTDIR))
os.makedirs(OUTDIR, exist_ok=True)
order = json.load(open(os.path.join(ROOT, 'doc/narrative-order.json')))
paths = [order['entry']] + [d['path'] for p in order['parts'] for d in p['documents']]

# mermaid-cli drives headless Chrome through puppeteer. Where puppeteer's own
# browser download is unavailable, a Chromium that Playwright has already cached
# serves just as well, so it is used before giving up. Setting
# PUPPETEER_EXECUTABLE_PATH overrides both.
def find_chrome():
    if os.environ.get('PUPPETEER_EXECUTABLE_PATH'):
        return os.environ['PUPPETEER_EXECUTABLE_PATH']
    import glob
    for pat in ('~/.cache/ms-playwright/chromium-*/chrome-linux64/chrome',
                '~/.cache/ms-playwright/chromium_headless_shell-*/'
                'chrome-headless-shell-linux64/chrome-headless-shell'):
        hits = sorted(glob.glob(os.path.expanduser(pat)))
        if hits:
            return hits[-1]
    return ''


CHROME = find_chrome()
if not CHROME:
    sys.exit('no chromium found; mermaid diagrams cannot be rendered')
print('chromium: %s' % CHROME)
env = dict(os.environ, PUPPETEER_EXECUTABLE_PATH=CHROME)

cfg = os.path.join(OUTDIR, 'puppeteer.json')
open(cfg, 'w').write(json.dumps({"args": ["--no-sandbox", "--disable-dev-shm-usage"]}))
mmdcfg = os.path.join(OUTDIR, 'mermaid-config.json')
open(mmdcfg, 'w').write(json.dumps({
    "theme": "neutral",
    "themeVariables": {"fontFamily": "DejaVu Sans, sans-serif", "fontSize": "15px"},
    "flowchart": {"useMaxWidth": True},
    "sequence": {"useMaxWidth": True},
}))


def modernise(src):
    """Repair label syntax that current mermaid no longer accepts.

    Mermaid stopped honouring a literal "\\n" as a line break in labels and now
    wants "<br/>", which every diagram in the corpus but one already uses. The
    stale one renders the escape as visible text, so it is converted here.
    """
    return src.replace('\\n', '<br/>')


def escape_semicolons(src):
    """Protect semicolons that sit inside label text.

    Mermaid reads a bare ";" as a statement separator, so a semicolon in a note
    or a message silently truncates the statement and the whole diagram fails to
    parse. The numeric entity renders as a semicolon, so the punctuation of the
    prose survives. Only the text after a statement's first colon is touched,
    which leaves structural separators (a trailing ";" on a graph line) alone.
    """
    out = []
    for line in src.split('\n'):
        head, sep, tail = line.partition(':')
        if sep and ';' in tail:
            line = head + sep + tail.replace(';', '#59;')
        out.append(line)
    return '\n'.join(out)


jobs = []
for n, rel in enumerate(paths, start=1):
    text = open(os.path.join(ROOT, rel), encoding='utf-8').read().split('\n')
    i = idx = 0
    while i < len(text):
        if text[i].strip() == '```mermaid':
            j = i + 1
            while j < len(text) and not text[j].strip().startswith('```'):
                j += 1
            idx += 1
            src = modernise(escape_semicolons('\n'.join(text[i + 1:j])))
            mmd = os.path.join(OUTDIR, 'ch%d-%d.mmd' % (n, idx))
            open(mmd, 'w', encoding='utf-8').write(src + '\n')
            jobs.append((rel, mmd, os.path.join(OUTDIR, 'ch%d-%d.pdf' % (n, idx))))
            i = j + 1
            continue
        i += 1

print('%d mermaid blocks in %d documents' % (len(jobs), len({j[0] for j in jobs})))
ok = fail = 0
for rel, mmd, pdf in jobs:
    if os.path.exists(pdf) and os.path.getmtime(pdf) >= os.path.getmtime(
            os.path.join(ROOT, rel)):
        ok += 1                                  # still current, keep it
        continue
    r = subprocess.run(['npx', 'mmdc', '-i', mmd, '-o', pdf, '--pdfFit',
                        '-p', cfg, '-c', mmdcfg, '-b', 'transparent'],
                       cwd=NPM_PREFIX, capture_output=True, text=True, env=env)
    if r.returncode == 0 and os.path.exists(pdf):
        ok += 1
    else:
        fail += 1
        print('FAILED %s (%s)\n%s' % (os.path.basename(pdf), rel, (r.stderr or r.stdout)[-500:]))
print('rendered %d, failed %d' % (ok, fail))
