# Nettai Digits

The Battle Network games' battle-screen digits as an outline font, for the enhanced mode's numbers. One design: the
HP box's digits (6 by 10 pixels), the HUD's '?', '+' and the "x" of "x2" (mapped to both `x` and `×`), traced from
their pixels with sharp corners. A glyph pixel is 100 units (1000 to the em, the digits 1000 tall), each glyph in the
game's 8-pixel cell (advance 800, the face from 100), so every digit has one advance.

The enemies' HP digits and the chip window's are this design at other heights, so the renderer fits this one font to
their boxes. The banners' digits are another drawing: Nettai Banner Digits (../nettai-banner-digits).

`NettaiDigits[wght].otf` is a variable font (CFF2 outlines, fvar, HVAR) with a weight axis, `wght` 400 to 700 (400 the
default, the traced outlines): toward 700 every horizontal edge inside a glyph's ink box moves into its counter, up to
half a pixel, so a top or bottom bar is 2 + (w - 400) / 600 pixels thick (a middle bar gains twice that); the glyph
keeps its height, width and advance, and the stems their width. The renderer sets the weight for each place from how
far its box squeezes the design, so the bars stay the game's 2 pixels (the enemies' HP, 8 tall: 700; the user: "the
numbering font looks weird when used as-is for enemy hp. you should implement opentype variable weight and thicken the
font appropriately to match the hp").

Built by bn6battle-verify's `tools/digits/build_digits_vf.py` (the outlines `tools/digits/build_otf.py` traces) from
EXE6's pack (EXE5's digits are the same); its README has the tracing rules and the user's calls on them.

Part of nettai, under the Mozilla Public License 2.0 (the repository's); the digits' design is the games' own.
