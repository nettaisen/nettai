# MuzaiPixel

The original mode's 8x16 font's strings (chip names, enemy names, the telop, the Program Advance's names) where a pack
has no font for the language shown (Chinese), drawn at its own size into the 240x160 frame (`pixeltext`): its hanzi
are 7 pixels wide and 11 tall in an 8-pixel advance, one of the game's cells each, as its letters are; the chatbox
beside it is in Fusion Pixel (`../fusion-pixel`). The user (2026-10-30) picked it from a specimen of narrow pixel fonts
drawn into the chip window, and kept it as drawn: "just use the original font".

MuzaiPixel (目哉像素), by DWNfonts: github.com/diaowinner/MuzaiPixel, release 2.0.20260705 (2.0.20241013 repacked),
`MZPXorig.ttf` (the build for square pixels; `MZPXflat` is its full-width one), based on Num Kadoma's k8x12. Its
glyphs lie on a 12-pixel grid (1200 units an em, 100 a pixel). One file has both forms of the hanzi.

`MZPXorig.woff2` is the release's .ttf converted to WOFF2 as it is (`fonttools ttLib.woff2 compress`; unpacked, its
15,936 glyphs, metrics and character map are the original's). The program unpacks it the first time the original mode
draws Chinese in it (`bundled`).

License: SIL Open Font License 1.1, with the Reserved Font Name MuzaiPixel (OFL.txt, the release's LICENSE.txt). The
font is the release's, unchanged.
