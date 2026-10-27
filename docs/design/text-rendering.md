# Text rendering: the extracted pixel fonts, and font files

**Status (2026-10-02): built.** The user chose crisp text at the window's resolution (option B of §2.1, which §6
had rejected as the default) with a bundled open-license font, as the default; §9 "As built" says what was built
and how, and records the decisions; §9.8 (2026-10-13) is the fix of its sizes and of Japanese off its line: one size
a role, a string narrowed to its box, kana and kanji on the capitals; §9.10 (2026-10-14) draws the glyphs from
signed distance fields made once for every size. §10 is the languages built on it
(`--lang en|ja`) and where display text lives (no definition holds any: `locales/<lang>.toml`). §11 (2026-10-14) is a
survey of the battle interface's pictures with lettering, and which of them the enhanced text could draw; §12
(2026-10-19) is what was built of it: the banners (with their bar), the labels and the help pictures in a
game-style font, the numbers in another. §0 to §8 are the investigation as written before it, unchanged.

The question was: should the frontend draw its text (chip names, descriptions, telops and the rest) with a real
font file rasterized at run time, instead of the pixel fonts extracted from the user's ROM? This document says what
the frontend drew and how, what "real font rendering" can mean here, what it would cost, how it sits beside the
pixel-for-pixel comparison with the original, and what I recommended.

The measurements in §8 were throwaway spikes outside the repository.

Related: [frontend.md](../frontend.md) (what is drawn and how it is verified), [asset-formats.md](asset-formats.md)
§4 (the HUD's files), [content-model-v2.md](content-model-v2.md) §3.1 and §6 (names and descriptions in the
definitions, `compat/text.toml`), [rollback.md](rollback.md) §3 (presentation under rollback),
[../engine/custom-screen.md](../engine/custom-screen.md) §3.5 (the chatbox).

## 0. Summary

- **EXE6 draws battle text with two fonts, and a lot of what looks like text is pictures.** The 8x16 font (224
  glyphs, fixed 8-pixel advance, a face color and a shadow color) draws chip names, telops, the other player's
  used chip and the HUD's text lines. The dialogue font (16x12 cells, proportional advance from a width table,
  about 460 glyphs with kana and kanji) draws the chatbox: chip descriptions and the no-running message. Banners,
  every number but the turn countdown (HP, damage, the count box, round numbers), "PAUSE" and "Cstmzing..." are
  tile pictures.
- **Step 0 is done** (the custom screen's milestone 4): the dialogue font is in the pack (HUD format 6, with its
  advances and its charmap, the two-byte glyphs included), the run message's words are in the navi definitions
  (`run_message = { counts, text, portrait }`), the Crosses have their descriptions, and the chatbox is drawn.
  Every string the frontend draws goes through `fonts.rs`: `cell_glyphs`, `cell_text`, `draw_cell_text` and
  `cell_glyph` for the 8x16 font (the HUD's sites too), `dialogue_glyphs` and `dialogue_text` for the dialogue
  font. (The paragraphs below describe the state before it.)
- **The 8x16 font spells every EXE6 chip name** (298 of 298, all within 8 glyphs). It has Latin letters, digits,
  kana, two kanji and some signs: no accents, no kanji to speak of, nothing else. A character it lacks is dropped
  from the name and reported by `--audit`; glyphs past the eighth are cut silently.
- **A font file rasterized into the 240x160 frame only looks right if it is a pixel font at its native size.**
  Measured: pixel fonts shipped as outlines come out with coverage exactly 0 or 255 and whole-pixel advances, the
  same from two rasterizers. A general UI font at 10 to 12 pixels is mush, and kanji are unreadable. A UI font is
  crisp only on a separate layer at output resolution, which is no longer GBA pixels and needs renderer work.
- **Recommendation: one text layer in the frontend with pluggable fonts; the extracted fonts stay the reference.**
  - Text goes through one layout and drawing path that takes "a font" (a glyph bitmap source): the pack's
    extracted fonts, or a font file (BDF, or TTF/OTF pixel fonts rasterized once at their native size).
  - Text is drawn into the 240x160 frame through the existing layers and sprite parts, so priority, fades, the
    banners' squash and the sprite limit work as they do now.
  - Three modes: `original` (extracted fonts only: what the comparison uses), `font` (font files only: what a pack
    without a ROM uses), and `auto`, the default: the extracted font wherever it can spell the whole string in its
    box, a bundled open-license pixel font for the strings it can't.
  - `auto` changes no pixel of EXE6's own content, so the default stays comparable with the original, and mods,
    translations and Japanese text with kanji stop losing characters.
  - Numbers and the picture labels stay pictures in every mode. Banners are out of scope.
- **Rejected as the main line: a vector font on an output-resolution layer** (cosmic-text or similar). It is the
  only way to get shaping for Arabic or Indic scripts and crisp text at any window size, but it breaks the look,
  needs a depth-aware text layer above the scaled frame, and brings about forty crates. It stays possible later
  on top of the same text layer (§6.3, step 7).
- **Two content findings that matter whichever way this goes.**
  - A description's line count and the run message's characters per line are simulation inputs (the chatbox's
    timing). Text layout must never feed back into them.
  - Names and descriptions are part of the content hash. A translation that edited them would make two peers'
    content differ. Display strings for other languages have to live outside the hashed content (§4.3).
- **The work**: about 1,300 to 1,600 lines in the frontend and the content crates for steps 1 to 5 of §6.3, one
  small dependency (`fontdue`), and one or two committed font files with their licenses. It should start after
  the custom screen is drawn (§6.5).

## 1. Today's state

### 1.1 The fonts the original has

| Font | Where (original labels) | Cell and advance | Colors | Glyphs | In the pack |
|---|---|---|---|---|---|
| The 8x16 font | `dword_86B7AE0`, drawn into tiles by `renderTextGfx_8045F8C` and as sprite glyphs by the HUD tasks | 8x16 cell, one glyph a cell, advance always 8; the ink sits almost wholly in rows 3 to 14 and is 7 pixels wide plus the shadow | 2: face (index 1) and a shadow to the right and below (index 2) | 0xE0 (224) | yes: `graphics/hud/font.png` and `font_chars` in `hud.json` |
| The dialogue font | `byte_86ACD60`, drawn by `sub_3006F8C` (the chatbox) and `sub_80461D0` | 16x12 cell; advance from a width table (`byte_8043CA4`; two more tables for other screens): capitals and digits 8, `I` 6, lowercase 6 to 8, the space 8, kana and kanji 11 | a face color (index 1) and a softer one on some pixels (index 3); the chatbox adds a color offset to pick the text color | about 460: the first 0xE4 by one byte, the rest by a two-byte code (`E4 nn`); the kana and about two hundred kanji are still in the US ROM | no (nothing draws the chatbox yet) |
| The banner letters | per banner, 20 pointers to 8x16 letter pictures that include the bar behind them (`pt_801EF84`) | 8x16, 20 cells a banner | the banner palette | (pictures) | as one picture a banner: `graphics/hud/banners/<name>.png` |

The text encoding is one table, `compat/text.toml` (what each byte below 0xE0 draws, as UTF-8). Glyphs with no
character of their own had bracketed names: `[EX]`, `[SP]`, `[RV]`, `[BX]`, `[FZ]`, the button marks `[A]`, `[B]`,
`[L]`, `[R]`, `[bat]`, `[z]` (since 2026-10-02 each is a character: §10.5). The extractor writes the same table into
`hud.json` as `font_chars`.

### 1.2 Every place text is drawn, or would be

"Cells" below are the 8-pixel columns and rows of the HUD's tile layer. "Drawn" says whether the frontend draws
it today.

| What | The string comes from | How it becomes pixels | The original's layout | Layer and colors | Drawn |
|---|---|---|---|---|---|
| The next chip's name in the hand window | the chip definition's `name` (UTF-8), through the hand's chip handle | `Hud::glyphs` spells it in the 8x16 font, longest glyph name first; at most 8 glyphs | from cell column 0 on rows 18 and 19 (y 144); the damage digits follow the last glyph at once, then `+` and the bonus, then `x2` | the HUD tile layer (priority 1), in the HP box's palette, so it turns with the HP box's color state (normal, healing, hurt or low) | yes |
| A telop | the chip the engine's `Banner::telop` names (`Battle::telop_for` gives the viewer's side of it), then the definition's `name` | the 8x16 font as 8x16 sprite parts | name, damage, `+bonus` centered as 15 glyphs are in the viewer's half of the screen (`x = place + (15 - n) * 4`; places (0, 32) and (120, 32)); `x2` after; the other player's moves 16 pixels left for an `x2`. No scroll. The banners' vertical squash: it grows over 5 ticks, holds 0x30, shrinks over 5 (`sub_801CE28`) | sprites of priority 0, front layer, bucket 6; the HP box's normal palette | yes |
| A hidden telop, "????" | a literal in the frontend (the original shows the name of chip 0x171) | four `?` of the 8x16 font | as a telop | as a telop | yes |
| The chip the other player just used | `Battle::used_chips`, then the definition's `name` | the 8x16 font as sprite parts | laid out as a telop at the right-hand place, without the squash, for 60 ticks | as a telop | yes |
| "????" beside the mugshot and at the right edge (a defensive chip is set) | nothing: it is a picture | HUD layer tiles 0x1CC and 0x1CD | 4x2 cells at column 6 or 26, rows 2 and 3 | HUD tile layer | yes |
| The turn countdown and "TIME UP!" | the pack's `hud.json` `texts` (lines 3 to 13 of the ROM's HUD text script, as glyph numbers; the seconds are lines like `"    5   "`) | the 8x16 font | 8 glyphs from column 11, rows 0 and 1; the padding spaces in the line center it | HUD tile layer, HP box palette | yes |
| The HUD message line, "COUNTER HIT!" | `Battle::message` picks the line; the text is the pack's `texts` line 14 | the 8x16 font | 17 glyphs from column 7, rows 2 and 3; padded with spaces | HUD tile layer, HP box palette | yes |
| The other HUD lines: "DOUBLE DELETE!", "TRIPLE", "QUADRUPLE", "PENALTY!!", "SHUFFLE!", "LEFT 0" | the pack's `texts` | the 8x16 font | as the message line | HUD tile layer | no (virus battles and the custom screen's) |
| "VS" between the judge's numbers | the frontend's literal (the pack's `texts` line 2 holds it too) | the 8x16 font | columns 14 and 15, rows 5 and 6 | HUD tile layer | yes |
| The chip's name on the custom screen | the definition's `name` | the 8x16 font through the fixed-cell renderer (`sub_8027D10` picks the text, `renderTextGfx_8045F8C` draws 8 cells) | 8 cells in the chip window at the top left | a tile layer of the custom screen | no |
| A chip's or a Cross's description (R on the custom screen) | the definition's `description`, its lines apart by `\n` | the dialogue font, composed into a 192-pixel line buffer and shown as six 32x16 sprites a line | up to three lines, 16 pixels apart, broken only where the text has a line break (`E9`); a whole line appears a tick (print speed 0) | sprites over the chatbox's box | no |
| The no-running message (L in a netbattle) | nowhere yet: the navi definition's `run_message` holds only the characters in each line (`{ 19, 12 }`), which is what the timing needs | the dialogue font | as a description, beside the operator's portrait; a character every other tick | as a description | no |
| Navi names | the navi definition's `name` exists, and nothing draws it as text in a netbattle: the names players see are in the banners | (pictures) | | | (as banners) |
| The frontend's own status line, the stop reason, the text custom screen of live play | the frontend | `text.rs`, a built-in 3x5 font | drawn over the finished frame in the window only | over everything | yes |

### 1.3 Pictures that look like text

These are tiles or sprites with their letters or digits painted in. No string is involved, and none of them goes
through a font.

- **Banners**: battle start, turn start with its number, the win, lose and deleted banners of each navi
  ("MEGAMAN DELETED"), draw, program advance and the rest: 45 pictures of 20 glyphs each, with the squash (the
  other two of the 47 banner entries are the telops' places). The round and turn numbers are the banner digits
  (`banner-digits.png`).
- **Numbers**: the HP box's digits and the damage digits with `+` and `x2` (HUD layer tiles), the opponent's HP
  digits under its navi (sprites in three colors), the judge's two numbers, the count box beside the mugshot
  (0 to 10, one picture each), the custom screen's damage digits and chip code letters.
- **Labels**: "PAUSE" (five sprite glyphs), "Cstmzing..." (8x2 tiles), the full gauge's "L or R", and the custom
  screen's fixed labels.

### 1.4 What the 8x16 font covers

- Space, `0`-`9`, `A`-`Z`, `a`-`z`; `* - × = : % ? + ! & , . ・ ; ' " ~ / ( ) ｢ ｣ _ █ ー ゜`; katakana with the
  voiced and small forms; 40 hiragana (no voiced forms, and no う, え, お, わ, を or ん: the table ends where the
  control codes begin); the kanji 熱 and 斗; and the eleven bracketed marks.
- No accented letters, no kanji beyond those two, no Cyrillic, Greek, Hangul or anything else, and no `[`, `]`,
  `#`, `@`, `<`, `>`.
- Every EXE6 chip name is spelled within 8 glyphs: 210 names take all eight, 57 take seven, the rest fewer
  (`ElecMan[EX]` is eight: `[EX]` is one glyph). The 295 descriptions have at most 11 characters a line as
  written and use, beyond letters and digits, `! & ' + , - . / : =`, the mark `[A]` and a few katakana.

What happens to a name it can't spell (`hud::name_glyphs`, `Hud::glyphs`):

- a character with no glyph is **dropped**, the rest of the name closes up, and the problem is recorded
  (`Problems`), which `--audit` prints and exits 1 on;
- glyphs after the eighth are **cut**, with no report;
- a name that is all missing characters draws nothing.

So a mod's "Épée", a long name, or a Japanese name with kanji draws wrong today, and only the audit says so.

### 1.5 Where the strings live

- **In the content definitions (UTF-8, committed, part of the content hash)**: a chip's `name` and
  `description`, a navi's `name`.
- **In the pack, as glyph numbers**: the HUD's text lines (`hud.json` `texts`). These are game text rather than
  media, and under content model v2 they belong with the content (§4.2).
- **In the frontend, as literals**: "????" and "VS".
- **Nowhere**: the run message's words.
- **In the engine's state**: no strings at all. The presentation outputs name things by handle (`Telop::chip`,
  `UsedChip::chip`) or by enum (`hud::Message`), and the frontend looks the words up.

## 2. What "actual font rendering" would mean, and the options

Three choices are independent, and it helps to keep them apart:

1. **Where glyph pictures come from**: the pack's extracted fonts, or a font file.
2. **What resolution text is drawn at**: the 240x160 frame, or the output window.
3. **Which strings change**: all text, or only what the extracted font can't draw.

### 2.1 Resolution

**A. Into the 240x160 frame.** A string becomes a small indexed bitmap (face and shadow as palette indices 1 and
2), which is drawn as tiles on the HUD layer or as sprite parts, exactly where the cell glyphs go today.

- What it needs from the renderer: nothing new. `Layer::draw_tile` and `SpritePart` already take any tiles.
- Interleaving comes for free, because the text is in the same composition as everything else:
  - priority: HUD-layer text is under priority-0 sprites (banners, the mugshot), telops are sprites of priority 0;
  - a dimming darkens only the stage's palettes, so text keeps its colors, as it does now;
  - the transformation's fade and the screen fades reach text through `Fades`, as now;
  - the telop's squash is `SpritePart::vscale` on the text's parts;
  - camera shake does not move HUD text (the HUD layer and the HUD's screen-placed sprites don't shake), and
    nothing mirrors it on the right-hand player's console: only object-anchored HUD pieces go through
    `project_hud`, and those are digit pictures and chip icons;
  - the sprite limit: a telop's glyphs count toward the 128 parts. A font-drawn telop should count the parts the
    original's glyphs would have, so the same objects are culled in every mode.
- The limit: sizes are tiny (cap height about 7 to 11 pixels), so only **pixel fonts at their native size** look
  right. Measured in §8: general UI fonts at 10 to 12 pixels are blurry with antialiasing and ragged without
  (the Rust rasterizers don't hint), and kanji from a UI font are unreadable at that size.

**B. On a separate layer at output resolution**, above the scaled frame.

- Crisp at any window size, with any font, any script, and room for longer names by condensing.
- No longer GBA pixels: smooth text beside pixel-art digits, icons and banners.
- What it needs from the renderer:
  - `Renderer::render` returns the frame **and a list of text items** (string, style, box in frame coordinates,
    depth key, fade, vertical scale) instead of drawing text;
  - a per-pixel depth mask from `compose` (the priority and rank of what won each frame pixel), so an item is
    hidden where something in front of it was drawn. Without it, HUD-layer text shows through the mugshot and
    banners that cover it;
  - the fades applied to text colors by the text layer (the screen fade, and the HUD fade for HUD-layer items);
    the telop's squash as a transform; sprites blended over text approximated;
  - the window (`app.rs`) and the PNG writer (`headless.rs`) compositing the items after scaling, and a real
    scaling policy if the window stops being an integer multiple;
  - the sprite limit handled as in A.
- A cheaper variant, text into the frame at 2x or 4x of a supersampled frame, has the same costs and still isn't
  pixels.

For a pixel font the two give the same picture at integer scales (§8), so B buys nothing unless the font is a
vector font.

### 2.2 Libraries

The versions are the ones examined; licenses and dependencies are read from their manifests. Newer versions may
exist.

| Crate | What it does | License | Dependencies | Notes |
|---|---|---|---|---|
| `fontdue` 0.9 | parses TTF/OTF and rasterizes outlines; a simple layout helper | MIT OR Apache-2.0 OR Zlib | `ttf-parser`, `hashbrown` | No shaping (its README points to cosmic-text for that), no hinting, no embedded bitmap strikes. One maintainer. The smallest way to turn a font file into glyph bitmaps |
| `ab_glyph` 0.2 | parses (through `owned_ttf_parser`) and rasterizes outlines; exposes embedded bitmap strikes (`EBDT`: mono, 2-, 4-, 8-bit) as raw data | Apache-2.0 | `ab_glyph_rasterizer`, `owned_ttf_parser` → `ttf-parser` | No shaping, no hinting. Reads the bitmap strikes that `.otb` and some pixel TTFs carry |
| `swash` 0.2 | scaling with hinting, rasterizing (`zeno`), bitmap strikes, color glyphs | Apache-2.0 OR MIT | `skrifa` → `read-fonts`; optional `zeno`, `yazi` | The rasterizer cosmic-text uses. Hinting is what makes small vector text tolerable |
| `harfrust` 0.3 | text shaping: the HarfBuzz port | MIT | `read-fonts`, `bitflags`, `bytemuck`, `smallvec`, `core_maths` | **It replaces `rustybuzz`**, whose repository was archived in July 2026 with a note to move to HarfRust |
| `cosmic-text` 0.15 | the whole stack: font database and fallback, shaping (HarfRust), bidi, line breaking, layout, rasterizing (swash) | MIT OR Apache-2.0 | 14 direct (`fontdb`, `harfrust`, `skrifa`, `swash`, `unicode-bidi`, `unicode-linebreak`, `unicode-script`, `unicode-segmentation`, `rangemap`, `self_cell`, `smol_str`, ...); about 40 packages in its tree without system-font discovery, some 30 of them new to this workspace | Its default features discover system fonts (`fontconfig`); turned off, it uses only the font data it is given. The right tool for option B and for scripts that need shaping |
| `bdf-parser` 0.1 | parses BDF bitmap fonts | MIT OR Apache-2.0 | `nom`, `bstr`, `strum`, `thiserror` | BDF is a small line-oriented text format; a reader of about 200 lines needs no crate |
| `unicode-linebreak`, `unicode-bidi` | the Unicode line breaking and bidirectional algorithms | Apache-2.0; MIT OR Apache-2.0 | none | Only needed for automatic wrapping and right-to-left text |

`parley` (the Linebender layout crate on the same `skrifa` and `harfrust` base) is an alternative to cosmic-text
that I did not examine.

**Determinism across platforms.** It only matters for the frontend's own golden tests, because text never
reaches the simulation (§5).

- Bitmap fonts (the pack's, BDF, embedded strikes): exact by construction, no arithmetic.
- Pixel fonts shipped as outlines, at their native size: every pixel's coverage is 0 or 1 by geometry, so a
  threshold at one half gives the same bitmap on any platform and from any rasterizer. `fontdue` and `ab_glyph`
  produced identical bitmaps and whole-pixel advances for the three pixel fonts tried (§8).
- Vector fonts at arbitrary sizes: the rasterizers use `f32`, some with SIMD paths chosen per platform, and they
  don't promise bit-identical output. The two rasterizers already differ from each other on the same glyph. Tests
  of such text would need a tolerance.

### 2.3 Which font

The repository can hold fonts under open licenses (OFL, Apache, public domain or as permissive). It cannot hold
a font traced from the ROM's glyphs: that is ROM-derived media.

| Source | For | Against |
|---|---|---|
| **A bundled open-license pixel font** | text always draws, the same on every machine; fits the frame; can be chosen to sit well beside EXE6's art | no open font is EXE6's; it needs choosing, and a CJK one is large (thousands of glyphs; not measured here) |
| **A bundled general UI font** (Noto Sans, Inter, M PLUS) | wide coverage, familiar | only usable with option B |
| **The user's system fonts** | nothing to bundle; covers the user's language | differs on every machine, so screenshots and tests don't reproduce; needs font discovery (`fontdb`, fontconfig on Linux); license unknown. Reasonable only as an explicit `--font <path>` |
| **A font the content or the pack names** | a mod, a translation or a from-scratch pack brings its own look and coverage; EXE6's pack is the case where the font happens to be extracted | the frontend still needs a fallback for a pack that names none |

These combine: the pack or the content root names fonts for the text roles, and the frontend bundles a fallback.

Candidates, with licenses as their projects state them (checked 2026-10-01; read the license file in the release
again before committing one):

| Font | Size | Covers | License | Fits |
|---|---|---|---|---|
| Ark Pixel Font | 10 and 12 px; monospaced and proportional builds; released as OTF, TTF, OTB, BDF, PCF | Latin, kana, kanji (Japanese, Chinese and Korean builds) | OFL 1.1 | the dialogue role (the original's is 12 rows tall with 11-pixel kana), and names where proportional is accepted. Actively maintained; its 16 px size was dropped in September 2026 |
| Fusion Pixel Font | 8, 10, 12 px | Latin and CJK, merged from Ark Pixel, Misaki, Galmuri and others | OFL 1.1 | as Ark Pixel, with an 8 px size |
| k8x12 | 8x12, one glyph an 8-pixel cell | kana, JIS level 1 and 2 kanji; half-width Latin | its author's free-software notice: use, copying, modification and redistribution without conditions | Japanese names in the original's 8-pixel cells, at the original's 12-row height. Not OFL: the user should read the notice |
| Misaki | 8x8 (7x7 ink) | kana, JIS level 1 and 2 kanji | as k8x12 | Japanese in 8-pixel cells where 12 rows don't fit |
| Galmuri | 8, 10, 12, 15 px | Latin-1, kana, 6,355 kanji, Hangul | OFL 1.1 | Korean; a DS-era look |
| GNU Unifont | 8x16 and 16x16 | the whole Basic Multilingual Plane | GPLv2+ with the font exception, and OFL 1.1 since 13.0.04 | the last-resort fallback: it has everything, in the original's cell height, but thin, and CJK takes two cells |
| Terminus | 8x16 (and others), with bold | Latin, Greek, Cyrillic: 1,356 characters | OFL 1.1 | Latin names in 8x16 cells with a bold weight near the original's |
| Spleen | 8x16 (and others); BDF, OTB, OTF | Latin-1, Latin Extended-A | BSD 2-Clause | as Terminus; permissive but not one of the three licenses named |
| Pixel Operator (and its Mono, Bold and 8 px cuts) | 16 px and 8 px | Latin | CC0 1.0 (its HB cut is OFL) | proportional Latin at 7 pixels a letter: 8 letters fit the 64-pixel name box |
| Press Start 2P | 8x8, 8-pixel advance | Latin, Greek, Cyrillic | OFL 1.1 | drops into the 8-pixel cells unchanged, at half the height; an arcade look |
| Departure Mono | 11 px, monospaced | Latin | OFL 1.1 | status and debug text |
| Noto Sans, Noto Sans JP, M PLUS 1p, Inter | vector | very wide | OFL 1.1 | option B only |

Under the OFL a subset or otherwise modified font may not keep a Reserved Font Name, if the font declares one.
Committing a release file unmodified, with its license text beside it, avoids the question.

### 2.4 Layout

The original's boxes were sized for 8-pixel cells: 8 cells for a name (64 pixels), 15 for a telop's name and
numbers (120 pixels, half the screen), 8 and 17 for the HUD lines, and 192 pixels by three lines for the chatbox.

- **Cell fonts** (the extracted 8x16 font, or any font with an 8-pixel advance): lay out by cells as today. This
  is the only layout the reference mode uses.
- **Proportional fonts**: lay out by the font's advances and place by pixel width. The telop's centering becomes
  `x = place + (120 - width) / 2`, which is the original's formula when every advance is 8.
- **A name wider than its box**, in order:
  1. if the font stack has a narrower or smaller cut (12, then 10, then 8 pixels), use the first that fits;
  2. let it run past the 64-pixel box into free space: the hand window has nothing to its right but the digits,
     which already follow the name; the hard limit is the telop's 120 pixels less its numbers;
  3. past the hard limit, cut it and end with an ellipsis.
  Pixel fonts don't shrink, so there is no shrink-to-fit in the frame. With option B a name can be condensed
  horizontally to about 80% before the ellipsis.
- **Descriptions**: break lines where the text says (`\n`), as the original does. A line wider than 192 pixels
  wraps onto a free line if there is one (three lines at most), else it is cut with an ellipsis. The engine's
  count of lines comes from the definition's `\n`, never from what the renderer did (§5).
- **Numbers stay the original's digits** in every mode: the HP box, the damage digits with `+` and `x2`, the
  opponent's HP, the judge's numbers, the count box, the banner digits. They are part of the game's look, they
  are separate tiles rather than font glyphs, and a digit has no coverage problem. The digits follow whatever
  width the name took. The turn countdown is the exception: its seconds are lines of the 8x16 font, so they go
  with "TIME UP!" beside them, which in `auto` means the extracted font.
- **The bracketed marks** (`[EX]`, `[SP]`, `[A]`) have no Unicode character. In a font file's text they are
  inline icons: a `[name]` token draws the pack's glyph for it if the pack has one, else its letters in a box.
  (As built they became characters, the Private Use Area's where Unicode has none: §10.5.)

### 2.5 Text beyond ASCII

- **Japanese** needs no shaping: no reordering, no ligatures. It needs glyph coverage, a fallback order, and for
  automatic wrapping the Unicode line breaking rules (descriptions carry their own breaks, so that is rarely
  reached). The 8x16 font spells katakana names itself; kanji, and the hiragana it lacks, need a font file. The
  Japanese ROM's own fonts are a question for the backlog of Japanese content: an extracted Japanese pack would
  bring its fonts the same way the US pack does.
- **Accented Latin, Greek, Cyrillic**: precomposed characters and a font that has them. Strings should be in
  Unicode normalization form C; the content check can enforce that.
- **Arabic, Hebrew, Thai, Indic scripts** need shaping and bidirectional layout: HarfRust and `unicode-bidi`,
  which in practice means cosmic-text and option B. Pixel fonts for them barely exist. They are out of scope for
  the frame text and would be the reason to build option B.
- **Fallback**: per string, not per glyph. A string is drawn whole in the first font of the stack that has all
  its characters, so a name never mixes two fonts.

## 3. Fidelity and verification

The frontend is compared pixel for pixel with the original under emulation, HUD included (frontend.md §4). Any
font but the extracted one differs wherever it draws.

- **`original` is the reference mode**: extracted fonts only, cell layout only, and a string it can't spell is a
  problem the audit reports, as today. The comparison in the verification workspace runs in this mode, passed
  explicitly, so a missing glyph fails loudly instead of falling back quietly.
- **`auto` is the default for watching and playing**: each string is drawn with the extracted font if that font
  spells all of it within its box, else with the font stack. On EXE6's content every string takes the first
  branch, so `auto` and `original` produce identical frames there. That identity is cheap to assert: a test that
  renders the comparison's frames in both modes, or simply that `auto` reports no fallback over a trace.
- **`font`** draws all text from font files. It is what a pack without extracted fonts gets, and what a user who
  prefers one consistent font chooses.
- **Comparing in `font` mode stays possible with text masked.** The text layer knows every rectangle it drew.
  With an option to write those rectangles beside each headless frame, the comparison script can mask them and
  check that everything else is still identical: that catches a enhanced mode that disturbed the sprite limit, the
  layer order or a fade. This is optional; the reference mode is the real check.
- **Tests in this repository** use a small hand-made bitmap font, as the frontend's tests use synthetic assets
  now: layout (centering, overflow, ellipsis, wrap), fallback choice, and exact bitmaps. The bundled font is
  tested through its bitmap form, which is exact (§2.2).

Why `auto` and not `font` as the default: the project measures itself by pixel-exactness, the extracted font is
always there when EXE6's pack is loaded (the frontend can't draw a sprite without the pack), and it is the look
players know. What a font file adds is the strings the extracted font can't draw. Making `font` the default would
trade the original look for consistency in every frame, to fix a problem EXE6's own content doesn't have.

## 4. Content model

### 4.1 What stays as it is

- `name` and `description` in UTF-8 in the definitions, as the canonical text.
- `description`'s `\n` as the line breaks. They are also the line-break hint: no new field.
- The 8-glyph limit is EXE6's box, not a rule of the content model. The content check can warn when an EXE6 name
  isn't spellable in 8 glyphs of `compat/text.toml` ("this name will be drawn with the fallback font"), without a
  pack.

### 4.2 What content should say that it doesn't

- **The run message's words.** Today a navi's `run_message` is characters per line. To draw it at all, in any
  mode, it should be the text (`"line one\nline two"`), with the engine deriving the counts as it derives a
  description's line count. The agent drawing the custom screen will need this first.
- **The HUD's lines and "????".** "TIME UP!", "COUNTER HIT!", the delete lines and the hidden chip's name are
  game text. They should be strings in the content root (a small strings module or table), with their centering
  done by the frontend instead of by padding spaces. The pack's `texts` then only checks them.
- **Which fonts a pack or content root offers, and for which role.** Two roles exist: names (the cell font) and
  dialogue. EXE6's pack offers its extracted fonts for both; another pack names font files. This is the
  frontend's data, never the engine's, like `compat/`: a `fonts` section of the pack's asset index and a small
  table in the content root that maps roles to fonts. No definition and no place in `Content`.
- **Optionally a full name** beside the 8-character one (`full_name = "GunDelSol 3"`), for places with room: a
  library or folder screen, a lobby, option B. Nothing in a battle needs it today, so it should wait for a
  consumer.

### 4.3 Translations must not change the content hash

`Content::hash()` covers the chip records, names and descriptions included, and netplay peers compare it. If a
translation were made by editing `name` and `description`, a Japanese and an English player could not play each
other, and a description with a different number of lines would change the chatbox's timing.

So display text for other languages should be a **strings table per language, keyed by definition** (a chip's
key, a navi's, a HUD line's name), kept in the content root next to the definitions, read only by the frontend
and left out of the hash. The definitions' own text stays the canonical language and the simulation's source for
line counts. A translated description is fitted into the same three-line box by the renderer; it does not have to
have the same number of lines, because nothing in the simulation reads it.

### 4.4 What should stay out of content

- Pixel positions, cell sizes and font sizes of EXE6's HUD: that is the frontend's EXE6 layout (`hud.json` and
  `hud.rs`).
- The text mode and the user's font choice: a preference.
- Rasterizer settings, the fallback order beyond what a pack names, the shadow's shape.
- The digit pictures and the labels of §1.3.
- A per-string style. If a string ever needs emphasis or a color, that is markup in the dialogue text, as the
  original's text commands are, and can wait for a string that needs it.

## 5. Rollback netplay

Text rendering is presentation only, and stays so under this design.

- The renderer reads `&Battle` and the content and writes nothing back. Its own rolling state
  (`Renderer::observe`) is outside `Battle`.
- What text reads of the battle is already on the digest's presentation list (`digest.rs`): which banner shows,
  the telop's chip, `used_chips`, `chip_hud`, `message`. Fonts, glyph caches, layout results and the rasterizer's
  floating-point arithmetic live in the frontend and can't reach the state or the digest.
- Under rollback the frontend draws the presented frame's state (rollback.md §3.1). A mispredicted telop appears
  and goes like any mispredicted sprite. Caches are keyed by string and style, so resimulation doesn't touch them.
- **The one place text touches the simulation is content data, not rendering**: `ChipData::description_lines`
  (the `\n` count, 1 to 3) and the run message's characters per line set when the chatbox takes keys
  (custom-screen.md §3.5). Those come from the hashed definitions. The rule to keep: the engine never asks the
  renderer how text was laid out, and the renderer's wrapping, fitting and translation never change those counts.
- Translated strings are outside the hash (§4.3), so two peers showing different languages or fonts run the same
  simulation.

## 6. Recommendation

### 6.1 What to build

1. **A text layer in the frontend.** The HUD code stops spelling glyph numbers inline. It asks for a string in a
   role (name or dialogue) at a place, with a box, an alignment and where it goes (HUD layer tiles, or sprite
   parts with a priority, bucket and vertical scale). The layer lays the string out and draws it.
2. **Fonts as data.** One in-memory form, a bitmap font: glyph bitmaps in up to three palette indices, an advance
   each, a line height and a baseline. Loaders make it from the pack's extracted fonts (the PNG, its charmap, and
   for the dialogue font its widths), from a BDF file, and from a TTF or OTF pixel font rasterized once at its
   native size with a one-half threshold. A font without a shadow is given one like the original's by offsetting
   the glyph.
3. **Drawn into the 240x160 frame** (§2.1 A), through the existing layers and sprite parts.
4. **Three modes**, `--text original|auto|font`, default `auto`, and `--font <path>[:<px>]` to put a font file
   at the head of the stack.
5. **A bundled fallback font** under an open license, committed with its license text.
6. **Numbers and picture labels unchanged. Banners out of scope.**

### 6.2 Why, and the alternative I rejected

- It answers what a font file is actually needed for: strings the extracted font can't draw, and packs that have
  no extracted font. It does so without giving up a pixel of the original on EXE6's content.
- It needs no change to composition, so priority, fades, squash and the sprite limit can't regress.
- It is small: one dependency for font files, or none if the bundled font is BDF.
- The text layer is worth having on its own. The custom screen is about to add three more text sites and a second
  font; routing all of them through one place is cheaper than a fourth copy of the glyph loop.

**Rejected: a vector font rendered at output resolution on a layer above the frame, as the default text.** It is
what "real font rendering" most often means, and it would give crisp text, any script and shrink-to-fit. Against
it:

- it changes the look of every frame, where the project's measure is the original's pixels;
- it needs the depth-masked text layer of §2.1 B, fades and squash reimplemented for text, and changes to the
  window and the PNG writer;
- it brings cosmic-text's dependency tree (about forty packages) for shaping that Latin and Japanese don't need;
- its output isn't reproducible across platforms, so its tests need tolerances.

It remains available as step 7 below, on top of the same text layer: the layer would hand its laid-out items to
an output-resolution drawer instead of the frame. If the user's aim is crisp, modern-looking text rather than
coverage, that step is the feature, and steps 1 to 3 are still its foundation.

Also rejected: system fonts by default (not reproducible); vector fonts in the frame (§8); mixing fonts within a
string; a font traced from the ROM (can't be committed, and the pack already has the real one).

### 6.3 Plan

Sizes: S is up to about 150 lines, M 150 to 500, L more than that or new architecture.

| Step | What | Size |
|---|---|---|
| 0 | (The custom screen's agent, already under way.) Extract the dialogue font with its width table into the pack; draw the chatbox; give `run_message` its words | theirs |
| 1 | The text layer with the extracted fonts only: text items, cell layout, the two outputs (HUD tiles, sprite parts). Convert the sites of §1.2. No pixel may change: the frame comparison is the test | M (about 400) |
| 2 | The bitmap font type and its loaders: from the pack's fonts; a BDF reader; TTF and OTF through `fontdue` behind a cargo feature. Tests with a hand-made font | M (about 350) |
| 3 | Proportional layout, the overflow rules, inline marks, the font stack and per-string fallback; `--text` and `--font`; the audit reports a fallback as a note in `auto` and a missing glyph as a problem in `original` | M (about 350) |
| 4 | Choose and commit the fallback font with its license; the content check's warnings (a name outside the glyph table or over 8 glyphs, text not in normalization form C); docs | S, plus the choice |
| 5 | The HUD's lines and "????" as content strings; per-language strings tables outside the hash; the frontend's `--lang` | M (about 300, plus content) |
| 6 | Optional: the comparison's text mask (the frontend writes text rectangles; the script masks them) | S, in both repositories |
| 7 | Optional, a separate decision: output-resolution text. The renderer returns text items and a depth mask; the window and the PNG writer composite them; cosmic-text with supplied fonts only; a UI font bundled | L (800 to 1,200, some thirty new crates) |
| 8 | Optional, a separate decision: banners made from a string, for navis a pack adds. It needs a banner-style font drawn for the purpose | L |

Steps 1 to 4 are the feature. Step 5 is what makes translations possible. Each step leaves the reference mode
pixel-identical, and steps 1, 3 and 5 should each end with the frame comparison run.

### 6.4 Crates and files

- `crates/nettai-assets`: a `font` module with the bitmap font type; `Hud` gains the dialogue font (step 0) and, at
  step 5, loses `texts` as a source of words.
- `crates/nettai-content`: `font.rs` (the pack's fonts to bitmap fonts, the BDF reader, the `fontdue` loader behind
  a feature); `hud.rs` and `names.rs` for a `fonts` section of the asset index; a `hud.json` version bump when the
  dialogue font lands.
- `crates/exe6-extract`: `hud.rs` for the dialogue font and its widths (step 0).
- `crates/nettai-frontend`: `text.rs` becomes a `text/` module (layout, drawing, modes, the font stack; the 3x5
  status font moves beside it or is replaced by the bundled font); `hud.rs` and the custom screen's drawing call
  it; `main.rs` gets the options; `audit.rs` the fallback notes; `Cargo.toml` one dependency.
- `crates/nettai-content-check`: the warnings of step 4.
- `content/exe6`: the strings of step 5; `run_message` as text.
- `crates/nettai-battle`: only step 0's `run_message` reading (counts derived from text). Nothing else: the engine
  has no part in text.
- A new `assets/fonts/<family>/` (or the frontend crate's own `assets/`) for the committed font and its license.
- `docs/frontend.md`, `docs/design/asset-formats.md` §4, and this document rewritten as built.
- In the verification workspace: the comparison passes `--text original`; optionally the mask.

### 6.5 Sequencing with the frontend work in progress

Another agent is fixing presentation differences in `crates/nettai-frontend` and is about to draw the custom
screen, which is where the chip window's name, the descriptions and the run message live. `hud.rs` is being
edited now.

- **Don't start step 1 until the custom screen is drawn and merged.** Step 1 rewrites the text sites that agent
  is touching and adding, and its test is "no pixel changes", which needs a settled baseline.
- **Ask that agent for three things now**, each of which it needs anyway:
  - extract the dialogue font with its width table and its charmap (the two-byte glyphs included) beside the
    8x16 font;
  - put the run message's words in the navi definitions;
  - draw its new text through one helper for cell text and one for dialogue text, rather than new copies of the
    glyph loop. Then step 1 is a mechanical change of two helpers and the sites `hud.rs` has today.
- **What can go ahead in parallel without touching the frontend**: step 2 (new files in `nettai-assets` and
  `nettai-content`), the content check's warnings, and choosing the font.

### 6.6 What the user must decide

1. **The aim.** Coverage and independence from the extracted font, with the original look kept (this
   recommendation), or crisp modern text at window resolution (step 7 becomes the feature)?
2. **The default mode**: `auto` (recommended), `font`, or `original`.
3. **The fallback font or fonts.** My suggestion is one family for both roles, Ark Pixel 12 px proportional in
   its Japanese build (OFL: Latin, kana, kanji), and to audition it in the hand window and a telop beside two
   cell fonts before committing: Terminus 8x16 bold (OFL) for Latin names in the original's cells, and k8x12 for
   Japanese in 8-pixel cells if its license notice is acceptable. Whether to commit a CJK font now (a large file)
   or Latin now and CJK with the Japanese content is part of this.
4. **Whether numbers change.** Recommended: no, in every mode.
5. **Whether banners are in scope.** Recommended: no; step 8 if a pack ever adds navis.
6. **Whether translations are wanted**, which decides whether step 5's strings tables are built now or the HUD's
   lines just move to content.
7. **Where the committed font lives**: with the frontend (a fallback for any content) or in `content/exe6` (EXE6's
   own choice). Recommended: the frontend's, with a pack or content root able to name its own over it.

## 7. Risks and open points

- **The custom screen may show more text than §1.2 lists.** The chip window's name and the chatbox are read from
  the original's code; the screen's other labels look like pictures but have not been drawn or compared yet. The
  agent drawing it will settle that.
- **The shadow.** The 8x16 font's shadow is painted per glyph. A generated one (the face offset right and down)
  is an approximation, which only shows in fallback strings.
- **A fallback string next to original ones** (one chip of a hand in another font) will look uneven. That is the
  price of `auto`; `font` is the consistent choice for a pack that needs the fallback often.
- **Font size on disk.** A CJK pixel font holds thousands of glyphs and is probably megabytes; I did not download
  one to measure. It should be loaded from a file at run time rather than compiled into the binary, and its size
  checked before it is committed.
- **The font candidates were not auditioned in the frame**, apart from the three pixel fonts of §8. Their licenses
  and sizes are from their projects' pages; how each looks beside EXE6's HUD is still to be seen (step 4).
- **`fontdue` has one maintainer and no hinting.** For pixel fonts at their native size neither matters. If it
  ever does, `ab_glyph` gave identical bitmaps in the spike and also reads embedded bitmap strikes, and a
  BDF-only path needs no crate at all.

## 8. Measurements

Throwaway spikes, run outside the repository against the user's own pack and locally available open fonts.
Nothing from them is committed.

- **The fonts' shapes** (§1.1): read from the ROM's bytes. The 8x16 font uses pixel values 0, 1 and 2 only and
  ink rows 2 to 15, almost all in rows 3 to 14; 200 of its 224 glyphs are a full 8 pixels wide. The dialogue
  font runs for about 460 glyphs, up to where the 8x16 font's data begins, in pixel values 0, 1 and 3; its first
  width table gives 8 for capitals, digits and the space, 6 for `I`, 6 to 8 for lowercase and 11 for kana and
  kanji.
- **Coverage of the content** (§1.4): every chip of `content/exe6` spelled against `compat/text.toml`.
- **Pixel fonts shipped as outlines are exact at their native size.** Three fonts (a 16 px proportional one, its
  8 px cut, and an 8x8 arcade font) rasterized with `fontdue` 0.9.4 and `ab_glyph` 0.2.32 at their native sizes:
  every coverage value was 0 or 255, every advance a whole number of pixels (7, 5 and 8), and the two crates'
  results were identical. The same fonts drawn at four times the size on a 960-pixel-wide canvas match the
  frame-resolution drawing scaled up, so an output-resolution layer adds nothing for them.
- **A UI font in the frame is not usable.** Noto Sans at 11 px gave a spread of some 150 distinct gray levels
  over one eight-letter name, different between the two rasterizers; drawn into the frame it is soft with
  antialiasing and uneven without. Noto Sans JP kanji at 10 and 12 px in the frame are unreadable. Both are sharp
  on the 960-pixel canvas, where they no longer look like the game.
- **Widths**: "GunDelS3 120" is 96 pixels in the original's cells, 96 in the 8x8 arcade font, 79 in the 16 px
  proportional pixel font and 71 to 74 in the UI fonts at 11 px. Seven katakana and a digit are 64 pixels in the
  original's cells and 78 in a 12 px full-width font.
- **Dependencies**: the spike built with `fontdue` (`ttf-parser`, `hashbrown`) and `ab_glyph`
  (`ab_glyph_rasterizer`, `owned_ttf_parser`) in about six seconds. `cargo tree` for cosmic-text 0.15 with its
  system-font feature off lists about forty packages.

## 9. As built (2026-10-02)

### 9.1 The user's decisions

The user's request: "implement native font rendering for places in the ui with dynamic text". On the questions of
§6.6:

1. **The aim: crisp text at the window's resolution.** Option B of §2.1: a vector font drawn at the output's
   resolution over the scaled 240x160 frame. Step 7 of §6.3 became the feature; the `auto` mode and the pixel
   fallback font of §6.1 were not built.
2. **The default mode: the font.** `--text font|original`, default `font`. `original` is today's exact pixel
   rendering, the reference the frame comparison and the tests use.
3. **The font: a bundled open-license font**, committed with the frontend with its license text, so the text looks
   the same everywhere. The user chose **Murecho** ("murecho supports jp text and looks mostly like your current
   font"), over my first pick, Saira (§9.4); the description text's weight is the user's too (300).
4. **Numbers, banners and other pictures stay pixel art** (§6.6 4 and 5).
5. **The font lives with the drawing**: `crates/nettai-render/fonts/murecho/` (nettai-frontend's until the drawing
   became its own crate), with `OFL.txt`. `--font PATH` puts another TrueType or OpenType file in its place.
6. Translations (§4.3) and the HUD's lines as content strings (§4.2) were not part of it. Translations came
   next: §10.

### 9.2 What is text and what stays a picture

Every string below comes from content or varies; each is drawn in the original's box for it.

| String | Role | Box (the original's) | In front of it (depth) | Fades |
|---|---|---|---|---|
| The next chip's name (the hand window) | fitted cell | its glyphs' cells from (0, 144); the damage digits stay after them | the HUD layer's | the HUD layer's, the screen's |
| The chip window's name (custom screen) | fitted cell | the window's 8 name cells where its map puts them, cut to the window's columns while it slides | the HUD layer's | as above (Beast Out's darkening, a Cross's whitening) |
| A telop, "????" for a hidden one | cell | the name's cells, centered; the digits after them | its sprite parts' (front layer, bucket 6) | the sprites', the screen's; the banners' squash |
| The chip the other player just used | cell | as a telop, without the squash | its sprite parts' | as a telop |
| The Program Advance animation's names and codes | cell | 8 cells for the name, the 9th for the code | the HUD layer's | the HUD layer's |
| The enemy names (round's first screen) | fitted cell | their glyphs' cells on BG0, ending at column 30 | BG0's | BG0's (the HUD palettes') |
| The HUD's lines: the turn timer's seconds, "TIME UP!", "COUNTER HIT!" | cell | the cells of the pack's line without its padding spaces | the HUD layer's | the HUD layer's |
| "VS" between the judge's numbers | cell | 2 cells | the HUD layer's | the HUD layer's |
| A description's and the run message's lines (the chatbox) | dialogue | 12 rows, every 14 rows from (51, 108), as wide as the open box's inside (to a pixel short of its right frame: 160 pixels in the description box, 184 in the message box; the line buffer's 192 run past the description box); cut where the line buffer's sprites end, plus 3 rows, which hold a third line's descenders whole | its 18 sprite parts' (front layer, bucket 3: the key-wait arrow and the portrait are in front) | the sprites', the screen's |

Not text, so unchanged: the Crosses' names in the Cross window (the pack's pictures, by each Cross's game); the
custom screen's slots and picked column, which show icons and the code strips' letters, no names; chip codes,
the damage digits, HP numbers, the judge's numbers, banners, "Cstmzing...", "PAUSE", the "????" beside the mugshot;
the frontend's own status text (`text.rs`).

### 9.3 How it works

- **Collecting** (`textlayer.rs`): the places that draw text ask a `TextSink`. In the enhanced mode, for a string the
  font has every character of, they leave its glyphs out of the frame and hand the sink a `TextItem` (the words,
  the role, the box, the alignment, a clip, the face and shadow colors of the palette the original draws it with,
  the squash, how many units are printed). A string the font lacks a character of is drawn whole in the game's
  font, as in the original mode (a string never mixes two fonts). What the glyphs leave behind is the original's
  background: blank HUD-layer cells, the chip window's name cells in the window's color 8, blank sprite parts in
  place of a telop's glyph parts (as many, where they were, so the sprite limit drops the same objects), the
  chatbox's 18 sprites blank. The helpers are `fonts::layer_text` and `fonts::layer_line`; the telop, the custom
  screen's names and the chatbox ask the sink themselves.
- **Depth** (`compose.rs`): `compose_with_depth` also returns, per frame pixel, the depth key of what won it:
  priority, then a sprite before a layer (layers by their order), then among sprites the earlier part
  (`depth_key`; 0 is in front of everything, which the window's status text uses). An item's key is its layer's,
  or its frontmost sprite part's (`SpriteList::insert_tagged` tags the parts it stands in for; an item whose parts
  the limit dropped isn't drawn). A pixel shows an item where the frame's key there is at least the item's: a
  banner, the mugshot, the key-wait arrow or a priority-0 sprite in front of HUD text hides it. Approximations: a
  semi-transparent sprite in front hides the text instead of blending over it, and mosaic doesn't reach text.
- **`Renderer::render` returns a `Frame`**: the picture, the depth keys and the items, each given its key and its
  fades: its layer's (the HUD layer's for the HUD layer and BG0, the sprites' for sprite items) and then the
  screen's. The HUD layer's shake (the custom screen's) moves its items with it.
- **Colors and fades**: the face and the shadow are the palette's indices 1 and 2 (the chip window's 9 and 10, as
  its name's pixels are shifted by 8; the chatbox's index 1 and no shadow), through `apply_fade` as the frame's
  own pixels go. So a dimming leaves text as it leaves the HUD, Beast Out's fade darkens the custom screen's names,
  a Cross's choice and the FlashBomb's flash whiten them, and the screen fades take everything.
- **The squash**: an item's `vscale` is the banners' `n / 256` texture step; the text is scaled vertically by
  256 / n about the box's middle (until 2026-10-14 cut to the box's 16 rows, as the sprite is; no longer: §9.9).
- **Drawing** (`present.rs`, `vfont.rs`): the window (`app.rs`) and the PNG writer (`headless.rs`) both go through
  `present`: the frame scaled to the output, then each item laid out and drawn at the output's scale from its
  glyphs' distance fields (§9.10) and blended over it, pixel by pixel through the item's clip and the depth keys of
  the frame pixel under it.
- **The scaling policy**: the picture takes the largest whole multiple of 240x160 that fits the output, centered
  on black, so every frame pixel is the same square whatever the size; only an output smaller than 240x160 gets a
  fractional, shrunk picture. The window is resizable and follows this; text is drawn at the same placement and
  scale. Headless PNGs are `--png-scale` times 240x160, exactly.
- **The rasterizer**: swash 0.2 (§2.2): shaping with the font's kerning (in the frame's pixels, independent of the
  output's scale), the outlines at variable fonts' axes. Until 2026-10-14 swash rasterized each glyph at the
  output's size (8-bit coverage, hinting under 40 pixels an em), cached by glyph, size, axes, scales and a
  quarter-pixel horizontal phase; since then each glyph is a signed distance field made once from its outline and
  sampled at every size (§9.10). Layouts are cached by string, role and box. Eight packages came with swash (swash,
  skrifa, read-fonts, font-types, zeno, yazi, bytemuck, bytemuck_derive); cosmic-text wasn't needed.

### 9.4 Layout

- **Roles.** The 8x16 font's strings (`Role::Cell`): capitals 10 frame pixels high (the original's take rows 4
  to 13 of the 16), the baseline 14 rows down, weight 700, a shadow in the palette's shadow color half a frame
  pixel right and down (the original's is a pixel; half reads better at 3x and 4x). Those whose cells something
  follows right under them, the chip window's name, the hand window's and the enemy names (`Role::FittedCell`,
  since 2026-10-14, §9.9): as `Cell`, its capitals 10 pixels high on the baseline 14 rows down, the original's, but
  with short descenders: the part of each glyph under the baseline drawn 0.31 times as high, so that a descender
  takes one row and its shadow the next, the cells' last, as the original's do. The dialogue font's (`Role::Dialogue`): capitals 9 pixels high, the baseline 10.5
  rows down, weight 300, no shadow. **A role has one size** (since 2026-10-13, §9.8): every string of it is drawn
  at it, whatever its length or its box. **No glyph is cut at its box's rows** (since 2026-10-14, §9.9).
- **Kana and kanji** stand in the capitals' rows, as the original's fonts draw them (the 8x16 font's kana take
  rows 4 to 13, its capitals' rows; the Japanese dialogue font's kana and capitals both rows 1 to 10). A Japanese
  font draws them in its em box, lower and taller than its capitals (Murecho's bold kana are 1.1 times its
  capitals' height and reach 0.05 em under the baseline), so the text layer draws a string's kana and kanji at the
  size that makes the font's kana box as high as the capitals, centered on them (`vfont::Face`: the median top and
  bottom of the ink of the font's full-size katakana at the role's weight, measured once). One measure of the
  font's, so kana, kanji and Latin of a string, and of every string, share one size and one baseline; kanji come out
  a little taller than kana (about 11 rows to 10 in the 8x16 font's role), as the original's do.
- **Fitting.** The box is the original's: the cells its glyphs take (one a character the pack's font lacks, up to
  the place's room), or where nothing follows a name (the chip window, the Program Advance's names) the whole of
  its cells. A string that fits is drawn at its natural width and never stretched; a cell string leaves a pixel
  free at the box's right, as the original's glyphs do before the digits. A string too wide is drawn in narrower
  cuts down to the width axis's "condensed" (75) for a font that has the axis, then **narrowed horizontally to its
  box**, its height and baseline unchanged; never made smaller (until 2026-10-13 it was squeezed to 85%, 70% with
  kana or kanji, and then made smaller and dropped onto the capitals' middle, so the names of one list came in
  different sizes, and Japanese ones smaller and lower than the original's: §9.8). It never leaves its box. What
  the bundled font narrows, by the chip names in their own cells (a cell a character) and the chatbox lines in
  their boxes, measured over en.toml and ja.toml of all three games:

  | Game, language | Names | Narrowed | How much | Chatbox lines narrowed |
  |---|---|---|---|---|
  | EXE6 English | 350 | 116 | to 83-99% (Magnum the most) | none of 1,072 |
  | EXE6 Japanese | 350 | 349 | to 62-79%, 185 of them under 65% (three kana in three cells, バリア, the most) | 4 of 1,056, to 98% |
  | EXE5 English | 356 | 117 | to 83-99% (Magnum) | none of 1,044 |
  | EXE5 Japanese | 357 | 357 | to 62-79%, 185 under 65% | none of 1,036 |
  | EXE4 English | 339 | 139 | to 82-99% (NumbMnDS) | none of 961 |
  | EXE4 Japanese | 339 | 339 | to 62-79%, 169 under 65% | none of 950 |

  These are the cell role's (the telop's), and the fitted role's (the hand, the chip window, the enemy names): the
  two are the same size, and short descenders change no width (§9.9).

  Every Japanese name is narrowed to about two thirds, because its box is the original's: the 8x16 font's kana are
  half-width, 7 pixels of ink in an 8-pixel cell and 10 high, where a Japanese font's are square. Narrowed to 62-70%,
  Murecho's kana come out 6.5 to 7.5 frame pixels wide and 10 high: the original's own proportions, and they read
  as the original's do (looked at in the chip window, the hand, the telop and the enemy names at 3x and 4x, §9.8).
  So nothing else is done for them: no abbreviation, no smaller size. A Japanese name shorter than 8 cells in the
  chip window or the Program Advance's names (whose box is the 8 cells whatever the name) has room and is drawn at
  its natural width. A chatbox line's box is the open box's inside, which the frontend reads off the box's map
  (`chatbox::text_room`: the right frame's inner edge on a text row), not the 192-pixel line buffer, which runs
  past the description box's frame (until 2026-10-02 the Japanese Crosses' longest lines did too); the test
  `every_chatbox_line_fits_its_box` measures EXE6's (the four Japanese Crosses' lines that are narrowed: DustCross,
  ElecCross, SlashCross and TomahawkCross, 164 pixels in 160).
- **Alignment**: left, as the original's glyphs start their cells; a telop's name centered in its cells.
- **Marks**: a mark is one character (§10.5). One the font has is the font's (Ⓡ, ✕, ○); else the layer draws it:
  a button (Ⓐ, Ⓑ) as its letter at 68% of the size in a ring centered on the capitals, a stacked mark (EX, SP) as
  its two letters one above the other in one cell.
- **Printing**: a chatbox line is laid out whole and drawn up to the units the chatbox has printed, so the line
  doesn't move as it grows.
- **Nothing reaches the simulation**: the chatbox's timing stays the content's counts (the description's line
  breaks, the run message's characters per line), never the layout (§5).

### 9.5 The font: Murecho

Murecho (Neil Summerour, Positype; SIL Open Font License 1.1, no Reserved Font Name), the Google Fonts release
(`ofl/murecho`, from the upstream project at 0efba44c), unmodified but renamed: `Murecho-VariableFont_wght.ttf`,
1,430,664 bytes, compiled into the binary.

- **Coverage**: 4,450 characters: Latin, Latin Extended, Greek, Cyrillic, 189 kana and 2,337 kanji. Every character
  of every name and description in `content/exe6` is in it, and of the 442 characters the pack's two fonts draw
  only 伊, 祐 and 綾 (story characters' names, which no battle string uses) are not. Japanese works as is: no
  shaping is needed, swash's shaper handles it; kana and kanji are drawn as tall as the capitals, centered on them,
  and narrowed into the original's cells (§9.4).
  A string with a kanji it lacks is drawn in the game's font (whose kanji are fewer still).
- **Weights**: one variable file, 100 to 900. Names, telops and the HUD's lines use 700 (bold, the weight the
  8x16 font's glyphs have); the chatbox uses 300, the user's choice, near the dialogue font's thin strokes.
- **The look**: a clean, slightly squared humanist sans with open shapes and a large x-height, legible at the
  10-pixel capitals of the HUD; it sits well beside EXE6's pixel art.
- **Size**: 1.4 MB of the frontend's 13 MB release binary. Not subset: every glyph a Japanese chip name or description
  could need stays in.
- **Before it**: I had picked Saira (OFL), a squarish technical sans with a width axis, so that a name too wide
  for its cells condensed within its family; auditioned in the HUD and the chatbox at 4x beside Barlow Semi
  Condensed, Exo 2, Inter, Oxanium and Russo One. It has no kana or kanji. The user chose Murecho, which looks
  close to it and covers Japanese; without a width axis, narrowing takes the axis's place.
- **Other scripts**: Arabic, Hebrew, Thai or Indic text would need a font for them and bidirectional layout
  (§2.5); swash shapes them, the layout here is left to right only. A font stack (a string in the first font that
  has all of it) is the way to add one beside Murecho; today there is one font and the game's as the fallback.

### 9.6 Verification

- **The original mode is unchanged.** Every frame of 201 scenarios, the two golden traces (machgun and soundmod),
  the sample, the custom-screen list and the chatbox list, 272,833 frames, renders byte for byte the same PNG (and
  `known.tsv`) with main's frontend and with this one in `--text original`. Against mGBA, machgun is still exact on
  all 2,404 frames (1,667, and the custom screen's 737).
- **The comparison pins the mode**: the verification workspace's `frontend-compare` scripts pass `--text original`
  to a frontend that has the option (a baseline from before it gets nothing extra).
- **Tests** (`cargo test -p nettai-render`): the depth mask; text hidden where a sprite in front won and faded
  with its layer; a font-mode telop as an item over blank parts where the original's glyph parts were; a HUD line
  without its padding; layout that never leaves its box and isn't stretched; a string too wide narrowed, its
  glyphs' size and baseline kept; kana and kanji at one size, their box the capitals' height on the capitals'
  middle, in every string; the placement policy; marks (a stacked mark in one cell, a button in its ring: §10.5);
  descenders drawn past the cells, and the fitted role's ending in them at 1x to 6x, its capitals the cell role's
  pixels on the original's baseline (§9.9). The font tests check
  layout and metrics, not pixels (the rasterizer's arithmetic is floating point, §2.2), except the descender
  tests, which check only which output rows are lit.
- **Looked at**, headless at 4x: the hand, the chip window, telops through their squash and stretch, a hidden
  telop, the other player's chip, a description and the run message (printing too), the Program Advance's names,
  "COUNTER HIT!", the seconds, "TIME UP!" and "VS", Beast Out's darkening of the enemy name, a Cross's whitening,
  Japanese names and a description (from a content copy with Japanese words), and the window (`NETTAI_WINDOW_SHOT`
  keeps its last picture). `--objects` lists each frame's items with how many of their box's pixels something in
  front covers.

### 9.7 Left open

- Text at 1x output is soft, as any vector text at that size (§8), and unhinted since §9.10; the HUD wants 3x or
  more.
- A HiDPI screen: the window (iced) presents at its size in points, so the text is drawn at that resolution and
  the system scales it up (`NETTAI_PHYSICAL_PIXELS` draws it at the display's density, at four times the cost on a
  Retina display: docs/frontend.md §7).
- `--audit` reports what the pack's fonts lack, in either mode; a string the vector font lacks (drawn in the
  game's font) isn't reported.
- The HUD's lines as content strings (§4.2): not done; they stay the pack's text script, per language (§10).
  Translated strings tables (§4.3) are built: §10.

### 9.8 One size a role, kana on the capitals (2026-10-13)

The user (2026-10-13): "the text rendering in game for sharp text looks a little janky in places, some japanese
is misaligned and the variable font size looks kind of funny".

**What was wrong**, measured on frames rendered headless at 3x and 4x in both text modes (the ink's rows and
columns in each text box against the original mode's):

- **The sizes.** A string too wide for its box was squeezed to 85% (70% with kana or kanji) and then made smaller,
  each by what it needed, so the names of one list came in different sizes. In Japanese nearly every name was made
  smaller: the 8x16 font's cells are half-width, Murecho's kana at the role's size are 13.7 frame pixels wide, so a
  Japanese name needed 56 to 75% and was drawn at 80 to 100% of the role's size, by its length.
- **Japanese off its line.** Murecho draws kana and kanji in its em box, which is lower and taller than its capitals:
  its bold kana are 1.11 times its capitals' height and reach 0.05 em under the baseline, its kanji 1.27 times and
  0.1 em under. The original's fonts draw kana in their capitals' rows (the 8x16 font's kana, capitals and digits all
  take rows 4 to 13, in each of the three games' fonts that has kana; the Japanese dialogue font's rows 1 to 10). With the
  shrink, which also moved a string down to keep it on the capitals' middle, the chip window's バッドメディスン
  stood in rows 13.5 to 23.75 where the original's stands in 10 to 22 (1.75 pixels low at the bottom, 2 short),
  the digit of スプレッドガン1 stood higher than its kana, and each name sat at its own height. The cause was the
  fit and the font's metrics for kana; no offset was tuned for Latin only (each role's baseline and capitals are the
  original's, and Latin was where it should be). In the chatbox the kana happened to be close (9.5 pixels, centered).
- **Descenders out of their cells.** Murecho's descenders reach 3.3 pixels under the 8x16 font's baseline, the
  original's one: the chip window's "Spreadr1" drew its p over the chip's picture, the enemy name's g ran past its
  bar. (The cut that answered it, below, is gone since 2026-10-14: §9.9.)
- **A comma closing words up.** The US scripts write "Lan,this is no time to run away!" (EXE6's and EXE5's MegaMan,
  20 strings in the three games): the original's fonts give a comma a capital's advance, so the words stand apart;
  the font's comma is narrow and joined them.

**What the text layer does now** (`vfont.rs`; §9.4 has the rules):

- every string of a role at the role's size; one too wide narrowed to its box, its height and baseline kept; the
  table of §9.4 says what is narrowed and how much (every Japanese name, to 62-79%, the original's own half-width
  proportions);
- kana and kanji at the size that makes the font's kana box (`Face`: the median ink of its full-size katakana at the
  role's weight, measured once a weight) the capitals' height, centered on them: the chip window's name now stands
  in rows 11 to 22.5 (the original's 10 to 22, its voiced marks reaching row 10), the description's first line in
  108.5 to 119.25 and 97.5 pixels wide (the original's 108 to 119 and 98), and Latin, kana and kanji of a string
  share one baseline;
- (until 2026-10-14) a cell string was cut at its box's rows, as the original's glyphs stay in their cells; since
  then no glyph is cut, and the chip window's, the hand window's and the enemy names' text, at the cell role's size
  on the original's baseline, has short descenders that end in their cells: what each glyph's outline has under
  the baseline is drawn 0.31 times as high, before it is rasterized (§9.9);
- a comma a letter follows at once is followed by a space's width (not between digits: "1,000").

**Left as they are, listed:**

- In the chip window and the Program Advance's names, whose box is 8 cells whatever the name, a short Japanese name
  has room and is drawn at its natural width (キャノン) while a long one is narrowed to about two thirds
  (バッドメディスン): their kana differ in width from name to name, never in height. The hand, the telop and the
  enemy names, whose boxes are the names' own cells, narrow every Japanese name alike. Drawing every kana at the
  original's cell width would make them alike everywhere, at the cost of narrowing names that fit; not done.
- Murecho's digits in Japanese text are its proportional ones, where the Japanese dialogue font's are full-width
  (11 pixels): "前方のてき1たいを" is tighter around its 1 than the original. The font's design.
- Murecho has no hinting (hinted and unhinted glyphs are the same pixels from 20 to 37 pixels an em, measured), so
  `HINT_BELOW` changes nothing with it, and a string's kana and Latin are drawn alike at every scale. Narrowing thins
  vertical strokes against horizontal ones (Murecho's bold kana at 62%: 0.10 em against 0.145); looked at, it reads
  as the original's narrow kana do, so the strokes aren't thickened to make up for it.

**Looked at**: headless frames at 2x, 3x and 4x, in the enhanced mode before and after and in the original mode, of
each game in each language: the custom screen with a full hand (the chip window's name for two chips of the hand,
the enemy name), the hand window with its damage, a telop (EXE6's; the EXE5 and EXE4 matches' other side used no
chip), a chip's description and the no-running message (the chip lab's recordings: custom/description-keys and
custom/run-message, EXE5's custom/description and custom/run-message, EXE4's custom/describe and
custom/run-message), from match files with long names (WindRack, Spreadr1, VarSwrd, AntiRecv: フウジンラケット,
スプレッドガン1, バリアブルソード, バッドメディスン); every chip name of EXE6 in both languages, in its own cells and
in 8 cells, and of EXE5 and EXE4 in Japanese, on sheets with the capitals' rows marked. The ink was measured in each
box against the original's.

**The original mode is unchanged**: with packs that have both languages' lettering it never reaches the vector
font, and the verification workspace's identity check (`tools/frontend-compare/identity.sh`, main's frontend against
this one, both `--text original`) found every frame byte for byte the same: EXE6's chatbox and custom-screen
lists in English and drawn in Japanese (68 scenarios each, 62,351 frames each), EXE5's and EXE4's lists (209
scenarios, 196,197 frames; 449, 538,975) and their Japanese consoles' (9, 10,845; 53, 95,249), the two of the
Japanese EXE6 consoles' list whose recordings the lab still has (2,891 frames; the other 141 aren't in it), and
with the last commit's frontend EXE6's sample (173 scenarios, 215,699 frames) and its chatbox list drawn in Japanese
again (27, 15,182). Only `nettai-render` changed; the simulation reads nothing of it, so no digest moved.

**Cost**: the frame's text as before. Drawing the custom screen's busiest text (the chip window's name, the enemy
name, three description lines) over a 4x frame, cached, took 180 µs in English and 236 µs in Japanese before and
182-189 µs and 225-236 µs after (3x: 108 and 139 µs before, 111-114 and 132-138 after; the development profile,
the least of eight batches of 200 frames; scaling the frame's picture takes 0.9 ms at 4x). A first frame (layouts
and glyphs made) is 2 to 4 ms in either. A string's layout is cached as before; the kana box is measured once a
weight, from 45 outlines.

### 9.9 No glyph cut, short descenders (2026-10-14)

The user (2026-10-14): "also letters like lowercase g get cut off when rendered in sharp text mode in the game".

Then, of the first fit (a baseline raised to 12.4 and capitals of 9 pixels, below), the user (2026-10-14): "the
baseline of the text is not aligned correctly anymore", of the chip names and the enemy names.

**The rule since**: no glyph is cut in the enhanced mode, anywhere. Where a role's descenders, drawn whole, would cover
something the original draws under its cells, the role keeps the original's baseline and size and its descenders
are drawn short, so they end in the room it has: what each glyph's outline has under the baseline is scaled down
vertically, by one factor for every string of the role, so the names of a role stay alike; the rest of each glyph
is the font's, whole and at its size. Where nothing of the original's is under them, they are drawn past the
cells, whole. Kana and kanji keep §9.4's rules (one baseline with Latin, the kana box the capitals' height), and
what of them reaches under the baseline is shortened the same way. `vfont::Role` has a role for each case of the
8x16 font's strings: `Cell` (drawn past) and `FittedCell` (short descenders); `Style::in_rows`, the cut, is gone.

**What was cut.** Until then each cell string was cut at its box's 16 rows (§9.8). Murecho's deepest descender at
weight 700 is g's, 0.235 em under the baseline, 0.323 of its capitals' height (0.728 em); p, q and y reach 0.224 em
(0.308 of the capitals), j 0.205, the comma 0.139, the parentheses 0.118 (and the same at 300). With the cell role's
capitals of 10 pixels on its baseline at row 14, g's ink ends on row 17.2 and its shadow on 17.75 to 18.0 (2x to
6x; 19 at 1x), measured on rendered frames: the cut took its last 1.75 to 2 pixels. Kana and kanji end on rows 15.0
to 15.7 with their shadow and were never cut. The original's descenders take one row under the baseline (row 14)
and their shadow the next (15): they end on the cells' last row.

**What is under each role's cells**, read from the original mode's frames at 1x in each game (the boxes are the same
in both languages):

| Where | Role | Box | What starts under its cells | Choice |
|---|---|---|---|---|
| The chip window's name | `FittedCell` | 8 cells from (16, 8) in EXE6 and EXE5, (8, 8) in EXE4 | the chip's picture on the next row (EXE6, EXE5: the name's band is rows 8 to 23, the picture starts on 24); the window's frame line (EXE4, row 24) | fitted to the 16 rows |
| The hand window's name | `FittedCell` | its cells from (0, 144) | the screen's edge (row 160) | fitted to the 16 rows |
| The enemy names | `FittedCell` | their cells, ending at column 30, rows 0 to 15 | their bar takes rows 3 to 13, its bottom the baseline; the original's descenders and their shadow are rows 14 and 15, under the bar and in the cells; then the field (EXE6, EXE5; EXE4 shows none) | fitted to the 16 rows, where the original's end |
| A telop; the other player's chip | `Cell` | the name's cells from row 32 | the field: no window or bar | drawn past, through the banner's bounce too (stretched 1.23 times for two frames, where the original's own glyphs are cut at the sprite's rows) |
| The Program Advance's names and codes | `Cell` | 9 cells, a name every 24 rows | the field, over the 8 rows to the next name, whose capitals start 4 rows into its cells | drawn past: a descender ends 2 rows into the 8 |
| The HUD's lines, "VS" | `Cell` | their cells | nothing to cut: no descenders in either language ("TIME UP!", "COUNTER HIT!", the seconds) | drawn past (nothing changes) |
| The chatbox's lines (description, run message) | `Dialogue` | 12 rows, every 14 rows | the line buffer's sprites, cut 3 rows under the third line's 12 (row 43 of the text) | as before: the third line's g ends on row 41.4 (2.9 pixels under its baseline at 38.5), never cut |

**The fitted role** (`Role::FittedCell`). Its room is the cells' 16 rows, and its capitals and baseline are the
original's: capitals 10 pixels high in rows 4 to 13, the baseline at row 14, as the cell role's (the same pixels:
the test `a_fitted_cells_capitals_stand_on_the_originals_baseline`). Under the baseline are 2 rows, 14 and 15; the
original's descenders take the first and their shadow the second. So the fitted role's descenders are drawn 1 pixel
deep (`Style::descender`), the shadow (half a frame pixel rounded to output pixels, at least one: 1 pixel at 1x,
2/3 at 3x, 3/5 at 5x, 1/2 at 2x, 4x and 6x) under it.

- **The factor**: the deepest descender of g, j, p, q and y (`vfont::DESCENDERS`), measured from the font's
  outlines once a weight: Murecho's g, 0.235 em under the baseline at weight 700. At the capitals' 10 pixels
  (13.74 pixels an em, Murecho's capitals being 0.728 em) that is 3.23 pixels; drawn 1 deep, the factor is
  1 / 3.23 = **0.31**. Every glyph of the role is shortened by it, so its descenders stay alike: g comes out 1.0
  pixel deep, p, q and y 0.95, j 0.87, the comma 0.59, the parentheses 0.50 (before the output's rounding).
- **Where**: on the outline, between the font's scaling and the rasterizer (`vfont::Shorten`): the points of the
  glyph's outline under the text's baseline are moved toward it by the factor, and the outline is drawn as any
  other (since §9.10 its distance field is made from it, in the field's own units, so one field serves every
  size). So a short descender is as sharp as the rest of the glyph at every scale, not a squashed picture. A
  glyph's field is cached with its shortening.
- **Deeper than g**: a glyph that still reaches past the cells with its shadow (Murecho's double low line ‗, 0.34
  em; the cedillas and Ŋ, a hair deeper than g at 0.237 and 0.242) is shortened just as much more as ends it
  there, so nothing leaves the cells, whatever the string. Since §9.10 "there" is the cells' last row less the
  shadow's offset at its most (one frame pixel, 1x's), the same at every size: such a glyph ends where g does, 1
  pixel under the baseline (it was 1.5 less the shadow's offset in output pixels, 1.0 to 1.4 pixels from 2x up).
- **Kana and kanji**: on their box (§9.4) most kanji and some kana reach under the baseline, 0.6 pixels for the
  median kanji, 0.9 for the deepest (琴's, 妹's, 舞's feet), the full-width marks further (＠ 1.1, ＿ 1.6). With the
  shadow they all ended in the cells already, except those marks; they are shortened as Latin is, being under
  the same baseline: a kanji's foot comes up 0.4 to 0.6 pixels.
- **What isn't**: the cell role (the telop, the Program Advance's names, the HUD's lines) and the chatbox keep the
  font's descenders whole; a button's ring (Ⓐ, 0.75 pixels under the baseline) isn't an outline and isn't
  shortened; it ends in the cells with its shadow. A round capital's overshoot under the baseline is shortened
  with the rest, by under a tenth of a pixel.

Measured on rendered pixels (the tests `a_fitted_cells_descenders_end_in_its_cells` and
`a_fitted_cells_capitals_stand_on_the_originals_baseline`, 1x to 6x), in rows of the cells: the capitals take rows
4 to 13 at every scale, the cell role's pixels; g, j, p, q and y end on row 15.0 (row 14 their last) and with their
shadow on 16.0 at 1x and 15.5 to 15.7 at 2x to 6x (the cell role's g ends on 17.25 to 18.0); kana end on 14.0 to
14.5, kanji on 14.3 to 15.0, and with their shadow on 14.7 to 16.0; accents start inside the cells.
Nothing leaves the cells, and nothing in front of them is covered.

**Narrowing**: the fitted role is the cell role's size, so it narrows what the cell role does (§9.4's table), more
than the first fit at 90% did. In their own cells (the hand; a cell a character), measured over en.toml and
ja.toml's chip names: EXE6's English 116 of 354, the most to 83% (11 at 90%), EXE5's 117 of 357, to 83% (16 at
90%), EXE4's 139 of 339, to 82% (18 at 90%); the Japanese all but one in EXE6 (353 of 354) and all in EXE5 (357) and
EXE4 (339), to 62-79% (69-88% at 90%), as before the first fit.

**The first fit** (2026-10-14, replaced the same day): the fitted role drawn at 90%, its capitals 9 pixels high,
on a baseline raised to 12.4 (rows 3.4 to 12.4, the original's 4 to 14), so that the descenders, whole, ended in
the 16 rows. It kept the descenders' shape at the cost of the names' place and size, which the user saw at once.

**The original mode is unchanged**: with packs that have both languages' lettering it never reaches the vector
font, and the 15 states below rendered in `--text original` are byte for byte the same with main's frontend and this
one; with short descenders, the 16 states of the shots below in both languages (32 frames at 3x) in `--text
original` are byte for byte the same with the first fit's frontend and this one. (The stand-in for a pack with no
lettering in the language shown, which draws pixel fonts into the frame in the original mode (§15),
lays strings out by these roles too; no comparison uses it.)

**Looked at**, headless at 3x, before and after: the chip window (EXE6's Spreadr1 and BigBomb, EXE5's and EXE4's
Magnum, スプレッドガン1), the enemy names (SpoutMan, GyroMan, アクアマン), the hand window (Spreadr1, Magnum,
スプレッドガン1), a telop at rest and in the banner's stretch (ComingRd), the Program Advance's names (Sword, WideSwrd,
LongSwrd: the chip lab's pa/0x153-lifesrd/recipe1), a description (Spreadr1's), from match files with folders of
names with descenders: each descender whole, nothing under the cells covered. With short descenders, at 3x, each
beside the same frame in `--text original` and the first fit's, in English and Japanese: the chip window (EXE6's
Spreadr1, BigBomb and YoYo, EXE5's Spreader, BugBomb and YoYo, EXE4's Spreader and Geddon1), the enemy name (EXE6's
and EXE5's MegaMan), the hand (the same names), and at 6x the chip window's BigBomb and スプレッドガン1 and the enemy
name: the capitals and kana on the original's rows, the descenders a row deep, their shadow on the cells' last row.

### 9.10 Distance-field glyphs (2026-10-14)

The user (2026-10-14), of the first custom screen's stutter on a 4K display in the enhanced mode: "can you use
distance field text?"

**What it was.** Each glyph was rasterized from its outline at the output's size (swash and zeno, 8-bit
coverage), the image cached by glyph, size, axes, scales and quarter-pixel phase: a glyph was made again at each
size, each phase and each narrowing, at a cost that grew with its area (13x is 178 pixels an em).

**What it is.** Each glyph is a signed distance field (`sdf.rs`), made once from its outline and drawn at every
size by sampling it at each output pixel:

- **Made once**: a field is keyed by what changes the outline's shape: the glyph, the variable font's axes and
  the short descenders (§9.9, now in the field's units, the floor a fixed one frame pixel). Its size, narrowing
  (`xscale`, a stacked mark's stretch) and squash (`vscale`) are transforms of the sampling, not cache entries.
  `Made::glyphs` counts fields made, and nettai-frontend's preload makes those of every name and description of
  both folders and navis ahead (`TextRenderer::prepare`), at no size: a window of any size, or one resized, draws
  them without making any.
- **The field**: 128 texels an em (`sdf::RES`), distances held 3 texels either way (`SPREAD`), stored in 256ths
  of a texel (i16; 10 to 35 KB a glyph). The cell role's glyphs are 13.7 frame pixels an em, so a texel is 1.4
  output pixels at 13x, 1 at 9x and half one at 5x. It is computed from the outline itself, with no dependency
  added: the curves cut into segments straying at most a tenth of a texel from them, each texel's sign by the
  nonzero rule (each row's crossings), its distance the least to a segment within the spread. Murecho's contours
  overlap (a variable font: e is a bowl and a bar, most kanji are strokes laid over each other); where a row's
  winding passes from inside to inside, the segments are cut where they cross and only the pieces with the glyph
  on one side are its edge (without that, the inside of a stroke had distances near 0 and showed faint flecks at
  13x).
- **Drawn**: each output pixel's center samples the field bilinearly; the distance in output pixels (across the
  field's gradient when the glyph is narrowed or squashed, so each direction is antialiased at its own scale) is
  ramped linearly over one output pixel, the coverage of a straight edge (a smoothstep is sharper than coverage,
  and the outline's coverage is what it stands in for). The glyph's origin is placed exactly across, so the
  quarter-pixel phases are gone, and on a whole output row down, as before (the chatbox's baseline is half a frame
  pixel down: placed exactly, its horizontal strokes straddled two rows at odd scales). Runs of texels deep inside
  are filled whole and those far outside skipped (`sdf::Runs`, at the depth a size's ramp needs), so only the
  pixels near an edge are sampled. The shadow pass, the clip, the depth keys, the fades and the button marks'
  rings are as before.
- **Why not multi-channel (MSDF)**: a plain field rounds sharp corners by about a texel, which MSDF avoids at a
  lower resolution. At 128 texels an em the rounding is under 1.4 output pixels at 13x, and against the outline
  rasterized at the same size (the test `a_field_draws_what_its_outline_covers`, Murecho's e, S, W, g, B, ガ, 熱
  and 撃 at 5x and 13x) the coverage differs by 0.1 to 0.6% of the ink in all, by more than a quarter at 0 to 0.4%
  of the ink's pixels (kanji corners), never by more than 0.55. MSDF's edge coloring and error correction are a
  good deal more code for a difference not visible here; what the resolution costs is the making (below). At 64
  texels an em the kanji's stroke ends looked rounded at 13x.

**Looked at** (headless, a frame of 19 strings over plain fields: chip names in each role, narrowed ones, a stacked
mark, a squashed and a stretched telop, deep descenders, two English and two Japanese chatbox lines with a button
ring), old and new side by side at 1x, 2x, 3x, 5x, 9x and 13x, and zoomed 4 to 6 times at 3x and 13x: the same
shapes, edges as sharp (the sum of neighbor differences, new over old: 1.003 at 3x, 1.001 at 5x, 0.999 at 9x, 0.998
at 13x), corners as square to the eye at 9x and 13x (W's, the kanji's stroke ends). Pixels more than 96 levels
apart: 0.01 to 0.02% from 2x up, nearly all the double low line's new floor (§9.9). What differs: at 1x the text is
unhinted and its ramp narrower (0.6 pixels: the spread is 0.3 output pixels there), a little more aliased (edge
energy 1.10; 1x is only the stand-in for a pack with no lettering); a pixel at a sharp corner can be up to half off
at 9x and up.

**Cost** (release build, a loaded ten-core machine, ranges of repeated runs):

| | Before | After |
|---|---|---|
| A glyph made | zeno at 13x: 3 to 9 µs Latin, 10 to 42 µs kana and kanji (less at smaller sizes), again at each size and phase | 8 to 27 µs Latin, 35 to 80 µs kana and kanji, once for every size |
| The 19-string frame's text drawn, cached (3x, 5x, 9x, 13x) | 0.5, 1.4, 4.1, 10.1 ms | 1.4, 2.7, 5.5, 9.5 to 10.0 ms |
| Its first drawing, over the cached one | 1.5 to 2.7 ms | 5.4 to 5.7 ms at every size |
| A match (EXE6, 1,500 ticks, every picture presented at 5x, 9x, 12x, 13x): the average picture | 3.7 to 3.9, 9.9 to 10.6, 16.6 to 17.4, 19.6 to 20.5 ms | the same within the machine's noise: 3.7 to 3.9, 10.1 to 10.4, 16.7 to 17.6, 20.3 to 20.5 ms |
| Its first custom screen at 13x (79 pictures: the first, the mean) | 18.3 to 18.9, 15.3 to 16.0 ms | 18.3 to 18.8, 16.0 ms |
| Glyphs made in the match after the preload | 126 to 180 (12 to 14 on the first custom screen), on 27 to 28 pictures | 1 (none on the first custom screen), on 1 picture |
| The preload | | 4 to 5 ms more in English, 10 to 15 ms in Japanese (the folders' and navis' glyphs) |

So a cached picture's text costs more to draw at small sizes (it is sampled where it was copied), about the same at
13x, and in a match's pictures, whose text is a few strings, the difference is within the noise. Making a glyph
costs 2 to 3 times what rasterizing it at 13x did, once instead of at every size; the preload makes the match's
glyphs ahead, so a match makes almost none. On this machine glyph making was never the stutter: the first custom
screen made 12 to 14 glyphs, under 0.2 ms, and its pictures were no slower than the rest.

## 10. Languages and display text (as built, 2026-10-02)

The user's requests: "add localization support so you can play either with japanese or english text"; then "the
english text should also be extracted into locales/en.toml. the actual luau code etc should be free from text, all
text should be stored via the locales file"; "why does ChipSpec even have description_lines? why not count it from
the locale string?"; and to call it strings or display text, not "words".

The frontend's `--lang en|ja` (default `en`) shows a battle's text in English, as the US games do, or in Japanese,
as the Japanese games (JP Falzar and Gregar) do, in either mode. No definition holds display text: every
string is a content root's `locales/<lang>.toml`.

### 10.1 What a battle shows as text, and where each comes from

| What | English | Japanese |
|---|---|---|
| A chip's name (the hand window, a telop, the other player's chip, the chip window, the Program Advance's names) | `locales/en.toml` `[chips]` | `locales/ja.toml` `[chips]`, the Japanese ROMs' chip names (キャノン, ハクシャク for Count) |
| A chip's description (R on the custom screen) | `en.toml` | `ja.toml`, the Japanese ROMs' descriptions |
| A Cross's description | `en.toml` `[forms]` | `ja.toml` `[forms]` |
| The no-running message (L) | `en.toml` `[navis]` `run_message` | `ja.toml` `[navis]` `run_message` |
| The enemy names (a round's first custom screen) | `en.toml` `[navis]` `name`, the ROM's name for the navi's NameID (ChrgeMan, GrndMan, TmhkMan, ProtoMan); `variant_name` when the side's rules ask (`battle.set_name_variant`: EXE5's Hub Style, BCMegaMn) | `ja.toml` `[navis]` `name` (ロックマン, キラーマン, アクアマン, ブルース...), `variant_name` (FCロックマン) |
| A Cross's name (the frontend's own text: live play's terminal summary, the plain-text screen's Cross window) | `en.toml` `[forms]` | (English) |
| A background's name (the app's arenas) | `en.toml` `[backgrounds]`, the US ROM's menu name of its area, as the menu writes it (RobCtrlComp; docs/frontend.md §1) | `ja.toml` `[backgrounds]`, the Japanese ROM's menu name (秋原エリア, ロボせいぎょPCの電脳) |
| A patch card's name (gen-content checks them; the frontend's match description and the editor's card list show them) | `en.toml` `[patch-cards]`, the fan translation's | `ja.toml` `[patch-cards]`, the Japanese ROMs' card names |
| A patch card's effect lines (the editor's card lists show them; verify's trace-tests patch_card_text writes and checks them) | `en.toml` `[patch_card_effects]`, the fan translation's (EXE5's: the US ROMs') | `ja.toml` `[patch_card_effects]`, the Japanese ROMs' |
| The HUD's lines (the seconds, "TIME UP!", "COUNTER HIT!"), "VS", "????" | the pack's text script, in the US font's glyphs | the same words, in the Japanese font's glyphs (the pack's Japanese lettering) |
| The 8x16 font and the dialogue font | the pack's | the pack's Japanese lettering: the Japanese ROMs' fonts, in their encoding |
| Banners | the pack's | ten differ (the pack's `-ja` banners): ROCKMAN, KILLERMAN, AQUAMAN and BLUES where the US has MEGAMAN, ERASEMAN, SPOUTMAN and PROTOMAN, each starting where its longer or shorter name does; the Program Advance's プログラムアドバンス. The other 37 are the same pictures |
| "Cstmzing..." | the pack's, eight tiles | カスタム中…, seven tiles (the Japanese `sub_801CA34` copies one column fewer) |
| The gauge's "L or R" | the pack's | the Japanese gauge's |
| The chip window's pictures for OK (no data selected; chip data transmission), the re-deal and scrap (TRASH CHUTE) | the pack's | the Japanese pictures (DUST SHOOT) |
| The Cross window's names | the pack's, by game | the Japanese ROMs' katakana (アクア, トマホーク...), by game |

The same in all four ROMs, so in both languages: BATTLE START and the other banners, "PAUSE", the custom gauge's
"CUSTOM", the window's frame, its turn-limit block, the chip codes, every number, the chatbox's box, the Beast Out
buttons, the sprites, and the chip pictures but for region art (verification workspace tools/jp/locale/pictures.py
and banners.py compared them). The region art (the Gregar Beast Out picture, BatCan's picture, which the US ROMs
blank, and the Bass and BassAnly icons) differs between the US and the Japanese ROMs as pictures, not text: left as
the US's (the user's choice).

### 10.2 Where the strings live

- **Every display string is a content root's `locales/<lang>.toml`**, keyed by definition key: `[chips]` (name,
  description), `[navis]` (name, variant_name, run_message), `[forms]` (a Cross's name and description),
  `[backgrounds]` (a background's name, by its asset name: the app's arenas, docs/frontend.md §1), and a table
  for each of the game's collections, by its name (`[patch_cards]`: a patch card's name), and the game's text tables
  that no definition owns (top-level tables of key to text the game's manifest declares, `text`: `[patch_card_effects]`, a patch card's effect lines by the
  effect's kind and choice). What reads each is §10.6. The engine's
  `content::strings::Strings` is one table; nettai-content's `locale` reads them. A definition holds none: the define
  phase refuses a `name`, `description` or `description_lines` field (core.d.luau's specs have none).
- **The content's own language** (`locale::OWN`, English for EXE6) is part of the content: the loader puts its table
  in `Content::strings`, and the define phase counts what the battle reads of it into the records: a description's
  lines (`ChipData::description_lines`, `FormData::description_lines`) and a no-running message's characters per line
  and which of them move the speaker's mouth (`RunMessage::counts`, `talking`). The hash covers the records, so it
  covers that shape; the strings themselves are presentation, out of it (renaming every chip changes nothing; a
  description with another number of lines changes the content). Nothing in the definitions duplicates the strings.
- **Another language's table is a frontend's alone.** It changes neither the battle nor `Content::hash()`, so two
  players can each read their own language in one netbattle. A translated description may have another number of
  lines than the own one; the battle keeps the own one's timing.
- **Since 2026-10-25 the game's strings in every table are the Legacy Collection's** (§14): what follows here is
  how the tables were first written, from the ROMs.
- **`en.toml` is the US ROMs' strings** (written once from the definitions' former text, which was the US ROM's,
  but for what the content names itself: the chips the US release cut and named otherwise or not at all, Count's
  and Django's; the Crosses' names; the patch cards' names; the three Giga chips' descriptions, which no ROM has). **`ja.toml` is
  the Japanese ROMs'**, written once by the verification workspace's `gen-content locale-draft ja`. Both are people's
  since, and `gen-content check` compares them with the ROMs (one check for both languages, `locale::check_table`):
  every chip's name and description, every Cross's description, every navi's name (by its NameID) and no-running
  message, and that the two Japanese ROMs agree. The DblBeast, Gregar and Falzar chips' scripts print a gift's text
  the save keeps (`FF 01 nn`); `ja.toml`'s are the text Tango's Japanese netplay saves hold, with a comment saying so,
  and the check only asks that the font can draw them.
- **The Japanese ROMs' text encoding** is compat/text.toml's `[jp]`: what each byte of their 8x16 font and dialogue
  font draws (written once from the fonts: each glyph whose bitmap is one of the US fonts' is that glyph's character,
  the rest read off the fonts; the verification workspace's tools/jp/locale/textjp.py). The extractor writes the
  Japanese fonts' charmaps with it; gen-content decodes the Japanese text with it and checks that every Japanese chip
  name encodes back to the ROM's bytes.
- **Pictures with text, and the fonts, are assets**: the pack's lettering in another language, extracted from the
  Japanese ROMs as `-ja` files (asset-formats.md §4, "Languages"; exe6-extract `lettering`).
- **The HUD's lines stay the pack's text script** (§4.2 is still open): the Japanese ROMs' are the same English
  words, which the Japanese font draws with the same pictures.

### 10.3 How the frontend uses them

- `--lang ja` loads the pack's graphics, swaps the Japanese lettering into the HUD's and the custom screen's fields
  (`Bundle::for_language`; nothing that draws them changed), and loads `locales/ja.toml`. A language the content
  root has no table of stops the frontend with what there is.
- **A pack without the language's lettering** (one made from the other release's ROMs alone: a pack states the
  languages it has, asset-formats.md "Languages") is drawn in its own language, the content's strings still in the
  chosen one: its pictures with words (the banners, the gauge's word, the chip window's pictures, the Crosses' names,
  the waiting words) are the language it has them in, never placeholders. Its fonts are that language's too, without
  most of the chosen language's characters (the US font has no kana; the Japanese fonts have 36 and 26 of the 62
  Latin letters and digits), so the strings are drawn in other fonts in either mode: the enhanced mode's vector font
  as always, and in the original mode pixel fonts into the 240x160 frame (§15: `Renderer::render`, `pixeltext`),
  rather than in the other language's glyphs. The frame comparison has nothing to compare there: no console draws a
  language its ROM has no font for.
- The places that draw content's text (hud.rs, custom.rs, chatbox.rs) ask the frame's `TextSink` for it
  (`strings.rs`, `DisplayText`): the language's string, else the content's own, else the key; `--audit` lists what
  the language's table lacks. The frontend's own text (live play's status line, folder listings) is the content's own
  strings.
- **`--text original --lang ja`** draws the Japanese text in the Japanese fonts as the Japanese games do: a string is
  encoded with the Japanese font's characters, as the English is with the US font's. **The enhanced mode** draws it with
  Murecho, which has the kana and kanji, and the dialogue font's symbols (✕, ○: the Japanese descriptions'
  "攻撃力✕2"); the marks it lacks the text layer draws (§10.5).
- **Timing stays the content's own.** A translated description shows its own lines, whole, in the proportion of the
  own description's lines printed (16 of the 410 Japanese chip descriptions, and HeatCross's and SpoutCross's, have
  two lines where the English has three); a translated no-running message prints in the proportion of the own
  message's characters printed, so it ends when that would. A Japanese console times its chatbox by its own text, so
  those boxes take keys a tick or more apart from the engine's; nothing else in the battle reads text.
- **The Japanese games' HUD code** (docs/engine/jp-differences.md §5), drawn on every console where the user chose it
  (nettai has the US release's presentation where the releases differ, with named exceptions, this among them): the
  custom screen's close also starts the chip window's
  HUD task, so the next chip's name shows from the screens' exchange through the turn's banner (`HudState`); "Cstmzing..." is as wide as its picture. One fix the Japanese consoles showed: a palette flash of variant 0 leaves the HUD layer as the
  transformation's fade left it (black), where the frontend had cleared that fade.
- **No key switches the language while the window runs** (the user's choice for now): the renderer borrows one
  bundle in one language.

### 10.4 Decisions made along the way

- The navis' English names are the ROM's names for their NameIDs (what the original draws as the enemy names):
  ChrgeMan, GrndMan, TmhkMan and ProtoMan, where the content had ChargeMan, GroundMan, TomahawkMan and "Navi 11"
  (the user's choice). gen-content reads them from the ROM's `TextScriptNaviChipNames`.
- A string may be empty: the invalid chip's name is, and the Japanese games print no description for some chips (the
  dark and arm chips' are empty lines, as the ROM has them).
- The language is the player's: the comparison with the original draws each console in its own language
  (verification workspace frontend-compare/console-lang.py: Japanese for a trace of a Japanese console).
- Developer labels in code (the dimming controllers' `phases { name = ... }`, which name a controller in an error
  message) are not display text and stay in the code.

### 10.5 The game's marks as characters

The user's request (2026-10-02): "stuff like [B] and [cross] should really be unicode characters. also the letters
in EX and SP should be stacked on top of each other for rendering rather than side by side."

The game's fonts have glyphs no letter or kana is: buttons, a cross, two letters stacked in one cell. They were
bracketed names (`[A]`, `[EX]`, `[cross]`) in compat/text.toml, the strings tables and the pack's charmaps, and the
frontend parsed a bracketed name as one unit. Each is now one character: a real Unicode symbol where one fits, else
one codepoint of the Private Use Area, the same in compat/text.toml's US and Japanese encodings (one glyph, one
character, so a string encodes and decodes the same way and a mark is one character as the battle counts them).

| Glyph (the old name) | In the fonts | Character | In the bundled font |
|---|---|---|---|
| The A, B, L and R buttons (`[A]`, `[B]`, `[L]`, `[R]`) | 8x16 0xB7, 0xB5, 0xB4, 0xB6 (US; the dialogue font draws them too) | Ⓐ U+24B6, Ⓑ U+24B7, Ⓛ U+24C1, Ⓡ U+24C7 | Ⓡ only: the others drawn as their letter in a ring |
| The zenny sign (`[z]`) | 8x16 0xB3 (US), 0x99 (JP) | Ƶ U+01B5 | no |
| A circle (`[circle]`) | dialogue E4 1F | ○ U+25CB | yes |
| A cross (`[cross]`: "攻撃力✕2", attack ×2) | dialogue E4 20 | ✕ U+2715. Not ×: the 8x16 font's 0x99 is ×, a smaller glyph, and one character can't name two glyphs; not ✚, an upright cross, which this diagonal one isn't | yes |
| Brackets (`[bracket1]`, `[bracket2]`) | dialogue E4 E6, E4 E7 (US) | `[` and `]` | yes |
| A small full stop (`[.]`, one pixel where `.` is two by two) | dialogue E4 E8 (US) | ﹒ U+FE52 | no |
| R over V (`[RV]`) | 8x16 0x40 (US), 0x84 (JP) | U+E000 | drawn stacked |
| B over X (`[BX]`) | 0x41, 0x85 | U+E001 | drawn stacked |
| E over X (`[EX]`: the EX navi chips) | 0x42, 0x94 | U+E002 | drawn stacked |
| S over P (`[SP]`: the SP navi chips) | 0x43, 0x95 | U+E003 | drawn stacked |
| F over Z (`[FZ]`) | 0x44, 0x96 | U+E004 | drawn stacked |
| M over B (`[MB]`) | dialogue E4 1B | U+E005 | drawn stacked |
| The bat (`[bat]`, a picture) | 8x16 0xA0 (US), 0x81 (JP) | U+E006 | no |
| End (`[End]`) | dialogue 0xE0; 8x16 0xC6 (JP) | U+E007 | no |
| D over S (EXE5's DS navi chips) | EXE5's 8x16 0x9F (US), 0x85 (JP); EXE5's dialogue 0x9F; EXE6's fonts none (an EXE6 match has no EXE5 chip to name) | U+E008 | drawn stacked |
| V over 2, 3, 4 and 5 (EXE4's) | EXE4's 8x16 and dialogue fonts: 0x94 to 0x97 (JP); V5 alone in the US fonts, 0x5C | U+E009 to U+E00C | not yet: the game's font |
| A square (`[square]`, EXE4's) | EXE4's dialogue 0x60 (US), E4 2A (JP) | □ U+25A1 | unchecked |

- **Where they are written**: compat/text.toml (`"\uE002"`, as TOML escapes: a Private Use Area character shows as
  nothing in most editors), locales/{en,ja}.toml (`"Count\uE002"`, `"Press Ⓐ\nto burn a..."`), gen-content's
  built-in charmap and the verification workspace's tools/jp/locale/textjp.py. The battle's strings use Ⓐ, Ⓑ, ✕,
  U+E002 and U+E003; the others are in the encoding only.
- **The original mode** draws the same pixels as before: the charmaps (`font_chars` and the dialogue font's `chars`
  in `hud.json`, written from compat/text.toml by the extractor) map the characters to the same glyph numbers.
- **The enhanced mode**: a mark the font has is drawn by the font (Ⓡ, ✕, ○ with Murecho). One it lacks the text layer
  draws (`vfont::mark`): a button as its letter at 68% of the size, centered in an antialiased ring 1.15 times the
  capitals' height with a stroke a tenth of it; a stacked mark as its two letters in one cell, each as high as half
  the capitals less a gap (10%), the first on top, so the pair spans the capitals' height as the game's glyph does,
  drawn 1.55 times wider and a weight heavier than the text (the game's stacked letters are as wide as its others,
  half as high) and hinted at their size, so they stay crisp at 3x and up. A mark with no drawing that the font
  lacks (the bat, End, Ƶ, ﹒) sends its string to the game's font, as any character the font lacks does.
- **The battle** counts characters (a description's lines; the run message's characters per line and which move
  the mouth): a mark is one, as the bracketed name was, so no record and no hash changed.
- **The hyphen of "power-up"** (the user's "fix character", 2026-10-02: SpoutMan's descriptions had read
  "powerーup"). The US ROM's text has byte 0xA1 there, which the encodings name ー, the Japanese fonts' long vowel
  mark; the US fonts draw 0xA1 as they draw 0x98, the hyphen (the same pictures in the 8x16 font and the dialogue
  font, the same advance). So the US encoding reads 0xA1 as a hyphen too: two bytes draw one character, as 0x00
  and 0xB1 both draw a space, and a string is encoded with the first (0x98), which draws the same pixels. The
  Japanese encoding keeps ー: its fonts draw a long vowel mark. A sweep of the US ROM's battle text (chip names and
  descriptions, the Crosses' descriptions, the names by NameID) found no other glyph the US fonts draw as an ASCII
  one: the rest of its non-ASCII text is the marks, and ミテイ (未定, "not decided"), which the US ROM really has
  as the description of 57 chips (0x15E-0x17B, 0x181-0x18F but 0x185, 0x19B-0x1A7), in katakana; en.toml keeps
  it for the 17 of them the content defines.
- **Checked**: `--text original` renders byte for byte the same PNGs as before (the frontend before the change with
  its pack, against this one with a pack extracted again): 206 English scenarios and traces, 275,844 frames (the
  sample, the custom-screen and chatbox lists, machgun, soundmod and the EX and SP navi chips), and the 142 Japanese
  consoles' traces in Japanese, 171,684 frames. gen-content's check passes (the ROMs' names and descriptions decode
  to the tables' characters). Looked at headless at 3x: Count's EX and SP in the chip window, the hand and the telop;
  DustCross's description with its Ⓑ; ✕ in SpoutCross's Japanese one.

### 10.6 What reads each table

The user's question (2026-10-02): "are the forms/weapons sections even used in the locales?"

- `[chips]`: names on the HUD, the telop, the custom screen and the Program Advance's names; descriptions in the
  chatbox (R), whose lines time it.
- `[navis]`: the enemy names on the round's first custom screen; the no-running message (L), whose characters time
  the chatbox.
- `[forms]`: only the Crosses'. A Cross's description is R in the Cross window, and its lines time that chatbox (the
  define phase counts them into `FormData::description_lines`); its name is the frontend's own text, live play's
  terminal summary of the Crosses drawn and the plain-text screen's Cross window (`strings::own_form_name`). The
  other 15 forms' names (the base form, the Beasts, the Crosses' Beast forms, Beast Over) were read by nothing and
  are gone; the check refuses a form that isn't a Cross, and the own language must name every Cross. R looks the hovered
  Cross up by its form (`CrossWindow::hovered`; docs/engine/custom-screen.md §4.1).
- `[backgrounds]`: a background's name in the app's arenas (the Practice preview's rounds); the check refuses a
  name the pack hasn't, and the own language must name every background of the pack.
- `[weapons]` (124 names: 69 navi and form weapons', 55 patch card weapons') was read by nothing (no screen
  shows a weapon's name; `Strings::weapon` had no caller): the table, `WeaponStrings` and its check are gone.
- `[patch_cards]` (a table of the game's collection's, `EntryStrings`; any collection of the game's root has one
  by its name, `[navicust_programs]` too): the patch cards' names, by card id, which gen-content checks (the English
  ones there, the Japanese ones the ROM's); no screen of the frontend shows them yet. (Until the cards became an
  engine definition kind, 2026-10-02, they were records and their names a `[records]` table.)

## 11. Lettered pictures and the enhanced text: a survey (2026-10-14)

The user (2026-10-14): "you should also survey all the sprite assets that have language-specific text and see if
they can be replaced with enhanced rendering". Then, of its scope: "don't survey chip art or character sprites or
backgrounds".

This is a survey; nothing was changed. Each lettered picture of the battle interface is listed with what it says
and how it is drawn, and judged as a candidate for drawing its words with the text layer (§9.3, `vfont.rs`) at the
window's resolution in the enhanced graphics, keeping the art behind the words.

### 11.1 What was surveyed, and how

- **The lists**: the extractor's lettering (`crates/nettai-extract/src/exe6/lettering.rs`, `exe5/lettering.rs`,
  EXE4's `lettering` in `exe4/graphics.rs` and `exe4/custom.rs`), every picture under the packs' `graphics/hud` and
  `graphics/custom` (EXE4's, EXE5's and EXE6's packs of all four ROMs, so with their `-ja` files), the custom
  window drawn from `window.png` through `custom.json`'s `window_maps` and EXE6's `form_list_maps`, the content's
  banner roles (`rules/roles.luau` `banners`, the navis' `banners`) and the renderer's drawing (`nettai-render`
  `hud.rs`, `custom.rs`).
- **Left out on purpose** (the user's scope): every chip's picture and icon; every character sprite (navis,
  viruses, their attacks, effects and portraits, the emotion window's faces among them); every battle background.
  Whatever lettering those have is not in this survey.
- **Not lettering**: chip codes (`codes.png`, `slot-codes.png`: A to Z and `*`, the same in every language) and
  every number (HP, damage, the count box, the banner digits, the custom screen's digits), which stay pictures
  (§9.1, 4); the hidden chip's `?` and the HUD layer's "????" (`hidden-icon.png`, `layer.png`); the Beast Out and
  soul pictures and EXE6's scrap button, which have no words.
- **Contact sheets** of every picture below at three times their size, the custom window drawn from its maps:
  `target/lettering-survey/exe4-lettering.png`, `exe5-lettering.png` and `exe6-lettering.png` (rendered from the
  packs in the survey's worktree, not committed).

### 11.2 The verdicts, and what the text layer has

- **(a) Easy**: plain letters on nothing, or on a plain or separable background. The text layer can draw the words
  in the game's colors in a role like `Role::Cell`.
- **(b) Possible with work**: styled lettering (an outline, a two-tone fill), which needs the text layer to grow a
  style, or art behind the words that has to be separated from them.
- **(c) Keep as pixel art**: a logo, letters that are part of a button or a frame, or no clean background in the
  pack or the ROM. The packs come from the ROMs alone (no drawn or generated pictures), so a background the ROM
  doesn't have can't be made.

What the text layer has today (`textlayer::TextItem`): a face color and an optional shadow (half a frame pixel
right and down) from the palette the original uses, its layer's fades, the banners' squash (`vscale`), a clip, and
the depth of what it stands in for. No outline, no two-tone fill, one font (Murecho).

Every banner but the Program Advance's has one look: cream capitals with a paler lower half (a two-tone fill) and
a dark navy rim, heavier right and down, on the blue bar, in 20 cells of 8x16 (`BannerLayout::glyphs`: the ROM's
20 pointers into one set of letter pictures, each letter with the bar behind it). The bar has cells of its own in
that set: its left end, its body (the space between words), its right end, and the transparent filler. EXE6's 45
banner pictures hold 52 distinct 8x16 cells (29 without the two Program Advance pictures), EXE5's and EXE4's 51
each. So **the clean bar is in the ROM**, and in the pack's own banners, and needs no new pixels.

### 11.3 EXE6

| Asset | Where it shows | English | Japanese | Size | Drawn as | Moves | Look beyond letters | Verdict |
|---|---|---|---|---|---|---|---|---|
| `hud/banners/*` but the Program Advance's (43 pictures; `-ja` for four navis' names) | a round's and a turn's start (`battle-number-start`, `turn-start`), `final-turn`, the result (`enemy-deleted`, `you-win`, `you-lose`, `draw`), the judge's (`hit-damage-judge`), and in a link battle the navis' `NAVI-win` and `NAVI-deleted`. In the pack, named by no role or definition: `battle-start`, `battle-number`, `finish`, `time-up`, `flag-deleted`, `round-fight`, `max-mode`, `penalty`, `liberate-success`, `liberate-failed`, `turn-liberate` | BATTLE n START!, TURN n START!, FINAL TURN!, ENEMY DELETED, YOU WIN!, YOU LOSE, DRAW!, HIT DAMAGE JUDGE, CHARGEMAN WIN!, TOMAHAWKMANDELETED (no room for the space)... | the same Latin letters, with ROCKMAN, BLUES, KILLERMAN and AQUAMAN for MEGAMAN, PROTOMAN, ERASEMAN and SPOUTMAN, each starting where its length puts it | 160x16, 20 cells of 8x16 | five 32x16 sprites (enhanced: `smooth::Overlay`, at the window's resolution) | squash and stretch: 5 ticks in, a bounce to 1.23 and 1.14, 0x30 held, 5 out; the numbered ones' digits (`banner-digits.png`) at `number_at` | two-tone fill, dark rim, on the bar | **(b)**: an outline and a two-tone fill; the bar's cells are in the ROM |
| `hud/banners/program-advance.png`, `program-advance-empty.png` | a Program Advance, and a selection that makes none | PrgAdvance | (the next row) | 160x16, about 90 pixels inked | as the banners | as the banners | italic white letters, a black outline, over a green arrow: a logo | **(c)**: a logo; no arrow without it |
| `hud/banners/program-advance-ja.png`, `program-advance-empty-ja.png` | as above, in Japanese | | プログラムアドバンス | 160x16, about 85 pixels inked | as the banners | as the banners | white katakana with a gray rim, nothing behind | **(b)**: the outline; no background to keep |
| `hud/pause.png` | the screen's middle while a player holds the battle | PAUSE | the same picture | 40x16 (a 32x16 and an 8x16 sprite: five glyphs) | sprites, in the enemy HP's palette | none | white letters, a dark slate rim | **(b)**: the outline alone; nothing behind |
| `hud/waiting.png`, `waiting-ja.png` | the HUD layer at column 22, row 4, while the other player is still choosing | Cstmzing... | カスタム中… | 64x16 (8x2 tiles); Japanese 56x16 (7x2) | HUD layer tiles, the banners' palette | blinks every 32 frames | a cream face with a paler lower half, no rim | **(a)**: one face color (both tones would want the banners' fill) |
| `hud/gauge.png` (the frame's word) | the custom gauge, top middle | CUSTOM | the same (`gauge-ja.png`) | 32x8 (4 tiles) of the 128x16 sheet | HUD layer tiles, the gauge's | none | light letters shading to blue, a dark rim, on the gauge's frame line | **(b)**: rim and shading; the frame line's blank cells are the gauge's own |
| `hud/gauge.png`, `gauge-ja.png` ("L or R") | the full gauge's cells | Ⓛ or Ⓡ (letters in rings) | L or R (plain letters) | two frames of 32x8 (4 tiles each) | HUD layer tiles (`GAUGE_L_OR_R`) | the frames alternate every 8 ticks, cyan and orange | none: letters on the empty cells | **(a)**: the rings are `vfont::mark`'s; the two colors are the frames' |
| `custom/turn-limit.png` | the HUD layer at column 15, row 4, beside the window, when the turn limit shows | FINAL TURN | the same picture | 56x16 (7x2 tiles) | HUD layer tiles, the layout's palette | none | yellow letters, no rim | **(a)** |
| `custom/window.png`: "BATTLE CHIP" | the chip window's top frame | BATTLE CHIP | the same tiles | about 70x8 | the window's map on the HUD layer | slides with the window (12 pixels a tick), shakes and fades with it | engraved into the frame, two tones | **(c)**: part of the frame |
| `custom/window.png`: "CHIP SELECT" | the header over the slots | CHIP SELECT | the same tiles | about 50x8 | as above | as above | small dark letters on the header strip | **(b)**: the strip's blank cells are in the window's tiles (not checked cell by cell); small letters, little to gain |
| `custom/window.png`: OK | the OK button | OK | the same tiles | about 24x16 | as above | as above, the cursor's corners over it | white letters filling a red button | **(c)**: no blank button |
| `custom/window.png` on the Cross window's maps: "▲CROSS SELECT", "CROSS SELECT", "B:EXIT" | the tab under the chip window; the Cross window's header and foot | ▲CROSS SELECT, CROSS SELECT, B:EXIT | the same tiles | about 60x8 and 40x8 | the form list's maps on the HUD layer | the window's three opening steps | dark letters on the frame's strips | **(b)**: as CHIP SELECT |
| `custom/window.png`: the Cross window's empty plates | a place with no Cross | CROSS | the same tiles | about 48x16 a plate | as above | as above | engraved into the plate | **(c)** |
| `custom/form-names-falzar.png`, `form-names-gregar.png`, their `-ja` | the Cross window: a Cross's name beside its face; palette 10 for the one under the cursor, gray for one not available | SPOUT, THAWK, TENGU, GRND, DUST; HEAT, ELEC, SLASH, ERASE, CHRG | アクア, トマホーク, テング, グランド, ダスト; ヒート, エレキ, スラッシュ, キラー, チャージ | 72x16 a name (9x2 tiles), ten a version (colored and gray) | the window's tiles | the hovered one's palette | white letters with a heavy dark outline on an orange bar, the Cross's face in the same tiles | **(c)**: the letters fill the bar, and no blank bar exists; the short names (THAWK, GRND) are no string the content has |
| `custom/pictures/ok.png`, `ok-ja.png` | the chip window's picture, the cursor on OK with nothing picked | NO DATA SELECTED / Choose a chip! | NO DATA SELECTED / チップが えらばれて いません | 56x48 (7x6 tiles) | the window's tiles, palette 10 | none | a yellow display title over plain cream words, on a flat indigo panel with a lighter frame | **(b)**: no panel without words in the pack, nor one found in the ROM; filling the panel from its palette's colors would do, if the user counts that as no new picture |
| `custom/pictures/ok-picked.png`, `ok-picked-ja.png` | the same, with chips picked | CHIP DATA TRANSMISSION / Sending chip data... | CHIP DATA TRANSMISSION / チップデータを 送信します | 56x48 | as above | none | as above | **(b)** |
| `custom/pictures/redeal.png`, `redeal-ja.png` | the chip window's picture, the cursor on the re-deal | SHUFFLE! / Shuffle Folder | SHUFFLE! / チップフォルダを シャッフル! | 56x48 | as above | none | as above | **(b)** |
| `custom/pictures/other.png`, `other-ja.png` | in the pack; the renderer draws it nowhere (battle mode 1's, no netbattle's) | BLOCKING! / Reject Disturb Icon | BLOCKING! / 1ターンのあいだ オジャマ攻撃を はねかえす | 56x48 | | | as above | **(b)**, not drawn |
| `custom/pictures/scrap.png`, `scrap-ja.png` | the chip window's picture, the cursor on the scrap | TRASH CHUTE | DUST SHOOT | 56x48 | as above | none | orange display letters with a dithered trail over a tiled green pattern | **(c)**: a logo on art |
| `custom/buttons/redeal.png` | the re-deal's slot | SHUFFLE | the same tiles | 96x24: three states of 32x24 (`sets: each`) | slot tiles | by state: selectable, unavailable, picked | dark letters filling a yellow plate with bolts; gray and red plates | **(c)**: no blank plate |
| `custom/buttons/beast_out-falzar.png`, `beast_out-gregar.png`, the third set | in the pack; not drawn (`sets: other` uses the first two, `hidden` the fourth) | BLOCK | the same tiles | 32x16 | slot tiles | | a plate filled by the word | **(c)**, not drawn |

EXE6: (a) 3, (b) 9, (c) 8.

### 11.4 EXE5

"As EXE6's" means the same look, drawing and motion as the EXE6 row.

| Asset | Where it shows | English | Japanese | Size | Drawn as, moves | Look beyond letters | Verdict |
|---|---|---|---|---|---|---|---|
| `hud/banners/*` but the Program Advance's (45 pictures; `-ja` for four navis' names) | as EXE6's (EXE5's navis: COLONEL, GYROMAN, KNIGHTMAN, MAGNETMAN, MEDDY, MEGAMAN, NAPALMMAN, NUMBERMAN, PROTOMAN, SERCHMAN, SHADOWMAN, TOADMAN, TOMAHAWKMAN; `banner-90` is TURN LIBERATE!) | as EXE6's; SERCHMAN, TOMAHAWKMANDELETED | ROCKMAN, BLUES, SEARCHMAN, TODOMAN for MEGAMAN, PROTOMAN, SERCHMAN, TOADMAN | 160x16 | as EXE6's | as EXE6's | **(b)** |
| `hud/banners/program-advance.png`, `program-advance-empty.png` | as EXE6's | PrgAdvance | | 160x16 | as EXE6's | the logo | **(c)** |
| `hud/banners/program-advance-ja.png`, `program-advance-empty-ja.png` | as EXE6's | | プログラムアドバンス | 160x16 | as EXE6's | white katakana, gray rim | **(b)** |
| `hud/pause.png` | as EXE6's | PAUSE | the same | 40x16 | as EXE6's | white, dark rim | **(b)** |
| `hud/waiting.png`, `waiting-ja.png` | as EXE6's | Cstmzing... | カスタム中… | 64x16; Japanese 56x16 | HUD layer tiles, blinks every 32 frames | two-tone cream | **(a)** |
| `hud/gauge.png` (the frame's word) | the custom gauge | CUSTOM | the same (`gauge-ja.png` is the same picture) | 32x8 | HUD layer tiles | rim and shading on the frame line | **(b)** |
| `hud/gauge.png` ("L or R") | the full gauge | L or R | the same | two frames of 32x8 | the two frames every 8 ticks | plain letters (no rings in EXE5) | **(a)** |
| `custom/turn-limit.png` | as EXE6's | FINAL TURN | the same | 56x16 | HUD layer tiles | yellow letters | **(a)** |
| `custom/window.png`: "BATTLE CHIP" | the chip window's top frame | BATTLE CHIP | the same | about 70x8 | the window's map, sliding with it | engraved | **(c)** |
| `custom/window.png`: "CHIP SELECT" | the header over the slots | CHIP SELECT | the same | about 50x8 | as above | dark letters on a strip | **(b)** |
| `custom/window.png`: OK | the OK button | OK | the same | about 24x16 | as above | filling a red button | **(c)** |
| `custom/pictures/ok.png`, `ok-picked.png`, their `-ja` | as EXE6's | NO DATA SELECTED / Choose a chip!; CHIP DATA TRANSMISSION / Sending chip data... | NO DATA SELECTED / チップが えらばれて いません; CHIP DATA TRANSMISSION / チップデータを 送信します | 56x48 each | the window's tiles, static | title and words on a flat panel | **(b)**, as EXE6's |
| `custom/pictures/redeal.png`, `redeal-ja.png` | the cursor on the re-deal (Shuffle) | SHUFFLE! / Shuffle Folder | SHUFFLE! / チップフォルダを シャッフル! | 56x48 | as above | as above | **(b)** |
| `custom/pictures/other.png`, `other-ja.png` | in the pack, drawn nowhere | BLOCKING! / Reject Disturb Icon | BLOCKING! / ゾクタイからの オジャマバットを はねかえします | 56x48 | | as above | **(b)**, not drawn |
| `custom/pictures/arm_change.png` (`arm_change-ja.png` is the same picture) | the cursor on Arm Change | ARM CHANGE | the same | 56x48 | as above | red outlined display letters over a white burst: a logo | **(c)** |
| `custom/buttons/redeal.png` | the Shuffle slot, with its uses left | SHUFFLE | the same | 96x24, three states | slot tiles by state | filling a yellow plate | **(c)** |
| `custom/buttons/soul.png`, `soul-ja.png` | the soul button (the special slot) | UNITE | uni son (two rows) | 72x16: three sets of 24x16 (`sets: other` uses the first two; the third, BLOCK, isn't drawn) | slot tiles; Soul Unison's and Chaos Unison's palettes | filling a red plate; gray when unavailable | **(c)** |
| `custom/buttons/arm_change.png` | the Arm Change slot, its held chip drawn in it (`held_at`) | ARM (one letter a row) | the same | 128x24 | slot tiles, `sets: unavailable` | letters in the slot's red frame | **(c)** |

EXE5: (a) 3, (b) 8, (c) 7.

### 11.5 EXE4

| Asset | Where it shows | English | Japanese | Size | Drawn as, moves | Look beyond letters | Verdict |
|---|---|---|---|---|---|---|---|
| `hud/banners/*` but the Program Advance's (39 pictures; `-ja` for five navis' names) | as EXE6's (EXE4's navis: AQUAMAN, FIREMAN, GUTSMAN, HEALNAVI, JUNKMAN, MEGAMAN, METALMAN, NOMLNAVI, NUMBRMAN, PROTOMAN, ROLL, SEARCHMAN, THUNDERMAN, WINDMAN, WOODMAN) | BATTLE n START!, TURN n START!, FINAL TURN!, ENEMY DELETED, YOU WIN!, YOU LOSE, DRAW!, HIT DAMAGE JUDGE, FLAG DELETED, NAVI WIN!, NAVI DELETED | ROCKMAN, BLUES, NUMBERMAN, HEEL NAVI, NORMAL NAVI for MEGAMAN, PROTOMAN, NUMBRMAN, HEALNAVI, NOMLNAVI | 160x16 | five 32x16 sprites; 9 ticks in (by 0x40 in the texture's step), 30 held, 9 out, no bounce | as EXE6's | **(b)** |
| `hud/banners/program-advance.png`, `program-advance-empty.png` | a Program Advance | ADVANCE, between two green arrows | | 160x16, about 85 pixels inked | as the banners | white letters on a blue pill between arrows: a logo | **(c)** |
| `hud/banners/program-advance-ja.png`, `program-advance-empty-ja.png` | as above | | プログラムアドバンス | 160x16 | as the banners | white katakana, gray rim | **(b)** |
| `hud/pause.png` | x 100, y 64 (`layout.pause`), in the banners' palette | PAUSE | the same | 40x16 | sprites, static | white, dark rim | **(b)** |
| `hud/waiting.png`, `waiting-ja.png` | the HUD layer at column 22, row 4 (0x08015624) | BUSY... | カスタム中… | 56x16 each (7x2 tiles; the pack's file lays the US one out 8 tiles a row) | HUD layer tiles, the banners' palette | two-tone cream | **(a)** |
| `hud/gauge.png` (the frame's word) | the custom gauge | CUSTOM | the same (`gauge-ja.png` is the same picture) | 32x8 | HUD layer tiles | rim and shading | **(b)** |
| `hud/gauge.png` ("L or R") | the full gauge | L or R | the same | two frames of 32x8 | two frames | plain letters | **(a)** |
| `custom/turn-limit.png` | the turn limit's block (`turn_limit_palette` 14) | FINAL TURN | the same | 56x16 | HUD layer tiles | yellow fading to white, a lavender rim | **(b)**: rim and fill |
| `custom/window.png`: "CHIP SELECT" | the header over the slots | CHIP SELECT | the same | about 60x8 | the window's map | dark letters on a strip | **(b)** |
| `custom/window.png`: OK | the OK button | OK | the same | about 24x16 | as above | filling a red button | **(c)** |
| `custom/pictures/ok.png`, `ok-picked.png`, their `-ja` | as EXE6's | NO DATA SELECTED / Choose a chip!; CHIP DATA TRANSMISSION / Sending chip data... | NO DATA SELECTED / チップが えらばれて いません; CHIP DATA TRANSMISSION / チップデータを 送信します | 56x48 each | the window's tiles, static | title and words on a flat panel | **(b)** |
| `custom/pictures/redeal.png`, `redeal-ja.png` | the cursor on Shuffle | SHUFFLE! / Shuffle Folder | SHUFFLE! / チップフォルダを シャッフル! | 56x48 | as above | as above | **(b)** |
| `custom/buttons/redeal.png` | the Shuffle button (its own cells, `place`) | SHUFFLE | the same | 96x24, three states of 32x24 | the window's map by state | filling a yellow plate | **(c)** |
| `custom/buttons/soul.png`, `soul-ja.png` | the UNITE button (its own cells on the window's map) | UNITE | uni son | 72x16: two sets of 24x16 and the window's fill | the window's map | filling a red plate; gray when unavailable | **(c)** |

EXE4 has no "BATTLE CHIP" on its window, no Cross window, and no "other" picture (its `other.png` is empty).

EXE4: (a) 2, (b) 8, (c) 4.

### 11.6 Languages

- **Pictures that differ by language today** (the pack's `-ja` files, which `Bundle::for_language` swaps in): in
  each game ten banners (eight with a navi's name, the Program Advance's two); "Cstmzing..." or "BUSY..."
  (カスタム中…); the OK and re-deal pictures, and EXE6's and EXE5's "other"; EXE6's scrap picture, its gauge's
  "L or R" and its Cross names; EXE5's and EXE4's soul buttons (uni son).
- **The same pictures in all four ROMs**, so English on a Japanese console too: every other banner, "PAUSE",
  "CUSTOM", "FINAL TURN", the window's labels and OK, the SHUFFLE, BLOCK and ARM buttons, "ARM CHANGE", and
  EXE5's and EXE4's "L or R".
- **What drawn text would gain**: any language the content has strings for. Today a language a pack lacks falls
  back to the pack's own pictures (§10.3: a pack of one release's ROMs shows that release's words); drawn, the
  words would follow the chosen language whatever ROMs made the pack, and a banner's words would be strings (its
  navi's name, the result's words), not one picture a banner a language. Drawing a Japanese word where the
  Japanese game shows English (BATTLE START, PAUSE) would be a translation no ROM has, which `gen-content check`
  (it compares the strings with the ROMs) could not check; the faithful Japanese strings there are the English
  words.
- **Names the 20 cells shortened** (TOMAHAWKMANDELETED in EXE6 and EXE5; EXE5's US SERCHMAN; EXE4's US NUMBRMAN,
  NOMLNAVI, HEALNAVI): the strings would hold the ROM's words; spelling them out is a change of words, the user's
  to choose.

### 11.7 Recommendation

1. **The banners first** ((b): EXE6's 43 pictures, EXE5's 45, EXE4's 39, eight of each game's with a Japanese
   picture of their own, and the Japanese Program Advance's two). Most of the way is
   built: a telop is already text on the banner block with the banners' squash (`TextItem::vscale`), and the
   banners are already drawn at the window's resolution (`smooth::Overlay`). It needs:
   - the extractor to write the bar's cells (left end, body, right end) from the ROM's set of banner letters,
     which it reads already: no new pixels;
   - one banner style in the text layer: the two-tone fill and the dark rim (an outline heavier right and down),
     in the banners' palette;
   - a strings table of the banners' words per language (`locales/<lang>.toml`, keyed by the banner's asset name,
     the navis' banners by the navi), which `gen-content check` can compare with the ROMs' letter cells as it does
     the chip names; the banner digits stay pictures at `number_at`;
   - the Program Advance: the US logos stay pictures; the Japanese katakana can take the same style, with no bar;
   - a user decision: Murecho bold, or a squarer display face nearer the banners' letters (a second bundled font).
2. **The small plain labels** ((a)): "Cstmzing...", "BUSY..." and カスタム中…, "L or R" (the rings are
   `vfont::mark`'s Ⓛ and Ⓡ), EXE6's and EXE5's "FINAL TURN". Nothing is behind them; they need strings and a role
   like `Role::Cell` in the game's face colors. "PAUSE" (and EXE4's "FINAL TURN") too, once step 1's rim exists.
3. **Leave as pixel art**: the buttons (SHUFFLE, UNITE and uni son, ARM, BLOCK), the window's OK and its engraved
   labels, the Cross window's plates and names, and the logos (PrgAdvance, ADVANCE, ARM CHANGE, TRASH CHUTE and
   DUST SHOOT). The chip window's OK and re-deal pictures only if the user accepts the frontend filling their panel
   from its palette's colors, as it fills the chip window's blank name cells, in place of a picture the ROM
   doesn't have.

## 12. The lettered pictures and the numbers in fonts (as built, 2026-10-19)

The user, in order: "can you redraw and localize some more assets to use smooth rendering? show me which ones you
plan to do before doing it"; after the plan (§11.7), "you should use a font that matches the game style more. keep the
english for those that don't differ between games", "make sure the fonts also support japanese"; after a font
audition, "ok do that. also you should replace the damage numbers with a real font too"; then "you should also redraw
the banner graphic for enhanced mode", "you should use high res rendering for CROSSSELECT and BATTLE CHIP as well",
"what about the hp numbers and the ???? trap. also the numbers sould really be fixed width. also yes do that too"
(CHIP SELECT and B:EXIT), "the number font should match the original font more", "i like bungee" (of a second font
audition for the numbers), "the ???? trap font should use the telop font", "also the emotion window counter should also
use the number font. also the chip text looks misaligned sometimes?" and "the colforce one looks wrong"; then "also
the chip letters and the text in the cross select screen", "i would prefer the same font for the cross names in en and
jp" and "yes do that use murecho" (§12.11).

All of it is the enhanced graphics' alone: the original mode draws the pictures as before, byte for byte (§12.9), and
nothing here reaches the simulation, its snapshot or `Content::hash()`.

### 12.1 The fonts

| Font | What it draws | Cut | File |
|---|---|---|---|
| Murecho | as before (§9): chip names, the telop's and the hand window's names, the chatbox, the HUD's lines, "VS", and now the HUD's "????" marks and EXE6's Cross names (§12.11) | 700, 300; 900 for the Cross names | `crates/nettai-render/fonts/murecho` |
| **M PLUS 1 Code** (OFL, Japanese too) | the lettering: the banners' words, "PAUSE", the waiting label, CUSTOM, L or R, FINAL TURN, the help pictures | 700; narrowed to 78% for the display words (the banners, PAUSE, the waiting label, FINAL TURN: the user's audition, "ok do that"); at its own width for the small labels (CUSTOM, L or R, the help pictures' titles); 500 for the help pictures' words | `crates/nettai-render/fonts/mplus1code` (moved from nettai-ui's fonts, the same Google Fonts file, which the UI's theme now imports from here) |
| **Bungee** (OFL) | the window's labels (§12.6: "pick bungee"); the chip codes until 2026-10-21 (§12.11); the numbers until 2026-10-20 | its one cut, each glyph's ink stretched to the game glyph's box | `crates/nettai-render/fonts/bungee` (Google Fonts' `ofl/bungee`) |
| **Nettai Digits** (nettai's own, MPL-2.0) | every number but the banners' (§12.4), its '+', the times sign of "x2" (`×` alone: no `x`), the '?' (from 2026-10-21; Saira 75/700 and Teko 600 before it) | the game's HP box digits traced, sharp corners, as a variable font (CFF2): its weight axis, `wght` 400 to 700, thickens the bars into the counters, set for each place from its box (§12.4) | `crates/nettai-render/fonts/nettai-digits/NettaiDigits[wght].otf`, the one file (built by bn6battle-verify's `tools/digits/build_digits_vf.py`, from the outlines `build_otf.py` traces) |
| **Nettai Banner Digits** (nettai's own, MPL-2.0) | the numbered banners' numbers (§12.4) | its one cut: the game's banner digits traced, another drawing | `crates/nettai-render/fonts/nettai-banner-digits` (the same tool) |
| **Nettai Chip Codes** (nettai's own, MPL-2.0) | the chip window's code, the big one beside the chip's picture (§12.11, from 2026-10-21) | its one cut: the game's codes A to Z and `*`, 7 by 11 pixels, traced | `crates/nettai-render/fonts/nettai-chip-codes` (bn6battle-verify's `tools/digits/build_codes_otf.py`) |
| **Nettai Slot Codes** (nettai's own, MPL-2.0) | the codes under the slots' icons (§12.11, from 2026-10-21) | its one cut: the game's small codes, 6 by 5 pixels, traced | `crates/nettai-render/fonts/nettai-slot-codes` (the same tool) |

They are compiled in (`vfont::LETTERING`, `numbers::FONT`, `numbers::DIGITS_FONT`, `numbers::BANNER_DIGITS_FONT`,
`numbers::CHIP_CODES_FONT`, `numbers::SLOT_CODES_FONT`; Bungee is 119 KB, the digit and code fonts under 4 KB each, M PLUS 1 Code 3.9 MB, which the UI's Slint bundle also embeds).
nettai's build script lists each with its license in the licenses (`crates/nettai/build.rs` `FONTS`: the OFL's
`OFL.txt`, the digit and code fonts the repository's `LICENSE`). The lettering font is `vfont::Role::Lettering`: its capitals' rows (top and
height, from the picture), its weight and its width; its glyphs are distance fields as the text font's (§9.10), cached
apart (`GlyphKey::lettering`).

### 12.2 What is drawn as text now, per game

| Picture | EXE6 | EXE5 | EXE4 | Font, look |
|---|---|---|---|---|
| Banners (every one with words: the round's and the turn's starts, FINAL TURN!, the results, HIT DAMAGE JUDGE, FLAG DELETED, the liberations', each navi's WIN! and DELETED, and the pack's unnamed ones) | yes | yes | yes | M PLUS 1 Code 78%, two-tone fill and rim, over the bar drawn as a shape (§12.3); the number in Nettai Banner Digits |
| Japanese Program Advance (プログラムアドバンス, both pictures) | yes | yes | yes | as the banners, no bar |
| US Program Advance logos (PrgAdvance, ADVANCE) | picture | picture | picture | (no words in the strings) |
| PAUSE | yes | yes | yes | M PLUS 1 Code 78%, white, slate rim |
| Cstmzing... / BUSY... / カスタム中… | yes (blinks as before) | yes | yes | M PLUS 1 Code 78%, its fill and rim |
| CUSTOM (gauge) | yes | yes | yes | its own width, light-to-blue fill on its plate (the letters taken off) |
| L or R / Ⓛ or Ⓡ (full gauge, both frames' colors) | yes | yes | yes | its own width; EXE6's US rings are the font's circled letters |
| FINAL TURN (beside the window) | yes | yes | yes | M PLUS 1 Code 78% in its rim and fill |
| BATTLE CHIP, CHIP SELECT, ▲CROSS SELECT (the tab; its triangle stays), CROSS SELECT (the Cross window's header), B:EXIT | all | BATTLE CHIP, CHIP SELECT | CHIP SELECT | Bungee, each letter fitted to the original letter's box, in the letters' colors, on their frame or strip (the letters taken off) |
| Help pictures (NO DATA SELECTED / Choose a chip!, CHIP DATA TRANSMISSION / Sending chip data..., SHUFFLE! / Shuffle Folder) | yes | yes | yes | title bold, words 500, each line where the picture's stands, on the panel (the letters taken off) |
| Numbers: HP box, enemies' and objects' HP (all colors) and damage taken, the hand window's damage +bonus x2, a telop's, the chip window's, the judge's, the banners', the emotion window's count | yes | yes | yes | Nettai Digits; the banners' Nettai Banner Digits (§12.4) |
| HUD "????" | yes | yes | yes | Murecho, the telop's cell role and colors |
| Chip codes (A to Z, `*`: the chip window's, the hand's under the slots) | yes | yes | yes | Nettai Chip Codes (the chip window's) and Nettai Slot Codes (the slots'), a glyph pixel a frame pixel across in the codes' face box, its colors (§12.11) |
| The Cross window's names (SPOUT ... CHRG, アクア ... チャージ) | yes | (no Crosses) | (no Crosses) | Murecho 900 in both languages, white or gray with the bar's dark outline, on the bar rebuilt from its colors (§12.11) |

Left pixel art, as agreed: the buttons (SHUFFLE, UNITE / uni son, ARM, BLOCK), OK, the Cross window's plates and faces,
the logos (PrgAdvance, ADVANCE, ARM CHANGE, TRASH CHUTE, DUST SHOOT), the hidden chip's "?" icon (a '?' on a framed
plate: art around it), and chip art, character sprites and backgrounds.

### 12.3 Each look is the picture's own

`crates/nettai-content/src/letters.rs` reads a picture's look off its palette indices as the pack is made, and the
pack keeps it (§12.12); the renderer's `lettering.rs` and `numbers.rs` put it in the palette the picture is drawn in.
Nothing is a game's in Rust, and no style is added to the packs (but the window's labels' places, §12.6):

- `letters(picture, ground, capitals)`: the outline is the edge color (of the letters' pixels next to the ground) that
  keeps the rest, the face, from the ground all round (a **rim**) or only right and down (a **shadow**: along at least
  half the face's right and bottom edges, so a lone pixel of a letter's own shading isn't one); the face's
  color on each of the capitals' rows (the bands of the banners: cream, pale, white, pale, cream down the letter); the
  capitals' rows, from the first letter (or all, for a set of digits); the letters' columns. The text layer draws a
  `textlayer::Ink`: the fill blended between the rows' middles at the output's resolution, and the rim as the glyphs'
  coverage grown by a disk of a frame pixel and that again half a pixel right and down, as the pictures' rims are
  heavier there (`vfont::Mask::dilated`).
- `bar_colors(picture)`: a banner's bar is the colors of its first cell, where that cell is a bar's end (every row of
  it unbroken to the bar's last pixel on the row); its letters are what stands on them. The bar itself is a drawing
  (§13), drawn with the words' squash and bounce (the banners' `vscale`), under them; the Japanese Program Advance has
  no bar.
- `lines(picture)`, `erased(picture)`: a help picture's lines (runs of rows of letters, each with its baseline, the row
  most of its letters end on, and its color) and the picture with its letters taken off, each letter pixel the ground's
  color nearest it on its row: the panel's or its frame's, by color, since the pictures' indices are bit planes, several
  of them one color. The same erasing takes the letters off CUSTOM's, L or R's and FINAL TURN's cells.
- A lettered item is painted once for what it is and where (bar, rim, shadow, fill: `vfont::Painted`, 32 kept), so a
  banner holding still costs a copy a frame. Measured roughly (fifty frames of BATTLE 1 START! at 10x, headless,
  writing the PNGs included): 1.83 s against main's 1.70 s.

### 12.4 The numbers

Each set of the game's number glyphs (`numbers::NumberSet`: the HP box's digits, the enemies' in each color, the
damage digits, a telop's, the chip window's, the banners', the '+', the "x2" pair, the emotion window's counts) is
read off its glyphs as the pack is made: the box its faces fill in their 8-pixel cells (the faces' columns of all the set's cells, their
rows), the face's colors down those rows (the damage digits' orange and yellow bands stay), and its rim or its shadow.
Every number is laid one way (`numbers::laid`; the user: "why can't the hp and the damage number in the custom
screen and the damage number in the telop all have the same number spacing?", then "why are you not just using the
font's own metrics for layout?"), in a font of the game's own digits, with the game's metrics: each glyph a game cell's
advance (8 frame pixels), its face 6 pixels wide a pixel in from the cell's left, a glyph pixel a frame pixel across;
down, each set's face box's height (`vfont::paint_fitted` draws a digit font's glyph a frame pixel a glyph pixel
across and its set's box tall). So every number has the original's spacing and box, never squeezed, and doesn't shift
as it changes (the test `digits_are_a_cell_apart`).

The design is 10 pixels tall with 2-pixel bars, and the game's sets at other heights keep their bars 2 pixels: fitted
to the enemies' 8-pixel box as it was, its bars came out 1.6 pixels while its stems stayed 2, lighter than the game's
(the user, 2026-10-21: "the numbering font looks weird when used as-is for enemy hp. you should implement opentype
variable weight and thicken the font appropriately to match the hp"). Nettai Digits is a variable font whose weight
axis (`wght`, 400 to 700) moves every horizontal edge inside a glyph's ink box into its counter, up to half a pixel (a
top or bottom bar 2 + (w - 400) / 600 pixels; the glyph's height, width and advance and its stems' width kept), and
each place is drawn at the weight that gives its bars back the game's 2 pixels: the box squeezes the design by s = the
face box's height / 10, so w = 400 + 600 x (2 / s - 2), within the axis (`numbers::Face::weight`, from the face box,
not a table of places): the enemies' HP, 8 tall, 700; the HP box and the damage digits, 10, 400; the chip window's,
11, 400, the axis's least. The run is shaped and its glyphs' outlines made at that weight (`numbers::shaped`,
`numbers::ink_box`, `vfont::number_glyph_ink`: the glyphs kept by face, weight and glyph). The static cut is gone (the
user: "get rid of NettaiDigits-Regular, that's not really use"): the variable font's default is its outlines. Looked
at: original, main's (400) and the weighted, side by side, the enemies' HP 8, 95, 600, 1000 and 9999 (EXE4's at most
1000) at 4x and 6x and the whole frame at 3x, in EXE6, EXE5 and EXE4 (match files, the custom screen open); the HP box
and the chip window draw the same as at 400 pixel for pixel. No recording shows an enemy's HP of 0: the number goes as
the navi is deleted. The run is placed where the picture's faces are
(`numbers::picture_span`): against the HP box's and the chip window's right, the damage taken's left, an enemy's HP
its middle, the judge's two numbers ending and starting where theirs do; at the output's 1x a number lands on the
original's pixels (the test `a_laid_numbers_faces_are_the_pictures`). A chip's damage follows its name ink to ink (the
name's last ink to the number's first), a quarter of the name's capitals' height apart (`hud::NAME_GAP`, 2.5 frame
pixels; the user: "fix the spacing between the chip name and the damage number"), in the hand window's label and both
telops; a telop's name and numbers are centered, ink to ink, where the original's are. A numbered banner's number is in
Nettai Banner Digits where the original's is (its faces on the picture's digits: a single digit a half cell on from a
pair's tens), and each side's words are centered on the picture's letters on that side, so both gaps are the
original's (the user, 2026-10-21: "gap should match the original"; before, the words and the number were one run with
the lettering font's word space between their inks, tighter than the original's). A chip's damage after its name runs past
where the original's ended where it is longer ("40+30×2"): nothing follows it in the hand's label or a telop. Its rim
is a frame pixel's square around it, or its shadow a frame pixel right and down.

The fonts, in the user's words (2026-10-21), after Bungee ("i like bungee", 2026-10-19), Teko 600 ("can you use teko
600 without squeezing?") and Saira 75/700 ("use saira 75/700", 2026-10-20): "could you trace a vector font of the
game's numbers?" ("just the numbers though for like damage/hp/chip damage etc"); the traces' shapes, glyph by glyph
(the 4's diagonal parallel, the 3's and 8's waists a "v notch", the "x" two crossing bars); "you should just create the
font yourself as an otf and commit it"; "the font should have sharp edges and not rounded"; "can it not just be the
same base font? and exclude the variants that don't fit" (the HP box's digits: the enemies' 8 pixels tall and the chip
window's 11 are the same design at other heights, "aren't the fonts just stretched or boldened or condensed or
whatever?", so the one font stretched to their boxes; the damage digits' 7, a pixel off, the emotion window's counts,
another drawing, and EXE4's own 7s take its shapes); "the banner i think is actually a different font? maybe those
should have variants?" (Nettai Banner Digits, a family of its own: "i think "Nettai Digits Condensed" is a misleading
name because it's a different font"; EXE4's banner digits, a third drawing, take its shapes); then "replace them
outright" (no setting: Saira is gone). The fonts are built by bn6battle-verify's `tools/digits/build_otf.py` from
EXE6's pack (EXE5's digits are the same), whose README has the tracing rules; Nettai Digits' variable font by its
`build_digits_vf.py` from those outlines.

- A set with a shadow (the damage digits, a telop's) stands beside a chip's name: its shadow is the text layer's, half
  a frame pixel right and down as the name's (the original's are both a pixel). The others' rims are a frame pixel all
  round, as their pictures'.
- The emotion window's count: its box is the counts' own with the digits taken off (the box around where the counts
  differ filled with the box's color around it; the HUD's box without a count, `count_box`, is another picture), the
  digit fitted to the box the counts' digits fill; EXE6's last two counts are other boxes and stay pictures, as does a
  box with no number. (At first the digits' place took its own most common color, which the digits make the blue of
  their strokes: the box showed a blue block and the "3" its inverse, nearly a B.)
- A numbered banner's words are drawn either side of its number (`BATTLE`, `START!`), each centered on the picture's
  letters on its side (the pack's columns for one digit and for two); the number is the banner digits' look, in its
  own color as the original's, on the original's digits.
- The '?' of the chip window's hidden damage is Nettai Digits' '?' (the HUD layer's own '?', traced); the HUD's "????"
  marks stay Murecho's (§12.1).

### 12.5 Words, languages and strings

The words are each game's text table `lettering` in `locales/{en,ja}.toml` (declared in its manifest's `text`): display
text, outside the hash, each player's own language. Keys: `banner.ASSET` (`{n}` the number), `pause`, `waiting`,
`gauge.custom`, `gauge.l-or-r`, `turn-limit`, `pictures.NAME.title` and `.words` (the help pictures, by the chip
window's picture: `ok`, `ok-picked`, a button's name), `window.NAME` (the window's labels), `crosses.FORM` (EXE6's
Cross names, by the form's key: §12.11). A picture with no words
in the table (the US Program Advance's logo) is drawn as a picture.

- Words the four ROMs of a game show alike are the English words in `ja.toml` too (BATTLE START!, PAUSE, CUSTOM, FINAL
  TURN, the window's labels, the help titles). Words that differ by language are each language's ROMs': the navis in the
  banners (EXE6: ROCKMAN, BLUES, KILLERMAN, AQUAMAN; EXE5: ROCKMAN, BLUES, SEARCHMAN, TODOMAN; EXE4: ROCKMAN, BLUES,
  NUMBERMAN, HEEL NAVI, NORMAL NAVI), プログラムアドバンス, カスタム中…, the help pictures' Japanese words, and plain
  "L or R" in Japanese EXE6.
- **A deliberate change: names the banners' 20 cells shortened are spelled out** in English: TOMAHAWKMAN DELETED (the
  ROMs' TOMAHAWKMANDELETED, EXE6 and EXE5), EXE5's SEARCHMAN (SERCHMAN), EXE4's NUMBERMAN (NUMBRMAN) and NORMALNAVI
  (NOMLNAVI). EXE4's HEALNAVI stays as the US ROM spells it (it isn't shortened). The banners' navi names are their
  own strings: the navis' display names are the ROMs' NameID names in mixed case (TmhkMan), and capitals are no
  Rust's to make (the app's rule).
- Not done: the verification workspace's content check doesn't compare these strings with the ROMs (the banners'
  letters are glyph pointers into a set of letter pictures with no text encoding; a check would need that set's
  letter order read off the ROM). They were written from the pictures, looked at in both languages.

### 12.6 The window's labels

The pack says where they are (`custom.json` `labels`, asset-formats.md: each one's name, its tiles, its letters' rows
and colors; the extractor's EXE6 BATTLE CHIP, CHIP SELECT, CROSS SELECT twice and B:EXIT, EXE5's BATTLE CHIP and CHIP
SELECT, EXE4's CHIP SELECT). Where the window's map shows those tiles the letters' pixels are taken off (each the
color nearest it on its row that isn't a letter's: the frame's or the strip's own) and the words drawn where they
stood, sliding, shaking and fading with the window, through the Cross window's opening steps. They are in Bungee
(the user, after M PLUS 1 Code at those five rows: "the font for the small window labels are incredibly ugly", then
"pick bungee but CROSS SELECT is too tall"): each letter of the words (the spaces left out, as the pictures have none)
fitted to the box of the picture's letter in its place, its columns and the rows the letters' ink takes (the letters'
own colors alone: not the strip, its shading or an outline), so each label is as tall and as wide as the original's,
where it was, the strip showing above and below it; in the letters' colors.

### 12.7 Other deliberate changes

- The help pictures' panels are filled from their own colors where the letters were (no blank panel exists in the
  pack or the ROM: §11.3); so are CUSTOM's plate, L or R's cells, FINAL TURN's and the labels' cells.
- A telop's name and its numbers follow each other a pixel apart, centered where the original's are, as the hand
  window's name and damage do: the text layer's name, narrower than the original's glyphs, was centered in their cells
  and stood apart from its numbers (the user: "the colforce one looks wrong": ColForce 30+30, on main's enhanced mode
  too). Every telop name with numbers is drawn so; the chip window's and the hand window's names were already right.
- The text layer's items gained a box offset between frame pixels (`TextItem::fine`), so the HP numbers under objects
  move with their sprites' true places (the enhanced mode's motion).

### 12.8 Looked at

Headless frames at 3x to 6x, each beside the original mode's at the same tick, in English and Japanese: banners (BATTLE
1 START!, TURN n START!, FINAL TURN!, HIT DAMAGE JUDGE, MEGAMAN WIN!, the Japanese Program Advance, and through a
content copy whose round banner was TOMAHAWKMAN DELETED, SEARCHMAN DELETED and NORMAL NAVI WIN!, through their squash
and bounce), PAUSE, the waiting labels (EXE6, EXE4's BUSY..., カスタム中…), CUSTOM, both L or R frames, FINAL TURN
(EXE6, EXE4), the window's labels (EXE6 with the Cross tab and the Cross window, EXE5, EXE4), the three help pictures
(and their Japanese), and the numbers: the HP box, an enemy's HP counting down, the hand window's 40+10, telops'
30+30 and 100×2, the chip window's damage, the judge's numbers (EXE6, EXE5, EXE4), the banners' number, the emotion
window's count, the "????" marks. Scenarios: the golden traces machgun and soundmod and the chip lab's recordings
(flow/judge-win, flow/pause, custom/cross-window-keys, pa/0x140-gigacan1/recipe1, chips/0x001-cannon/atk10, EXE5's
flow/judge-win and ko-win-netbattle, EXE4's telops/time-up-doubled-side1, custom/pause, flow/ko-cannon).

### 12.9 The original mode is unchanged

Main's frontend (ce73eda28) and this one, both `--text original`, rendered the same PNGs byte for byte with the packs
extracted again: 16 runs, about 17,700 frames, of the scenarios above, in English and Japanese (EXE6 machgun 1-1500
twice, judge-win around its custom screens and its end twice, pause, soundmod's telops, cross-window-keys, recipe1 in
Japanese, atk10; EXE5 judge-win twice and ko-win-netbattle; EXE4 time-up-doubled-side1 twice, pause, ko-cannon). The
original mode never measures a picture, never asks the strings for lettering, and draws the labels' tiles untouched.

For the chip codes and the Cross names (§12.11), main at 9deacee5f against this one, the same way: 20 runs, about
18,600 frames, 0 differing: the runs above (machgun in Japanese only) and cross-window-keys in Japanese, Gregar's
cross-window-keys in both languages, EXE4's custom/one-code and EXE5's custom/picks.

### 12.10 Left open

- The app's preload makes the names' and descriptions' glyphs ahead (§9.10); the lettering's and the numbers' are made
  the first time a picture shows them.
- EXE6's Ⓛ and Ⓡ are the lettering font's circled letters (outlines), where the original's are filled badges.
- The Japanese Program Advance's katakana are upright, where the original's lean.

### 12.11 The chip codes and the Cross names

The user: "also the chip letters and the text in the cross select screen", then, of a first build in M PLUS 1 Code
for the Japanese names and Bungee for the English, "i would prefer the same font for the cross names in en and jp"
and "yes do that use murecho".

- **Chip codes.** Wherever the window puts a code's glyph (`custom/codes.png` in the chip window, `custom/slot-codes.png`
  under the hand's slots; A to Z and `*`, not the special codes past them), its letter is taken off the cell and drawn in
  a font of the game's own codes (the user, 2026-10-21: "you should trace ABCDEFGHIJKLMNOPQRSTUVWXYZ* for the chip codes
  font like you did digits: you should make a font for both the big chip code displayed in the chip window and the little
  chip code displayed under the chip icon"; Bungee before): the chip window's in Nettai Chip Codes (7 by 11 pixels),
  the slots' in Nettai Slot Codes (6 by 5). They keep the game's metrics as the digit fonts do (§12.4: a glyph pixel 100
  units, an 8-pixel cell's advance; the chip window's ink from 50, the slots' from 100, on the baseline): the code is
  drawn in the box all the set's glyphs' faces fill, a glyph pixel a frame pixel across, at its place in the cell, and
  down, its face's top to the baseline the box's height, each glyph at its own rows (a `*` where the game's stands;
  `numbers::Face::rows_as_drawn`, `vfont::paint_fitted`), in its colors (row by row) and its rim or shadow, where it
  stood, in every palette the window gives it. A color two of a row's pixels differ in (EXE4's small Q's orange tail)
  is the row's one fill. EXE4's own drawings of a few codes (its A, R, S, V and W) take the fonts' shapes. The cell's ground is its most common color and any color filling a whole side of
  it (the slots' frame: `letters::ground_of`); the chip window's letters touch their cell's sides, so a color merely
  on the edge is no ground. The same in every language and game (EXE4's, EXE5's and EXE6's windows).
- **The Cross names.** EXE6's Cross window shows each Cross's face and name on an orange bar (`form-names-falzar`,
  `-gregar` and their `-ja`, a look under the cursor and a gray one). The words are the strings' `crosses.FORM`
  (the ROMs' short names: SPOUT, THAWK, TENGU, GRND, DUST; HEAT, ELEC, SLASH, ERASE, CHRG; アクア, トマホーク, テング,
  グランド, ダスト; ヒート, エレキ, スラッシュ, キラー, チャージ), each version's own. The letters are found on the
  picture's first look (`letters::enclosed_letters`): their outlines are the pixels of the bar's edge color off the
  bar's edge (a letter's outline may touch it, as トマホーク's and スラッシュ's do), and the letters are those and what they enclose; the first look is read since the gray
  look's face draws pixels of the outline's color. Their pixels, in the look shown, become the nearest pixel on their
  row that is neither letter nor outline (the bar's own colors: no picture added), and the name is drawn as text over
  their span and capitals' rows, its width fitted to the span, the fill the face's colors in that look (white under the
  cursor, gray otherwise) and a rim of the outline's color. The font is one setting in `custom.rs`
  (`CROSS_NAME_FONT`, `CROSS_NAME_WEIGHT`: the text font, Murecho, at 900), for both languages; the face stays the
  picture's.
- Looked at: the chip window's A and `*` and the hand's codes (EXE6 chips/0x001-cannon/atk10, EXE4
  custom/one-code), the Cross window under the cursor and not, in English and Japanese, Falzar
  (custom/cross-window-keys) and Gregar (gregar-console/custom/cross-window-keys). In the code fonts, original and
  enhanced side by side at 4x, the chip window's code and the slots' (each on the original's pixels, the same box):
  EXE6's `*` (chips/0x001-cannon/atk10, frame 232), A and Q (gregar-console/chips/0x130-colforce/hit, 2312 and 2320);
  EXE5's Q (chips/0x070-recov150/miss, 289) and `*` (chips/0x05c-dublcrak/miss, 289); EXE4's Q
  (chips/0x099-rollaro3/miss, 267) and `*` (chips/0x094-copydmg/miss, 267).

### 12.12 Read as the pack is made

The user: "can you statically extract all assets and avoid detection at runtime?" (after the Cross names were found
at run time, and the Japanese トマホーク and スラッシュ quietly stayed pictures where their last letter's outline touched
the bar's edge).

- **Extraction reads every picture once** (`nettai_content::letters::read`, from `nettai-extract` after the graphics
  are made, in each language the pack has): the number sets (the HP box's, the enemies' three, the damage digits, '+'
  and "x2", the banners', a game's own telops', the chip window's), the emotion window's counts and their blank box,
  "PAUSE", the waiting label, the gauge's CUSTOM and both frames of L or R, FINAL TURN, the banners (their bars, their
  letters, their letters' columns either side of the number as one digit and as two), the chip codes of both sets, the
  window's labels, the chip window's lettered pictures (OK's, each button's, each version's own) and the form list
  names (each look, by its first look's letters). The pack keeps what it found (`nettai_assets::letters`, `letters` in
  `hud.json` and `custom.json`, asset-formats.md) and each lettered picture as its base with the letters off and a layer
  of its letters (§12.13). A frontend puts the look in the palette of the moment and the base's tiles in place; it reads
  no picture.
- **What isn't found is said.** Extraction writes a line to `extraction.txt` for anything every game has that it
  can't find (a number set with no face, a code glyph with no letter, a form list name whose letters aren't enclosed,
  a label whose maps lay it otherwise, a button whose picture's lines differ in its other palettes). The content audit
  (`--audit-content`, `lookups::pack_letters`, `banner_letters`, `form_name_letters`) says every picture the strings
  have words for and the pack has no letters of, and a lettered picture whose lines are fewer or more than its words'.
  At run time a missing look is a problem of the frame (`lookups::letters`) and the picture is drawn as it is: right for
  a pack without the words, and no current ROM set makes one.
- The frames are what they were: main's renderer (f73148761, reading the pictures as it drew) and this one, on the
  same packs extracted again, drew the same PNGs byte for byte at twice the frame, in the enhanced mode and in the
  original: 27 runs each, about 22,400 frames, 0 differing (EXE6 machgun 1-1500, judge-win around its custom screens
  and its end, pause, cross-window-keys of both versions and take-back-cross, each in both languages; soundmod's
  telops, recipe1 in Japanese; atk10 in both; EXE5 judge-win and picks in both, ko-win-netbattle; EXE4
  time-up-doubled-side1 in both, custom/pause, flow/ko-cannon, custom/one-code in both). They show the banners (BATTLE
  START!, TURN n START!, FINAL TURN!, the judge's and the results'), the labels, the help pictures, the gauge's CUSTOM
  and L or R, the waiting label, FINAL TURN, PAUSE, the HP box and the enemies' numbers, the emotion window's count,
  the hand line's and the telops' numbers, the chip window's code and damage, the slots' codes and the Cross names.
- **Lines a mark joins** (the user: "yes", to working out EXE4's Japanese OK picture). Its words チップが / えらばれて /
  いません touch: えらばれて's voiced mark reaches the line above, and no clear row is left between them. A picture's
  lines are its runs of inked rows, but where a run's next one starts two of the picture's line pitches or more after it
  (the pitch: the most common step from a run's first row to the next's, with three runs or more), the run is that
  many lines at that pitch (`letters::line_rows`): rows 19-36 are 19-28 and 29-36, a pitch of 10 to いません at 39. Read
  again, every other help picture of the three games' packs, in both languages, has the same lines as before (the
  packs' `letters` are identical but this picture's); the EXE4 audit has 0 problems.
- **Colors stay palette indices** (decided 2026-10-19: do not redo). A look's colors (its face down the rows, its rim,
  its shadow, a bar's shapes) are indices into the palette the original draws that picture with at that moment: the
  custom window's live palette (a code's, a Cross name's under the cursor or used), the gauge's, the banners', the
  emotion window's as its fade record leaves it. That is the lookup the original mode does for the tiles, not sampling
  or guessing, and it keeps the cursor's flashes, the used looks and the fades exact. Resolving them to RGB per state at
  extraction would mean listing every palette state a picture is drawn in and fading RGB again, a second palette
  system that disagrees with the original wherever a state is missed. The content audit checks each look's indices
  instead: every index it names is a palette entry, its fill isn't index 0 (transparent), and its rim isn't its fill.
- **Words a font can't draw** leave a picture as it is, and that is never silent. The content audit checks every
  lettering string against the font it is drawn in (`lookups::drawn_in`: the labels' and the gauge's word in the numbers'
  font, the Cross names in the text font, the rest in the lettering font), and a frame that keeps a picture for a
  missing character, or for a help picture whose lines are fewer or more than its words', says so
  (`TextSink::lettering_in`). EXE4's, EXE5's and EXE6's Japanese "チップフォルダを / シャッフル！" stayed pixels for its
  full-width '!' (M PLUS 1 Code has none); the strings now say "シャッフル!". The battle-mode-1 picture's words
  (`pictures.other`: BLOCKING!) are in the strings of EXE5 and EXE6 too, so the audit covers them.
- **Measured once.** A label's ink extent (its run stretched to its letters' columns) is measured the first frame it
  shows and kept (`TextRenderer::ink_extent`); the fonts' kana boxes and the numbers' glyphs' ink were already measured
  once a font. They stay measured, not constants: the text font can be the player's own.

### 12.13 Outlines, layout and the pack's pictures (2026-10-19, later)

The user, in order: "the redeal/ok/other blanks look really janky in exe6, is that correct?"; "i think the blanks in en
and ja should be the same anyway"; "the scrap/beast button shouldn't have blanks"; "instead of having the composed and
blanked templates, can you instead just mask the original lettering out in the content then compose it in the
engine?"; "the custom box fonts like BATTLE CHIP and CROSSSELECT are forcibly packed into the grid which makes it look
really bad, instead of laid out naturally. could you fix that somehow?", then "BATTLE CHIP and CHIP SELECT and CROSS
SELECT should be stretched to fill the frame"; "the cross select crosses don't have drop shadows on the names, only
outlines", "same on the banner, there's no drop shadow on it", "same for Cstmzing...", "and PAUSE", FINAL TURN, and "i
think there's a lot of drop shadows on hud text where they shouldn't be"; "the text above the custom gauge CUSTOM looks
really bad too"; "you should bounding box them to the longest one (e.g. tomahawk in japanese) and center align them in
the same spot on each label, so you don't force the original metrics"; and of the numbers: one scale a set, no glyph
stretched to its box, then "chip code and chip window damage should probably be the same size?".

- **Outlines as the originals'.** Extraction reads whether a lettering's rim is heavier right and down than up and
  left (`letters::outline_heavier`: the outline's mean thickness past the face's edges, right and down over left and
  up by more than half a pixel) and keeps it (`heavier`); every picture of the three games reads even, a pixel all
  round: banners (BATTLE START!, the results, the judge, FINAL TURN!), PAUSE, the waiting label, FINAL TURN, the Cross
  names. The text layer draws a lettering's rim even, heavier only where the pack says. What has a one-sided shadow
  in its pixels keeps a shadow: the 8x16 font (chip names, telop names, the enemy names, "????": its shade is right and
  down), the damage digits (the hand line, telops, the judge). The rest have no edge and get none: the HP box's
  digits, the emotion window's count, the window's labels (B:EXIT too: the user, "B:EXIT in the original did not
  have a drop shadow"; one shading pixel in its B had read as one, as one in EXE4's slot code Q), CUSTOM, L or R, the
  codes, the help pictures, the chatbox (whose font has no shade).
- **Labels and CUSTOM as text.** The window's labels are one run of Bungee at its own spacing (LetteringFont::Numbers),
  its capitals the letters' ink height, the whole run stretched or narrowed by one factor to put its ink from the
  original's first letter's ink to its last's (TextRenderer::ink_extent). CUSTOM is the same run at the font's own
  width, narrowed only if it overruns, standing as the original's letters stand in their cells (`align`).
- **Cross names in one box.** Every form list name is centered in the columns all of them take, every version's and
  language's together (`form_name_box`: EXE6's 31 to 69), at Murecho 900's own width, narrowed only where wider.
- **Numbers at a fixed pitch.** Each set's glyphs are drawn at one scale (its glyphs' ink height the face box's), at
  their own widths, narrowed alike only where the set's widest would overrun the box, each centered in its box: a 1
  stands thin, the pitch is the cell's. The signs too. The chip window's code and damage are one family, narrowed
  alike (`Fitted::family`); no other sets share a line and a height.
- **One picture, one base.** The pack keeps a lettered picture as a base all its languages share and a layer each
  (asset-formats.md, "Lettered pictures"; `letters::shared_base`). The Japanese blanks that looked janky were the
  Japanese pictures' blanks written for viewing in the English pictures' palettes: the chip window's pictures are bit
  planes, the palette picks the plane, so the other words showed. The data was right (the frames were); with no blank
  files now each layer is in its own picture's palette. Where two languages' letters cross on a pixel no one index
  shows as both blanks, the second language shows the base's pixel (extraction notes how many: EXE4's Japanese OK 1
  and OK-picked 2, EXE5's and EXE6's Japanese OK-picked 6, EXE6's Japanese form list names 460 and 352).
- **Shared grounds, recovered** (later the same day; the user: "ok/other/redeal actually share the same background",
  "for the cross bar, you should mask out the cross portrait then superimpose it over the yellow bar", "falzar and
  gregar should have the same bar"). The chip window's panels with words, every picture and language, share one
  panel; the Cross names of both versions lie on one bar, each with its portrait as a layer of its own; a picture's or
  name's letters are what differs from those, so its layer holds its letters and their outline and nothing of the
  ground. Both grounds are what most pictures show where their letters aren't (asset-formats.md, "Lettered
  pictures"), never one picture against one other: EXE4's panel had taken a pixel of its Japanese OK's taller body at
  its right edge, a tie of one picture each, now broken by every picture whose letters miss it. The crossing pixels
  above are gone with them. A window label's letters are every pixel of its strip off the row's ground, so EXE6's
  CHIP SELECT loses the lighter dot in its P that had been smeared into the strip.
- **One kind of pixel a layer** (the user, of the pack's files: "why do the extracted masks have shuffle
  underimposed?", "it doesn't seem like it's a pure text extraction"). A panel picture's layer had been every pixel
  where it differs from the shared panel by index, with a mask of its letters: the bit planes' other words in the
  panel's own colors showed as ghosts. A picture is now its base and its letters, one layer, no masks. The bit
  planes' pixels take the shared panel's index where every palette the picture is drawn in shows the two alike (all
  of them: "can you not remove them?"); a name's few bar pixels that differ from the shared bar, and a panel
  picture's own panel where it departs from the shared one, are in its letters ("fold into letters", "fold them into
  the letters layer"). The Original frames are the same; the pictures compose to the ROM's colors, not all its
  indices.
- **Not lettered.** A chip window picture is a lettered panel only where its panel's and frame's colors are two thirds
  of it or more: the buttons' art with their names over it (EXE6's TRASH CHUTE / DUST SHOOT and Beast Out, EXE5's soul
  and Arm Change) have no letters and no blank; no strings had words for them.

## 13. The frames as shapes (as built, 2026-10-19)

The user: "can you redraw more of the game ui in vector?", then, in order: "the svg looks a little messy because it's
not really getting the shapes right, it's tracing the pixels too closely"; "the svgs should be extracted at
extraction time, they shouldn't be runtime traced"; "maybe instead of extracting these vector assets you should just
straight up redraw them by hand and include them in the repo"; "when redrawing the custom window/custom bar etc you
should make the shapes actually look like the shapes instead of like matching the pixels"; "i really want the entire
HUD in vector so it can be scaled"; "no need to vectorize trash chute and beast out actually"; "small icons should
also not be vector"; and, of the details, "the whole orb holder is all circular", "there's a lot more rounding of
corners on the custom screen than you think", "i don't think you should round the corners on anything that needs to
hold a chip icon", "the chip stack has a shadow under it". The enhanced mode draws the HUD's and the custom screen's
frames, buttons and panels as filled shapes at the output's resolution; the original mode draws the pictures as
before, byte for byte (§13.6).

### 13.1 Drawn by hand, checked as the pack is made

Each frame is an SVG drawn by hand, as the art means it (a round holder, a rounded corner, one clean outline where
the pixels stack two, a shadow under what casts it), in frame pixels, in any SVG a drawing needs (shapes, paths
with curves, strokes of an even width, groups, transforms, `use`, clip paths: usvg reads it, resvg draws it). Each
shape names its palette entry (`data-color` for its fill, `data-stroke` for its outline, `data-palette` for the row,
on it or a group round it), never a color: the drawing is filled in the palette the picture is drawn in at that moment, so every palette effect (a
fade, a dimming, a flash, a version's colors) reaches it as it reaches the picture. The files live in each game's content,
`<game>/extract/shapes/` (`hud/`, `custom/`, `banners/`), a game's own or a `.same` naming by its path in the bundle
one exelib's `extract/shapes/` holds where the art is the same in two games or more (EXE5's and EXE4's HP box, gauge
cells, emotion frame, banner bars, help panel; EXE5's column cells, cursor, code plate, name bar); the extractor reads
them from the bundle (docs/design/extraction-recipes.md §2.3). Extraction composes each picture
from the ROM (a window map's cells, a set of tiles, a banner's bar with its letters off), reads the drawing, checks
its size and that every color it names is one the picture uses, and notes the share of the picture's pixels it gives
at their centers (words' boxes aside), calling out any under 85%: it is a check that the drawing is of that picture,
not a target (the drawings are 91 to 100%, the gauge's full shimmer in EXE6 72 to 78% by design). It is the one input
of the extractor's that isn't a ROM (asset-formats.md, "Frames as shapes"). `cargo run -p nettai-extract --example
shapes_check -- <game> <pack> [out] [--recolor]` checks them against a pack without extracting it again, writes each
picture beside its drawing with `out`, and with `--recolor` sets the files' viewing fills.

### 13.2 What a pack holds and how it is drawn

The pack keeps each drawing as its SVG (`shapes/`, asset-formats.md) and the JSON what SVG can't say: which cells of a
map each drawing stands in for (the custom window's maps, the Cross window's, the gauge's frame, the chatbox's boxes:
`nettai_assets::shapes::MapDrawing`, each cell drawn by the first drawing, the map's own then the others most like
it, that draws there and was drawn for the same pixels), and which of a set each is. The renderer (`shapes.rs`) turns a
drawing into a text-layer item (`textlayer::Drawn`): its pieces (a drawing each, flipped as its tiles are laid, cut
where a gauge's fill ends), each from its raster (`rasters.rs`): every drawing of the packs is read once and rasterized
by resvg at the output's scale before the first picture (a preload's), keeping each palette entry's share of every
pixel, so a frame colors it in the palette's colors of the moment, after the fades, without drawing the file again,
and keeps what it colored while the colors hold; a drawing is laid at whole output pixels (a sprite's part of a pixel,
at most half an output pixel, isn't shown); when the output's scale changes the drawings are rasterized again away
from the frames, the old rasters resampled until then, and a banner's squash is the raster resampled. Nothing is read
or rasterized while the battle plays (`nettai-tool --time` counts it). The text layer's items are kept the same
way: each painted once for what it is (its words, font, size, style and colors before the fades, at the output's
scale), unplaced, laid at whole output pixels and faded as it is laid; drawings and text share one cache, the least
recently used let go first; the cells it may draw in (the tiles it stands in for aren't drawn); and
a mask of the pixels it keeps off (`Drawn::off`: the Cross names' portraits stay the picture's, over the vector bar).
Where something must stay hidden behind a frame (the chatbox's box hides the navi's HP), the tiles are still drawn
under the shapes, but only where the shapes cover a pixel wholly (`shapes::coverage`), so the box keeps its place in
front without pixel corners showing past its curves and cuts (a pixel an edge crosses shows the shapes' own smooth edge
over what is behind; with centers as the test, the art's one-pixel step showed as a notch under the name tab's light
cut, the user: "why does it look like there's such a weird cut out here?"). Overlays drawn in front of the window (the spinning emblem) stay in front.
A drawing may carry words (`Drawing::words`, its `text` elements with `data-words`, left out of what resvg draws): the
game's string for a key in a box, drawn by the text layer over it in the numbers' font (or the lettering font,
`data-font`), fitted to the box, a line a share of its height, with an outline of a given width and a slant where the
art has them (OK, SHUFFLE, UNITE / UNI SON, BLOCK, A R M, CHIP SELECT, the Program Advance logos' words).

What stands in for what:

- **HUD**: the HP box, the gauge's frame (its word as text, §12) and cells (the fill cut where it ends, so it fills
  smoothly), its full shimmer's frames, the L or R cells; the emotion window's frame (its ring a drawing; the faces kept
  without it in the pack, `MugshotFrame`), the count box and the link navi's box; the banners' bars (with their
  squash); the Program Advance logos; the chatbox's boxes, open and opening, and its key-wait arrow's three frames.
  The boxes and their name tabs are chamfered, not stepped as the art: the outline's cuts as light as the art's (legs
  through its stairs' middles: 2.5 pixels in EXE6, 1.5 on its opening box's left and in EXE5 and EXE4), every layer
  inside (band, ground) a parallel offset of them, so a band is as thick through a cut as along its edges and an inner
  cut is smaller, square where its layer lies further in than the cut reaches; the box's own corner under a tab square,
  the tab's being the corner there (the user: "dialog box corners still look weird, why do they look stair stepped
  instead of chamfered?", then, of a 4-pixel cut, "the corners are chamfered waayyy to much on the dialog box now,
  the original just have a light chamfer").
- **Custom screen**: the window's frame for each map (EXE6's with the Cross tab and without, the Cross window and its
  opening frames; EXE5's; EXE4's), the column's cells, the cursor and the Cross list's cursor, the Regular chip's
  frame, the element icons' frame (rounded, its symbols drawn over it as their pictures: below), the empty slots' boxes
  (square, as every box a chip's icon goes in) and the slots' code plates (rounded, the empty ones' too), the enemy
  names' bar, the Cross names' bar, OK (in the window's drawing), the buttons by set (`button-NAME-K`: SHUFFLE in all
  three games, EXE5's soul and arm change buttons, EXE4's soul button), and the chip window's panel for OK and the
  buttons (`picture-NAME`, a language's own as `NAME-LANG`).

### 13.3 Left as pictures

By the user's word: TRASH CHUTE / DUST SHOOT and the Beast Out button (art and words), and the small icons (the
element icons' symbols, the hidden chip's '?', the gauge's warning arrow; the chatbox's arrow at first, drawn since at
the user's word, "the chatbox arrow should also be vectorized"). The element icons' frame is a drawing, its symbols
pictures over it (the user: "can you also redraw the element icon frame? then you can pull the actual element images
out of the frame and render them on top. the element icon frame actually has rounded edges and is not chamfered"):
the frame round, a light ring a pixel wide, its outer and inner edges round the same middle (radii 2.5 and 1.5, the
art stepping the outer corner a pixel and shading the inner one half), the dark ground inside it and nothing round it
(the user: "don't draw the square background under the element icon since it just looks awkward, only the icon frame
should be filled in": the app shows its own background there, the battle the window's panel, EXE6's and EXE5's chip
window laying the frame's ground there as the panel round it); extraction takes the
icons apart (the frame each pixel's index most icons share, the ground the frame's most common; a symbol its icon's
pixels on the ground that aren't the ground's); then (the user: "you should draw the element icons in the ui with
the smooth frame as well and make the extract element icons transparent and borderless") the pack holds the icons
once, in `elements.png`, each its symbol alone, transparent and borderless, beside the frame as a picture
(`element-frame.png`), an icon keeping the frame's pixels it draws its own way (EXE6's and EXE5's fourth icon's inner
corners, their no-element icon's top right), so the original mode's frame with the icon over it is the game's icon,
byte for byte. The enhanced mode lays the icon's tiles in the chip window (EXE4: the element sprite's) with the
frame's drawing under it, kept off the symbol's pixels inside the ring (`Drawn::off`, `CustomScreen::element_inside`),
so an icon's own pixels of the frame stay under the smooth ring; the app's chip lists and details draw the same: the
frame's drawing in the family's colors as an SVG image, smooth at the window's scale, the symbol's pixels over it
square-edged (`pictures::element_icon_svg`). Pictures rather than frames:
chip art and chip icons, faces and the emotion window's per-face boxes (EXE5's and EXE4's patterned boxes are each
face's own art), portraits, the navis' emblems (sprites), EXE4's orb, the sprite at the EXE6 window's bottom right,
and the soul and arm change pictures in the chip window. The Program Advance logos (`BannerLayout::logo`: a banner
with no bar its words stand on) are drawings whose words are text, by the user's word ("the program advance logo
should also be vectorized, but the text should use a font. it should also be italicized"): PrgAdvance on its arrow
(EXE6's and EXE5's) and EXE4's ADVANCE between its arrowheads, their shapes drawn and their words in italic; the
Japanese one (プログラムアドバンス, the same picture in the three games) only its words, white, italic, each letter
outlined two pixels wide in the picture's dark ("the japanese program advance text is actually outlined and not on a
black box like that").

### 13.4 The window's cells, audited

A window drawing stands in for a cell only where the screen lays that cell as the map does; anywhere else the cell
is drawn as its tiles and the renderer notes a problem. The trace audit (`nettai-tool --audit --draw`, enhanced) over
the custom-screen labs of the three games and the replays finds none (§13.6 lists what was run).

### 13.5 Decisions

- The window's OK is part of its map and so of its drawing; the extractor no longer leaves its cells out.
- A help picture's ground is one drawing for all its pictures and both languages (their blanks are the same).
- The Cross names' bar is drawn under the portraits, kept off their pixels, rather than cut to whole cells.
- EXE4's chatbox, gauge frame, column cells, cursor, empty box and code plate are their own drawings (other indices);
  EXE5's and EXE4's chatboxes have a name tab at the left.
- The drawings are read and drawn by usvg and resvg 0.48.1, the version Slint builds into the app, not by a parser and
  filler of our own (the user: "is there no lightweight svg render you could use? it seems crazy to have your own svg
  parser and renderer"); extraction's check draws a file at its pixels' centers (each pixel whole) to compare.
- EXE4's custom window is drawn as raised pieces casting drop shadows (the user: "this should be a drop shadow over
  the header bar thing", "this should really just be a shadow"): the plate and the column's housing one piece, its
  shadow down its left and under the plate, turning under the rail (gray on the white ground, dark blue on the bars);
  the white panel's shadow along the bottom section's bars, stepping as its edge does.

### 13.6 Checked

- The original mode, main's frontend against this one (both `--text original`, each on packs its own extractor made
  from the same ROMs, `tools/frontend-compare/identity.sh`): 33 runs, 140,725 frames, 0 differing (EXE6 machgun,
  judge-win, pause, cross-window-keys of both versions, take-back-cross, redeal-right-half, run-message,
  description-keys, soundmod, cannon atk10, and in Japanese machgun, judge-win, cross-window-keys, redeal-right-half,
  gigacan1's recipe; EXE5 judge-win, picks, arm-change, run-message, ko-win-netbattle, and the first three in Japanese;
  EXE4 time-up-doubled-side1, pause, ko-cannon, one-code, describe, run-message, and three in Japanese); after the logos
  and main merged, 14 runs more (the Program Advance recipes of the three games, the buttons, the chatboxes, in both
  languages), 22,356 frames, 0 differing.
- The trace audit, enhanced, drawing every frame: the custom-screen and flow labs of the three games and EXE6's
  replays (371 traces, about 958,000 frames): no problem of the drawings (the window's cells never laid otherwise).
- Looked at, original beside enhanced, at 3x to 24x: EXE6's custom window (every corner, the housing, the socket, the
  stack's bottom and shadow, the glare, the tab), the cursors, the slots and plates, the Cross window and its bars,
  OK and SHUFFLE in their three looks, the help panel, the chatbox (message box, its corners), the Program Advance
  logo; EXE5's custom window, UNITE, the chatbox; EXE4's custom window and the description chatbox; the gauge's and the
  emotion window's drawings of EXE5 and EXE4 against their pictures.

## 14. The Legacy Collection's words, Chinese, and one font (as built, 2026-10-25)

The user: "can you extract simplified and traditional chinese translations for content from bnlc vol2 and use
those? and also extract english and japanese and if there are differences use the bnlc translations. also add
simplified chinese and traditional chinese locales for nettai with full translations", and along the way "can you
make it generate directly from bnlc instead? instead of from the rom", "you should use one unified font for all
languages so it's not inconsistent" (then Noto Sans CJK, from a specimen of candidates), "just use noto sans cjk
without subsetting it", "don't force anything into any fixed cells like that" and "i don't think english and
japanese should do that either".

### 14.1 Where the words come from

Mega Man Battle Network Legacy Collection Vol. 2 runs the Japanese ROMs and draws its own text in English,
Japanese, Simplified and Traditional Chinese from message packs keyed by the Japanese ROMs' text archives, and
its card reader's texts (the patch cards', EXE4's e-Reader chips'). bn6battle-verify's tools/bnlc reads them
(bnlc.py: the formats) and places every table string in them (locales.py `places`, places/GAME.tsv: by the chip's
number, by the table's string, near matches where the original wrote a button the collection draws as an icon,
the area menu for backgrounds, the card reader). The game's strings of all four tables are the collection's
(`apply`; `check` holds them to it): EXE5's gen_content.py and EXE4's chip_locales.py write the tables from the
ROMs and then take the collection's in (`from_collection`); EXE6's gen-content no longer compares the strings with
the ROMs, only that their fonts draw them.

- Where the collection's English or Japanese differs from the ROMs', the tables now have it: EXE6 AquaMan for
  SpoutMan (the navi, its chips, AquaCross), TmhawkCross, Z Saber, HeatPres's and AirRaid's descriptions, the patch
  cards' official names and effect lines; EXE4's patch cards' official names (MAX HP+100); and others, about 450
  strings in all.
- English: the collection's dashes are hyphens and its full-width comma a comma and a space ("Lan, this is no
  time"). A no-running message's characters per line time its chatbox (§10.3), so those messages are a character
  longer than the US ROM's: a deliberate difference from the US games (gen-content's navi check counts the
  content's string where it isn't the ROM's), and one of the content's hash.
- The collection's button icons are Ⓐ Ⓑ Ⓛ where the language's GBA font has the marks (EXE6's US fonts, and the
  Chinese, which the vector font draws), else the letters ("サイトをAでとめて": the Japanese fonts and EXE4's and
  EXE5's US fonts have no marks); the D-pad as the original's words (Left, 左, いどう; 左, 方向键). The game's large
  cross (✕) is ×, which every font has.
- Not taken: the collection's stand-ins (ミテイ, a gift's text the save keeps), what it leaves empty, and the English
  and Japanese of a place outside its field's archives (a JP-only chip named in a dialogue).

### 14.2 The Chinese tables and the app

`locales/zh-Hans.toml` and `zh-Hant.toml` of each game: the collection's Chinese for every game string, and the
content's own strings (rulesets, battle themes, emotions, the bugs EXE4 lacks, the DS's party programs and chips,
the content's patch card effect lines) translated by hand in its terms. The lettered pictures' words follow the
collection's Chinese art: the banners stay English with the Japanese games' names (ROCKMAN, BLUES), as its
telops are; "Cstmzing..." is 改装中… (改裝中…); the Cross window's names are the Crosses' Chinese names (高热交错);
the chip window's help pictures keep their English titles over Chinese words. No pack has Chinese pictures or a
Chinese pixel font (no GBA ROM has either), so a Chinese battle's pictures are its pack's own language's and its
strings are drawn by the vector font in the enhanced mode and by pixel fonts in the original mode (§15).

The app's catalogs (crates/nettai-ui/lang/zh-Hans, zh-Hant) have every string, in the collection's terms (芯片/晶片,
文件夹/資料夾, 领航员/領航員, 个人化装置/個人化裝置); docs/app.md §3.

### 14.3 One font

Every language's dynamic text, in the battle and in the app's body, is Noto Sans CJK: Google Fonts' variable Noto
Sans JP (English and Japanese), SC and TC, whole (crates/nettai-render/fonts/noto-sans-cjk). `vfont::Cut::of`
gives a language's cut; the lettering (the banners' words, the labels) is the same cut, bold and narrowed, in
place of M PLUS 1 Code, which has no Chinese; Murecho is gone. The program holds the files once: the renderer
embeds them and the app registers the same bytes with Slint's shared font collection (`lang::register_fonts`).
The tests of the old font's metrics went with it.

### 14.4 Boxes as wide as their text

A chip's name, the other player's chip's telop, the custom screen's enemy names and the HUD's seconds take, on the
text layer, as many cells as their text at its own width (`fonts::box_cells`), in every language: nothing is
narrowed into the original's glyphs' cells (§9.4's rule stays for the boxes the original fixes: the chatbox's). The
enemy names are right-aligned, ending at column 30 as the original's do.

## 15. The original mode's pixel fonts (as built, 2026-10-25)

The user: "for chinese in original mode, is it possible to use a pixel font instead?", then from specimens of
candidates drawn into battle frames "Fusion Pixel 12px looks good for the dialog box font, but i think the telop
and enemy name and chip name font needs to be different and thicker", "is there a font that is like less ugly" and
"Fusion Pixel Bold 12px is perfect"; then, of what it drew, "telop for double beast doesn't seem to be showing up in
chinese", "there appears to be misalignment on the enemy name" and "why are the numbers so small in fusion pixel
font?".

### 15.1 What draws what

Where a pack can't spell the language shown (Chinese, which no GBA ROM has a font for; a pack of one region shown
in the other's language), the original mode lays the strings out as the enhanced mode does (a `TextSink` that
takes and measures them with the pixel fonts) and draws them into the 240x160 frame (`pixeltext`, at the end of
`Renderer::render`), in place of the vector font shrunk to the frame's pixels that it drew before:

- **Fusion Pixel Bold 12px** for the 8x16 font's strings (chip names, the enemy's name, the telop, the Program
  Advance's names, the HUD's seconds) and **Fusion Pixel 12px** for the chatbox's: TakWolf's fonts, release
  2026.10.25, whole, in three cuts by `Cut::of` (ja for English and Japanese, zh_hans, zh_tw), under the OFL with
  the notices of the fonts they are stitched from (fonts/fusion-pixel, fonts/fusion-bold-pixel; the About screen
  lists both). About 30 MB of the program.
- A glyph's outline lies on a grid of 100 units a pixel, so at 12 pixels an em each of its pixels is a frame's
  (swash rasterizes it; coverage of half or more is ink).
- In the item's face color, with the 8x16 font's shadow (right, below and both) in its shadow color, faded as the
  item is, hidden where something in front won the pixel (the depth key), cut by its clip; the chatbox printing
  shows its first characters; the telop's squash maps the box's rows as the original's sprite rows step
  (`compose`'s formula).
- The hanzi's ink ends on the row the role's capitals end on: row 13 of the 8x16 font's 16, so a name sits in its
  bar (rows 3 to 13) as the original's letters do; row 10 of the chatbox's 12. (Centered on the capitals, the
  first build's hung a row under the enemy name's bar.)
- What the pack's own font has of a string's plain characters (ASCII's digits, Latin letters and signs) is the
  pack's glyph, as tall as the original's: the pixel fonts' are half-width and some 7 rows, the original's 10
  ("攻击+10" ends in the game's "+10"). The game's marks (the stacked EX, SP, DS: the Private Use Area, §10.5) are
  the pack's glyphs always; the fonts' own glyphs there are other things.
- A string is never narrowed: its box is as wide as its text (§14.4). The chip window's name has the original's
  eight cells, which its clip keeps: five hanzi fit (13 pixels each, the last column blank), a longer name is cut
  at the box (MEGA加农炮, 小小热水壶1: 22 of EXE6's 354 chip names, 22 of EXE4's, 28 of EXE5's). The user chose that
  from four mocked in real frames (cut, a 10px font, squeezed to the box, run past it): "keep cutting".

The telop's text rides on blank sprite parts (so it is as deep as they are, and the sprite budget counts them):
they were one a glyph of the pack's for the name, so a name of which the pack has no glyph (双重野兽, DblBeast's)
had none and wasn't drawn, in either mode. They are now one a cell of the text's box.

### 15.2 Choosing

Three rounds of specimens, real EXE6 frames in the original mode with each font drawn in at the frame's pixels:
Fusion Pixel 12px and 10px, Ark Pixel 12px (lacks 35 Simplified and 77 Traditional characters), Cubic 11,
Boutique Bitmap 9x9 and GNU Unifont 16px; then those strokes doubled; then Fusion Pixel's bold build, outlined
variants, ChillBitmap 16px doubled, ZLabs Pixel 12px and MuzaiPixel 8x12. Left out: Zpix (its license costs money
and forbids changes), Ark Pixel 10px (almost no hanzi), and the collection's own Chinese 8x16 font (smooth
high-resolution lettering, not pixel art).

### 15.3 Checked

The renderer's tests (pixeltext's: a string drawn into the frame in its colors round its box; boxes as wide as the
text, the bold the wider; a mark from the pack; depth, clip and printing), the About screen's; headless frames of
EXE6 in zh-Hans and zh-Hant: the custom screen with a chip's description, the chip in hand with its damage, the
other player's telop (Atk+10), DblBeast's telop, the enemy name on its bar, and chip names too long for the chip
window. Packs with the language shown never reach it: the frame comparisons are unchanged.
