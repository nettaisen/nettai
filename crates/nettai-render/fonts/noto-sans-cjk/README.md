# Noto Sans CJK

The font every language's dynamic text is drawn in, in the battle (the enhanced mode's text and lettering; the
original mode's strings where a pack has no font for the language are Fusion Pixel's, `../fusion-pixel`) and in
the app's body text, which the app registers from these files at its start (`lang::register_fonts`). The repository
holds them as WOFF2, which the program unpacks once (`bundled`). The user (2026-10-25): "you should use one
unified font for all languages so it's not inconsistent", then chose Noto Sans CJK from a specimen of candidates,
and "just use noto sans cjk without subsetting it".

Noto Sans CJK is one design (Google's and Adobe's, the Source Han Sans glyphs) with each region's own forms of the
hanzi, shipped as one family per region. These are Google Fonts' variable files (github.com/google/fonts, ofl/
notosansjp, notosanssc, notosanstc; weights 100 to 900), whole, each converted to WOFF2 as it is
(`fonttools ttLib.woff2 compress`, which unpacks to the same tables and glyphs):

- `NotoSansJP[wght].woff2`, family "Noto Sans JP": English and Japanese.
- `NotoSansSC[wght].woff2`, family "Noto Sans SC": Simplified Chinese (zh-Hans).
- `NotoSansTC[wght].woff2`, family "Noto Sans TC": Traditional Chinese (zh-Hant, Taiwan's forms).

License: SIL Open Font License 1.1 (OFL.txt).
