# Nettai Digits

The Battle Network games' battle-screen digits as an outline font, for the enhanced mode's numbers. One design: the
HP box's digits (6 by 10 pixels), the HUD's '?', '+' and the times sign of "x2" (`×` only; no `x`), traced from
their pixels with sharp corners. A glyph pixel is 100 units (1000 to the em, the digits 1000 tall), each glyph in the
game's 8-pixel cell (advance 800, the face from 100), so every digit has one advance.

The enemies' HP digits and the chip window's are this design at other heights, so the renderer fits this one font to
their boxes. The banners' digits are another drawing: Nettai Banner Digits (../nettai-banner-digits).

Built by bn6battle-verify's `tools/digits/build_otf.py` from EXE6's pack (EXE5's digits are the same); its README has
the tracing rules and the user's calls on them.

Part of nettai, under the Mozilla Public License 2.0 (the repository's); the digits' design is the games' own.
