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

"""Assemble the MUUDD narrative chain into one pandoc-ready markdown file.

Source of truth for the document set and its order is
lib/muudd/doc/narrative-order.json ("entry" plus "parts"/"documents").
"""
import json
import os
import re
import sys

if len(sys.argv) != 3:
    sys.exit('usage: assemble.py <muudd repository> <output.md>')
ROOT = os.path.realpath(sys.argv[1])
OUT = sys.argv[2]
MERMAID_DIR = os.environ.get('MERMAID_DIR', '')   # if set, .pdf renders live here

order = json.load(open(os.path.join(ROOT, 'doc/narrative-order.json')))

# ---------------------------------------------------------------- document set
docs = []
for part in order['parts']:
    for i, d in enumerate(part['documents']):
        docs.append({
            'part': part['title'] if i == 0 else None,
            'path': d['path'],
            'title': d['title'],
        })
docs.insert(0, {'part': None, 'path': order['entry'], 'title': 'MUUDD'})

by_real = {}
for n, d in enumerate(docs, start=1):
    d['n'] = n
    d['real'] = os.path.realpath(os.path.join(ROOT, d['path']))
    by_real[d['real']] = d

# ------------------------------------------------------------------- utilities
LATEX_ESC = {'\\': r'\textbackslash{}', '{': r'\{', '}': r'\}', '$': r'\$',
             '&': r'\&', '#': r'\#', '^': r'\^{}', '_': r'\_', '~': r'\~{}',
             '%': r'\%'}


def tex(s):
    return ''.join(LATEX_ESC.get(c, c) for c in s)


NAV = re.compile(r'^\*(?:Previous|Next):.*\*\s*$')
HEBREW = re.compile(r'[\u0590-\u05FF]+(?:[ \u0590-\u05FF]*[\u0590-\u05FF])?')
# link labels wrap across source lines throughout the corpus, so the pattern is
# applied to whole (code-free) text with DOTALL rather than line by line
LINK = re.compile(r'(!?)\[((?:[^\[\]]|\[[^\]]*\])*?)\]\(\s*<?([^)\s>]+)>?(\s+"[^"]*")?\s*\)', re.S)
SPLIT_TARGET = re.compile(r'\]\(([^)\s]+)\n\s*([^)\s]+)\)')
SENTINEL = '\x00%d\x00'
# footnote labels are per-document in the sources, so concatenating the corpus
# collides them; each label is prefixed with its document number
FOOTNOTE = re.compile(r'\[\^([^\]\s]+)\]')

stats = {'internal': 0, 'external': 0, 'dropped': 0, 'anchor': 0, 'image': 0,
         'nav_stripped': 0, 'mermaid_rendered': 0, 'mermaid_source': 0,
         'h1_added': 0, 'joined_targets': 0, 'hebrew': 0}
dropped_targets = []


def code_mask(lines):
    """Per-line flag: True when the line belongs to a code block (fenced or indented)."""
    mask = []
    fence = None
    prev_blank = True
    in_indent = False
    for ln in lines:
        stripped = ln.strip()
        m = re.match(r'^\s*(`{3,}|~{3,})', ln)
        if fence is None and m:
            fence = m.group(1)[0] * 3
            mask.append(True)
            prev_blank = False
            continue
        if fence is not None:
            mask.append(True)
            if stripped.startswith(fence):
                fence = None
            prev_blank = False
            continue
        if re.match(r'^(?: {4,}|\t)\S', ln) and (prev_blank or in_indent):
            in_indent = True
            mask.append(True)
            prev_blank = False
            continue
        if stripped == '' and in_indent:
            mask.append(True)               # blank line inside an indented block
            prev_blank = True
            continue
        in_indent = False
        mask.append(False)
        prev_blank = (stripped == '')
    return mask


def protect(lines, mask):
    """Swap code lines for sentinels so text transforms cannot reach into them."""
    kept, code = [], []
    for ln, is_code in zip(lines, mask):
        if is_code:
            code.append(ln)
            kept.append(SENTINEL % (len(code) - 1))
        else:
            kept.append(ln)
    return '\n'.join(kept), code


def restore(text, code):
    return re.sub('\x00(\\d+)\x00', lambda m: code[int(m.group(1))], text)


def rewrite_links(text, doc):
    base = os.path.dirname(doc['real'])

    def repl(m):
        bang, label, target, title = m.group(1), m.group(2), m.group(3), m.group(4) or ''
        if '\n\n' in label:                         # not a link: a label cannot
            return m.group(0)                       # span a blank line
        if bang:                                    # image
            if not target.startswith(('http://', 'https://')):
                stats['image'] += 1
                return '![%s](%s%s)' % (label, os.path.realpath(os.path.join(base, target)), title)
            return m.group(0)
        if target.startswith(('http://', 'https://', 'mailto:')):
            stats['external'] += 1
            return m.group(0)
        if target.startswith('#'):                  # same-document anchor
            stats['anchor'] += 1
            return label
        tgt = os.path.realpath(os.path.join(base, target.partition('#')[0]))
        hit = by_real.get(tgt)
        if hit:
            stats['internal'] += 1
            return '[%s](#ch:%d)' % (label, hit['n'])
        stats['dropped'] += 1
        dropped_targets.append(os.path.relpath(tgt, ROOT))
        return label

    return LINK.sub(repl, text)


def hebrew_rtl(text):
    """Set Hebrew runs right to left in a font that covers them.

    XeTeX here has no bidi package, so the grapheme clusters (each base letter
    with the points that follow it) are reversed in the source instead. Hebrew
    has no contextual joining, so the result on the page is the same.
    """
    def repl(m):
        stats['hebrew'] += 1
        clusters = []
        for ch in m.group(0):
            if clusters and 0x0591 <= ord(ch) <= 0x05C7:      # points and marks
                clusters[-1] += ch
            else:
                clusters.append(ch)
        return r'`\hebtext{%s}`{=latex}' % ''.join(reversed(clusters))
    return HEBREW.sub(repl, text)


def mermaid_blocks(body, doc):
    """Swap in a rendered diagram where one exists, otherwise label the source."""
    lines = body.split('\n')
    out, i, idx = [], 0, 0
    while i < len(lines):
        if lines[i].strip() == '```mermaid':
            j = i + 1
            while j < len(lines) and not lines[j].strip().startswith('```'):
                j += 1
            idx += 1
            art = os.path.join(MERMAID_DIR, 'ch%d-%d.pdf' % (doc['n'], idx)) if MERMAID_DIR else ''
            if art and os.path.exists(art):
                stats['mermaid_rendered'] += 1
                out += ['', '![](%s)' % art, '']
                i = j + 1
                continue
            stats['mermaid_source'] += 1
            out += ['*Diagram, Mermaid source:*', '', '```']
            i += 1
            continue
        out.append(lines[i])
        i += 1
    return '\n'.join(out)


def prepare(doc):
    raw = open(doc['real'], encoding='utf-8').read().replace('\r\n', '\n')
    # ASCII-art legend glyph with no coverage in DejaVu Sans Mono
    raw = raw.replace('\U0001f50c', '\u25c6')      # electric plug -> black diamond
    lines = raw.split('\n')

    # strip the reading-chain navigation lines
    kept = [ln for ln, is_code in zip(lines, code_mask(lines))
            if is_code or not NAV.match(ln)]
    stats['nav_stripped'] += len(lines) - len(kept)
    while kept and kept[-1].strip() == '':
        kept.pop()
    if kept and re.match(r'^\s*(-{3,}|\*{3,}|_{3,})\s*$', kept[-1]):
        kept.pop()                                  # rule left behind by the Next line

    text, code = protect(kept, code_mask(kept))
    text, n = SPLIT_TARGET.subn(lambda m: '](%s%s)' % (m.group(1), m.group(2)), text)
    stats['joined_targets'] += n
    text = FOOTNOTE.sub(lambda m: '[^d%d-%s]' % (doc['n'], m.group(1)), text)
    text = hebrew_rtl(rewrite_links(text, doc))
    body = restore(text, code).strip('\n')
    body = mermaid_blocks(body, doc)

    # each document opens with a level-1 heading (which becomes the chapter) and
    # carries its repository path immediately underneath
    out_lines = body.split('\n')
    mask = code_mask(out_lines)
    h1_at = next((i for i, (ln, c) in enumerate(zip(out_lines, mask))
                  if not c and re.match(r'^# \S', ln)), None)
    if doc['n'] == 1 and h1_at is not None:
        # the entry document is unnumbered, so that chapter numbers match the
        # document numbers in narrative-order.json
        title = tex(out_lines[h1_at][2:].strip())
        out_lines[h1_at:h1_at + 1] = [
            '```{=latex}', '\\chapter*{%s}' % title, '\\phantomsection',
            '\\addcontentsline{toc}{chapter}{%s}' % title, '```']
        h1_at += 4
    # the label has to sit after the chapter heading so that \hyperref
    # lands on the chapter's own anchor, not the preceding one
    pathline = ['', '```{=latex}', '\\label{ch:%d}' % doc['n'],
                '\\srcpathline{%s}' % tex(doc['path']), '```', '']
    if h1_at is None:
        stats['h1_added'] += 1
        out_lines = ['# ' + doc['title']] + pathline + out_lines
    else:
        out_lines = out_lines[:h1_at + 1] + pathline + out_lines[h1_at + 1:]
    return '\n'.join(out_lines)


# -------------------------------------------------------------------- assemble
chunks = []
for doc in docs:
    if doc['part']:
        chunks.append('```{=latex}\n\\clearpage\n\\part*{%s}\n'
                      '\\addcontentsline{toc}{part}{%s}\n\\markboth{}{}\n```\n'
                      % (tex(doc['part']), tex(doc['part'])))
    chunks.append('```{=latex}\n\\clearpage\n\\markboth{%s}{%s}\n```\n'
                  % (tex(doc['path']), tex(doc['path'])))
    chunks.append(prepare(doc))

with open(OUT, 'w', encoding='utf-8') as fh:
    fh.write('\n'.join(chunks) + '\n')

print('documents: %d' % len(docs))
for k in sorted(stats):
    print('  %-18s %d' % (k, stats[k]))
uniq = sorted(set(dropped_targets))
print('link targets outside the document set: %d unique / %d links '
      '(rendered as plain text)' % (len(uniq), stats['dropped']))
