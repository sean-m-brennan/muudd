# Single-PDF build

Renders the whole narrative chain, every document named in
[`doc/narrative-order.json`](../../doc/narrative-order.json), into one book.

```bash
tools/pdf/build.sh                 # writes doc/MUUDD.pdf
tools/pdf/build.sh /tmp/draft.pdf  # or somewhere else
```

The output path is ignored by the repository. Intermediates go to
`$WORK`, which defaults to `$TMPDIR/muudd-pdf` and is safe to delete; keeping it
means a rebuild neither reinstalls mermaid-cli nor re-renders a diagram whose
source has not changed.

## What it needs

`pandoc`, `xelatex` with `fontspec`, `fvextra`, `newunicodechar` and
`xurl`, `python3`, and `node` with `npm`. The fonts are TeX Gyre Pagella for the
prose and DejaVu Sans and DejaVu Sans Mono for everything Pagella does not
cover, which is the Greek, the Hebrew, and the box-drawing glyphs in the ASCII
diagrams.

`mermaid.py` needs a Chromium. It uses `PUPPETEER_EXECUTABLE_PATH` if that is
set, otherwise a browser Playwright has already cached under
`~/.cache/ms-playwright/`, otherwise it stops and says so. This matters where
puppeteer's own browser download is firewalled: a cached Playwright Chromium
works just as well, so check for one before concluding that diagrams cannot be
rendered.

## The pieces

| File | Does |
|---|---|
| `build.sh` | Runs the other three, then pandoc, then three xelatex passes, then reports diagnostics |
| `mermaid.py` | Extracts every fenced mermaid block and renders it to its own PDF |
| `assemble.py` | Concatenates the chain into one markdown file for pandoc |
| `preamble.tex` | LaTeX preamble, passed to pandoc with `--include-in-header` |

Three passes, because the table of contents and the cross-references between
documents each need the labels written by the pass before. The run ends with a
count of overfull boxes, missing characters, undefined references and multiply
defined labels; all four should stay where they are or fall.

## What `assemble.py` does to the sources

Nothing is edited in place. Per document, it

- strips the `Previous` and `Next` chain-navigation lines, which are a
  reading aid for the markdown and are meaningless in a linear book,
- rewrites a link that points at another document in the set into a
  cross-reference to that chapter, and flattens a link that points outside the
  set to its label text, since the PDF has nothing to point at (those targets
  are code and scenario files, and they do exist; they are just not documents),
- prefixes every footnote label with the document number, because the labels
  are unique per document and collide once the corpus is one file,
- replaces a fenced mermaid block with the rendered diagram,
- reverses the grapheme clusters of a Hebrew run and sets it in a font that
  covers it, there being no `bidi` here,
- and gives each document a chapter heading carrying its repository path, which
  is also what the running head shows.

## Two traps worth not falling into again

Both invent a character inside an identifier, and both are silent.

`hyphenat`'s `htt` option hyphenates typewriter text, so a file name breaks as
`proto-cols.md` and an environment variable as `AT_BOOTSTRAP_DIS-ABLED`, and a
reader has no way to tell the inserted hyphen from a real one. Separately,
LaTeX's own `\_` carries a discretionary that breaks *with* a hyphen, giving
`recognition_-protocols.md`.

So the preamble does neither. It takes the break opportunities from XeTeX
inter-character classes instead, which insert a penalty and no character: after
`/` and `_` anywhere, and after `.`, `:` and `(` only where the current family
is the typewriter one, so that ordinary prose keeps its punctuation. The guard
is worth a moment's care, since getting it wrong breaks prose at every abbreviated
word. `aaa.bbb.ccc.ddd` set narrow is the check: it stays whole in Pagella and
splits at each dot in the monospace font.

## Diagram sources are worth rendering before trusting

Two mermaid blocks in `lib/autonomous_trust` are wrong at the source, and both
fail the same way in any renderer, GitHub included. `mermaid.py` repairs them on
the way past, so a green build here is not evidence that the sources are fixed.

1. A semicolon inside a note is a statement separator, so the note truncates and
   the diagram fails to parse. It reaches `doc/architecture/reputation.md` from
   the description text in `reputation-canonical.yaml` by way of
   `tools/at_diagram.py`, which already flattens newlines out of note text for
   exactly this reason and does not yet handle the semicolon. The entity `#59;`
   renders as a semicolon and parses, so the prose need not change.
2. `doc/architecture/node-lifecycle.md`, which is hand-written rather than
   generated, uses a literal `\n` for a line break in a label. Mermaid wants
   `<br/>`, which every other diagram in the corpus already uses, so the escape
   showed up as visible text in the rendered diagram.
