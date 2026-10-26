# Noto Sans CJK

The font every language's dynamic text is drawn in, in the battle (the enhanced mode's text and lettering; the
original mode's strings where a pack has no font for the language are Fusion Pixel's, `../fusion-pixel`) and in
the app's body text, which the app registers from these files at its start (`lang::register_fonts`). The user (2026-10-25): "you should use one
unified font for all languages so it's not inconsistent", then chose Noto Sans CJK from a specimen of candidates,
and "just use noto sans cjk without subsetting it".

Noto Sans CJK is one design (Google's and Adobe's, the Source Han Sans glyphs) with each region's own forms of the
hanzi, shipped as one family per region. These are Google Fonts' variable files (github.com/google/fonts, ofl/
notosansjp, notosanssc, notosanstc; weights 100 to 900), whole:

- `NotoSansJP[wght].ttf`, family "Noto Sans JP": English and Japanese.
- `NotoSansSC[wght].ttf`, family "Noto Sans SC": Simplified Chinese (zh-Hans).
- `NotoSansTC[wght].ttf`, family "Noto Sans TC": Traditional Chinese (zh-Hant, Taiwan's forms).

License: SIL Open Font License 1.1 (OFL.txt).
