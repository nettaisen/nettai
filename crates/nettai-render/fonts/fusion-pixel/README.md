# Fusion Pixel 12px

The original mode's chatbox text where a pack has no font for the language shown (Chinese), drawn at its own size
into the 240x160 frame (`pixeltext`). The 8x16 font's strings beside it (chip names, enemy names, the telop) are in
MuzaiPixel (`../muzai-pixel`). The user (2026-10-25), from a specimen of candidates drawn into battle frames: "Fusion
Pixel 12px looks good for the dialog box font".

Fusion Pixel Font (缝合像素字体), by TakWolf: github.com/TakWolf/fusion-pixel-font, release 2026.10.25,
`fusion-pixel-font-12px-monospaced-otf`. Its glyphs lie on a 12-pixel grid (1200 units an em, 100 a pixel), so
drawn at 12 pixels an em each pixel of a glyph is a pixel of the frame. Whole files, one per Chinese's forms of the
hanzi (the release's Japanese cut isn't bundled: every pack spells English and Japanese in its own fonts):

- `fusion-pixel-12px-monospaced-zh_hans.woff2`: Simplified Chinese (zh-Hans).
- `fusion-pixel-12px-monospaced-zh_tw.woff2`: Traditional Chinese (zh-Hant, Taiwan's forms).

Each is the release's .otf converted to WOFF2 as it is (`fonttools ttLib.woff2 compress`, which unpacks to the same
tables and glyphs). The program unpacks one the first time the original mode draws Chinese in it (`bundled`).

License: SIL Open Font License 1.1 (OFL.txt). The fonts are the release's, unchanged. Its author made Fusion
Pixel as the stand-in for Ark Pixel while Ark (hand-drawn) still lacks many characters: Ark Pixel's glyphs, the rest
taken from two other open pixel fonts, Cubic 11 and Galmuri ("stitched", 缝合, in its name). Their authors'
copyright notices come with it in its release (LICENSES), as the OFL asks; nothing else is bundled here.
