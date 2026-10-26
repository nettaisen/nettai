# Noto Sans CJK, three cuts

The font every language's dynamic text is drawn in, in the battle (the enhanced mode's text and lettering, and the
original mode's strings where a pack has no font for the language: Chinese) and in the app's body text. The user
(2026-10-25): "you should use one unified font for all languages so it's not inconsistent", then chose Noto Sans
CJK from a specimen of candidates.

Noto Sans CJK is one design (Google's and Adobe's, the Source Han Sans glyphs) with each region's own forms of the
hanzi, shipped as one family per region. The cuts here are Google Fonts' variable files (weights 100 to 900):

- `NotoSansJP-Nettai[wght].ttf`, family "Noto Sans JP": English and Japanese.
- `NotoSansSC-Nettai[wght].ttf`, family "Noto Sans SC": Simplified Chinese (zh-Hans).
- `NotoSansTC-Nettai[wght].ttf`, family "Noto Sans TC": Traditional Chinese (zh-Hant, Taiwan's forms).

Each is subset (`subset.py`, which says what it keeps): Latin, symbols, CJK punctuation and full-width forms, kana,
the language's first level of hanzi (JIS X 0208's, GB 2312's, Big5's), and every character the content's strings
tables and the app's catalogs use, so a table or a catalog that gains a character needs `subset.py` run again
(nettai-render's tests say which character a cut lacks). Hints are dropped (the text is drawn from distance fields).

Licence: SIL Open Font License 1.1 (OFL.txt), whose Reserved Font Name is "Source"; these modified versions keep
the Noto names, which aren't reserved.
