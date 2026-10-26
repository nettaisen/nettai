#!/usr/bin/env python3
"""subset.py [FULL]: the three cuts of Noto Sans CJK the frontend and the app draw text in, made from Google Fonts'
variable files (ofl/notosansjp, notosanssc, notosanstc of github.com/google/fonts: NotoSansJP[wght].ttf ...) in the
folder FULL, with fontTools' pyftsubset. Without FULL: how many characters each keeps.

The characters each Noto Sans CJK cut keeps: ASCII, Latin-1 and Latin Extended-A, common symbols and marks,
CJK punctuation and full-width forms, kana, the language's first level of hanzi (JIS X 0208 level 1, GB 2312 level 1,
Big5 level 1), and every character the content's tables and the app's catalogs (and its .slint sources) use.
"""

import glob, os, subprocess, sys, tomllib

# The repo's root (this file is crates/nettai-render/fonts/noto-sans-cjk/subset.py) and the full fonts' folder.
E = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..', '..'))
FULL = sys.argv[1] if len(sys.argv) > 1 else None
def toml_chars(p):
    out = set()
    def walk(t):
        for v in t.values():
            if isinstance(v, dict): walk(v)
            elif isinstance(v, str): out.update(v)
    walk(tomllib.load(open(p, 'rb'))); return out
def po_chars(d):
    out = set()
    for f in glob.glob(d + '/*.po'):
        for l in open(f, encoding='utf-8'):
            if l.startswith(('msgstr', '"')): out.update(l)
    return out
def codec(c, his, los):
    out = set()
    for hi in his:
        for lo in los:
            try: out.add(bytes([hi, lo]).decode(c))
            except Exception: pass
    return out
base = set(map(chr, range(0x20, 0x7f))) | set(map(chr, range(0xa0, 0x180))) | set(map(chr, range(0x2000, 0x2070))) \
    | set('←→↑↓⇧⏎⌫✓✕×÷±°′″№™©®ⒶⒷⓁⓇ○●◆◇□■△▲▽▼☆★♪…') | set(map(chr, range(0x3000, 0x3100))) | set(map(chr, range(0xff01, 0xff5f)))
used = {l: set() for l in ('en', 'ja', 'zh-Hans', 'zh-Hant')}
for g in ('exe4', 'exe5', 'exe6'):
    for l in used: used[l] |= toml_chars(f'{E}/content/{g}/locales/{l}.toml')
for l in ('ja', 'zh-Hans', 'zh-Hant'): used[l] |= po_chars(f'{E}/crates/nettai-ui/lang/{l}')
for f in glob.glob(f'{E}/crates/nettai-ui/ui/**/*.slint', recursive=True): used['en'] |= set(open(f, encoding='utf-8').read())
level1 = {
    'jp': codec('euc_jp', range(0xb0, 0xd0), range(0xa1, 0xff)),
    'sc': codec('gb2312', range(0xb0, 0xd8), range(0xa1, 0xff)),
    'tc': codec('big5', range(0xa4, 0xc7), list(range(0x40, 0x7f)) + list(range(0xa1, 0xff))),
}
langs = {'jp': ('en', 'ja'), 'sc': ('zh-Hans',), 'tc': ('zh-Hant',)}
for k in ('jp', 'sc', 'tc'):
    s = base | level1[k] | set().union(*(used[l] for l in langs[k]))
    s = {c for c in s if c >= ' ' and c != '\x7f' and not 0xe000 <= ord(c) <= 0xf8ff}
    if FULL is None:
        print(k, len(s))
        continue
    name = {'jp': 'JP', 'sc': 'SC', 'tc': 'TC'}[k]
    text = os.path.join(os.path.dirname(os.path.abspath(__file__)), f'{k}.txt')
    open(text, 'w').write(''.join(sorted(s)))
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), f'NotoSans{name}-Nettai[wght].ttf')
    subprocess.run(['pyftsubset', os.path.join(FULL, f'NotoSans{name}[wght].ttf'), f'--text-file={text}', f'--output-file={out}',
                    '--layout-features=*', '--no-hinting', '--desubroutinize', '--name-IDs=*', '--name-legacy',
                    '--name-languages=*'], check=True)
    os.remove(text)
    print(out, len(s), 'characters', os.path.getsize(out), 'bytes')
