# Fusion Pixel Bold 12px

The original mode's 8x16 font's strings (chip names, enemy names, the telop, the Program Advance's names) where a
pack has no font for the language shown (Chinese), drawn at its own size into the 240x160 frame (`pixeltext`); the
chatbox beside it is in Fusion Pixel (`../fusion-pixel`).
The user (2026-10-25), after specimens of thicker fonts: "Fusion Pixel Bold 12px is perfect".

Fusion Bold Pixel Font (缝合粗像素字体), Fusion Pixel's bold build, by TakWolf: github.com/pixel-font-studio/
fusion-bold-pixel-font, release 2026.10.25, `fusion-bold-pixel-font-12px-monospaced-otf`. Its glyphs lie on a
12-pixel grid (1200 units an em, 100 a pixel). Whole files, one per Chinese's forms of the hanzi (the release's
Japanese cut isn't bundled: every pack spells English and Japanese in its own fonts):

- `fusion-bold-pixel-12px-monospaced-zh_hans.otf`: Simplified Chinese (zh-Hans).
- `fusion-bold-pixel-12px-monospaced-zh_tw.otf`: Traditional Chinese (zh-Hant, Taiwan's forms).

The program holds both builds' files packed, all four in one zstd frame, and unpacks them the first time the
original mode draws Chinese (`bundled`, the crate's build.rs).

License: SIL Open Font License 1.1 (OFL.txt). The files are the release's, unchanged. Its author made Fusion
Pixel as the stand-in for Ark Pixel while Ark (hand-drawn) still lacks many characters: Ark Pixel's glyphs, the rest
taken from two other open pixel fonts, Cubic 11 and Galmuri ("stitched", 缝合, in its name). Their authors'
copyright notices come with it in its release (LICENSES), as the OFL asks; nothing else is bundled here.
