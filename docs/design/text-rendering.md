# Text rendering: the extracted pixel fonts, and font files

An investigation, not a built feature. The question: should the frontend draw its text (chip names, descriptions,
telops and the rest) with a real font file rasterized at run time, instead of the pixel fonts extracted from the
user's ROM? This document says what the frontend draws today and how, what "real font rendering" can mean here,
what it would cost, how it sits beside the pixel-for-pixel comparison with the original, and what I recommend.

Nothing in the frontend was changed for it. The measurements in §8 were throwaway spikes outside the repository.

Related: [frontend.md](../frontend.md) (what is drawn and how it is verified), [asset-formats.md](asset-formats.md)
§4 (the HUD's files), [content-model-v2.md](content-model-v2.md) §3.1 and §6 (names and descriptions in the
definitions, `compat/text.toml`), [rollback.md](rollback.md) §3 (presentation under rollback),
[../engine/custom-screen.md](../engine/custom-screen.md) §3.5 (the chatbox).

## 0. Summary

- **BN6 draws battle text with two fonts, and a lot of what looks like text is pictures.** The 8x16 font (224
  glyphs, fixed 8-pixel advance, a face colour and a shadow colour) draws chip names, telops, the other player's
  used chip and the HUD's text lines. The dialogue font (16x12 cells, proportional advance from a width table,
  about 460 glyphs with kana and kanji) draws the chatbox: chip descriptions and the no-running message. Banners,
  every number but the turn countdown (HP, damage, the count box, round numbers), "PAUSE" and "Cstmzing..." are
  tile pictures.
- **The frontend draws only the 8x16 font today.** The custom screen is not drawn yet, so descriptions and the
  run message are not drawn, and the dialogue font is not in the pack.
- **The 8x16 font spells every BN6 chip name** (298 of 298, all within 8 glyphs). It has Latin letters, digits,
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
    box, a bundled open-licence pixel font for the strings it can't.
  - `auto` changes no pixel of BN6's own content, so the default stays comparable with the original, and mods,
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
  small dependency (`fontdue`), and one or two committed font files with their licences. It should start after
  the custom screen is drawn (§6.5).

## 1. Today's state

### 1.1 The fonts the original has

| Font | Where (original labels) | Cell and advance | Colours | Glyphs | In the pack |
|---|---|---|---|---|---|
| The 8x16 font | `dword_86B7AE0`, drawn into tiles by `renderTextGfx_8045F8C` and as sprite glyphs by the HUD tasks | 8x16 cell, one glyph a cell, advance always 8; the ink sits almost wholly in rows 3 to 14 and is 7 pixels wide plus the shadow | 2: face (index 1) and a shadow to the right and below (index 2) | 0xE0 (224) | yes: `graphics/hud/font.png` and `font_chars` in `hud.json` |
| The dialogue font | `byte_86ACD60`, drawn by `sub_3006F8C` (the chatbox) and `sub_80461D0` | 16x12 cell; advance from a width table (`byte_8043CA4`; two more tables for other screens): capitals and digits 8, `I` 6, lowercase 6 to 8, the space 8, kana and kanji 11 | a face colour (index 1) and a softer one on some pixels (index 3); the chatbox adds a colour offset to pick the text colour | about 460: the first 0xE4 by one byte, the rest by a two-byte code (`E4 nn`); the kana and about two hundred kanji are still in the US ROM | no (nothing draws the chatbox yet) |
| The banner letters | per banner, 20 pointers to 8x16 letter pictures that include the bar behind them (`pt_801EF84`) | 8x16, 20 cells a banner | the banner palette | (pictures) | as one picture a banner: `graphics/hud/banners/<name>.png` |

The text encoding is one table, `compat/text.toml` (what each byte below 0xE0 draws, as UTF-8). Glyphs with no
character of their own have bracketed names: `[EX]`, `[SP]`, `[RV]`, `[BX]`, `[FZ]`, the button marks `[A]`, `[B]`,
`[L]`, `[R]`, `[bat]`, `[z]`. The extractor writes the same table into `hud.json` as `font_chars`.

### 1.2 Every place text is drawn, or would be

"Cells" below are the 8-pixel columns and rows of the HUD's tile layer. "Drawn" says whether the frontend draws
it today.

| What | The string comes from | How it becomes pixels | The original's layout | Layer and colours | Drawn |
|---|---|---|---|---|---|
| The next chip's name in the hand window | the chip definition's `name` (UTF-8), through the hand's chip handle | `Hud::glyphs` spells it in the 8x16 font, longest glyph name first; at most 8 glyphs | from cell column 0 on rows 18 and 19 (y 144); the damage digits follow the last glyph at once, then `+` and the bonus, then `x2` | the HUD tile layer (priority 1), in the HP box's palette, so it turns with the HP box's colour state (normal, healing, hurt or low) | yes |
| A telop | the chip the engine's `Banner::telop` names (`Battle::telop_for` gives the viewer's side of it), then the definition's `name` | the 8x16 font as 8x16 sprite parts | name, damage, `+bonus` centred as 15 glyphs are in the viewer's half of the screen (`x = place + (15 - n) * 4`; places (0, 32) and (120, 32)); `x2` after; the other player's moves 16 pixels left for an `x2`. No scroll. The banners' vertical squash: it grows over 5 ticks, holds 0x30, shrinks over 5 (`sub_801CE28`) | sprites of priority 0, front layer, bucket 6; the HP box's normal palette | yes |
| A hidden telop, "????" | a literal in the frontend (the original shows the name of chip 0x171) | four `?` of the 8x16 font | as a telop | as a telop | yes |
| The chip the other player just used | `Battle::used_chips`, then the definition's `name` | the 8x16 font as sprite parts | laid out as a telop at the right-hand place, without the squash, for 60 ticks | as a telop | yes |
| "????" beside the mugshot and at the right edge (a defensive chip is set) | nothing: it is a picture | HUD layer tiles 0x1CC and 0x1CD | 4x2 cells at column 6 or 26, rows 2 and 3 | HUD tile layer | yes |
| The turn countdown and "TIME UP!" | the pack's `hud.json` `texts` (lines 3 to 13 of the ROM's HUD text script, as glyph numbers; the seconds are lines like `"    5   "`) | the 8x16 font | 8 glyphs from column 11, rows 0 and 1; the padding spaces in the line centre it | HUD tile layer, HP box palette | yes |
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
  digits under its navi (sprites in three colours), the judge's two numbers, the count box beside the mugshot
  (0 to 10, one picture each), the custom screen's damage digits and chip code letters.
- **Labels**: "PAUSE" (five sprite glyphs), "Cstmzing..." (8x2 tiles), the full gauge's "L or R", and the custom
  screen's fixed labels.

### 1.4 What the 8x16 font covers

- Space, `0`-`9`, `A`-`Z`, `a`-`z`; `* - × = : % ? + ! & , . ・ ; ' " ~ / ( ) ｢ ｣ _ █ ー ゜`; katakana with the
  voiced and small forms; 40 hiragana (no voiced forms, and no う, え, お, わ, を or ん: the table ends where the
  control codes begin); the kanji 熱 and 斗; and the eleven bracketed marks.
- No accented letters, no kanji beyond those two, no Cyrillic, Greek, Hangul or anything else, and no `[`, `]`,
  `#`, `@`, `<`, `>`.
- Every BN6 chip name is spelled within 8 glyphs: 210 names take all eight, 57 take seven, the rest fewer
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
  - a dimming darkens only the stage's palettes, so text keeps its colours, as it does now;
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
  - the fades applied to text colours by the text layer (the screen fade, and the HUD fade for HUD-layer items);
    the telop's squash as a transform; sprites blended over text approximated;
  - the window (`app.rs`) and the PNG writer (`headless.rs`) compositing the items after scaling, and a real
    scaling policy if the window stops being an integer multiple;
  - the sprite limit handled as in A.
- A cheaper variant, text into the frame at 2x or 4x of a supersampled frame, has the same costs and still isn't
  pixels.

For a pixel font the two give the same picture at integer scales (§8), so B buys nothing unless the font is a
vector font.

### 2.2 Libraries

The versions are the ones examined; licences and dependencies are read from their manifests. Newer versions may
exist.

| Crate | What it does | Licence | Dependencies | Notes |
|---|---|---|---|---|
| `fontdue` 0.9 | parses TTF/OTF and rasterizes outlines; a simple layout helper | MIT OR Apache-2.0 OR Zlib | `ttf-parser`, `hashbrown` | No shaping (its README points to cosmic-text for that), no hinting, no embedded bitmap strikes. One maintainer. The smallest way to turn a font file into glyph bitmaps |
| `ab_glyph` 0.2 | parses (through `owned_ttf_parser`) and rasterizes outlines; exposes embedded bitmap strikes (`EBDT`: mono, 2-, 4-, 8-bit) as raw data | Apache-2.0 | `ab_glyph_rasterizer`, `owned_ttf_parser` → `ttf-parser` | No shaping, no hinting. Reads the bitmap strikes that `.otb` and some pixel TTFs carry |
| `swash` 0.2 | scaling with hinting, rasterizing (`zeno`), bitmap strikes, colour glyphs | Apache-2.0 OR MIT | `skrifa` → `read-fonts`; optional `zeno`, `yazi` | The rasterizer cosmic-text uses. Hinting is what makes small vector text tolerable |
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

The repository can hold fonts under open licences (OFL, Apache, public domain or as permissive). It cannot hold
a font traced from the ROM's glyphs: that is ROM-derived media.

| Source | For | Against |
|---|---|---|
| **A bundled open-licence pixel font** | text always draws, the same on every machine; fits the frame; can be chosen to sit well beside BN6's art | no open font is BN6's; it needs choosing, and a CJK one is large (thousands of glyphs; not measured here) |
| **A bundled general UI font** (Noto Sans, Inter, M PLUS) | wide coverage, familiar | only usable with option B |
| **The user's system fonts** | nothing to bundle; covers the user's language | differs on every machine, so screenshots and tests don't reproduce; needs font discovery (`fontdb`, fontconfig on Linux); licence unknown. Reasonable only as an explicit `--font <path>` |
| **A font the content or the pack names** | a mod, a translation or a from-scratch pack brings its own look and coverage; BN6's pack is the case where the font happens to be extracted | the frontend still needs a fallback for a pack that names none |

These combine: the pack or the content root names fonts for the text roles, and the frontend bundles a fallback.

Candidates, with licences as their projects state them (checked 2026-10-01; read the licence file in the release
again before committing one):

| Font | Size | Covers | Licence | Fits |
|---|---|---|---|---|
| Ark Pixel Font | 10 and 12 px; monospaced and proportional builds; released as OTF, TTF, OTB, BDF, PCF | Latin, kana, kanji (Japanese, Chinese and Korean builds) | OFL 1.1 | the dialogue role (the original's is 12 rows tall with 11-pixel kana), and names where proportional is accepted. Actively maintained; its 16 px size was dropped in September 2026 |
| Fusion Pixel Font | 8, 10, 12 px | Latin and CJK, merged from Ark Pixel, Misaki, Galmuri and others | OFL 1.1 | as Ark Pixel, with an 8 px size |
| k8x12 | 8x12, one glyph an 8-pixel cell | kana, JIS level 1 and 2 kanji; half-width Latin | its author's free-software notice: use, copying, modification and redistribution without conditions | Japanese names in the original's 8-pixel cells, at the original's 12-row height. Not OFL: the user should read the notice |
| Misaki | 8x8 (7x7 ink) | kana, JIS level 1 and 2 kanji | as k8x12 | Japanese in 8-pixel cells where 12 rows don't fit |
| Galmuri | 8, 10, 12, 15 px | Latin-1, kana, 6,355 kanji, Hangul | OFL 1.1 | Korean; a DS-era look |
| GNU Unifont | 8x16 and 16x16 | the whole Basic Multilingual Plane | GPLv2+ with the font exception, and OFL 1.1 since 13.0.04 | the last-resort fallback: it has everything, in the original's cell height, but thin, and CJK takes two cells |
| Terminus | 8x16 (and others), with bold | Latin, Greek, Cyrillic: 1,356 characters | OFL 1.1 | Latin names in 8x16 cells with a bold weight near the original's |
| Spleen | 8x16 (and others); BDF, OTB, OTF | Latin-1, Latin Extended-A | BSD 2-Clause | as Terminus; permissive but not one of the three licences named |
| Pixel Operator (and its Mono, Bold and 8 px cuts) | 16 px and 8 px | Latin | CC0 1.0 (its HB cut is OFL) | proportional Latin at 7 pixels a letter: 8 letters fit the 64-pixel name box |
| Press Start 2P | 8x8, 8-pixel advance | Latin, Greek, Cyrillic | OFL 1.1 | drops into the 8-pixel cells unchanged, at half the height; an arcade look |
| Departure Mono | 11 px, monospaced | Latin | OFL 1.1 | status and debug text |
| Noto Sans, Noto Sans JP, M PLUS 1p, Inter | vector | very wide | OFL 1.1 | option B only |

Under the OFL a subset or otherwise modified font may not keep a Reserved Font Name, if the font declares one.
Committing a release file unmodified, with its licence text beside it, avoids the question.

### 2.4 Layout

The original's boxes were sized for 8-pixel cells: 8 cells for a name (64 pixels), 15 for a telop's name and
numbers (120 pixels, half the screen), 8 and 17 for the HUD lines, and 192 pixels by three lines for the chatbox.

- **Cell fonts** (the extracted 8x16 font, or any font with an 8-pixel advance): lay out by cells as today. This
  is the only layout the reference mode uses.
- **Proportional fonts**: lay out by the font's advances and place by pixel width. The telop's centring becomes
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
  spells all of it within its box, else with the font stack. On BN6's content every string takes the first
  branch, so `auto` and `original` produce identical frames there. That identity is cheap to assert: a test that
  renders the comparison's frames in both modes, or simply that `auto` reports no fallback over a trace.
- **`font`** draws all text from font files. It is what a pack without extracted fonts gets, and what a user who
  prefers one consistent font chooses.
- **Comparing in `font` mode stays possible with text masked.** The text layer knows every rectangle it drew.
  With an option to write those rectangles beside each headless frame, the comparison script can mask them and
  check that everything else is still identical: that catches a font mode that disturbed the sprite limit, the
  layer order or a fade. This is optional; the reference mode is the real check.
- **Tests in this repository** use a small hand-made bitmap font, as the frontend's tests use synthetic assets
  now: layout (centring, overflow, ellipsis, wrap), fallback choice, and exact bitmaps. The bundled font is
  tested through its bitmap form, which is exact (§2.2).

Why `auto` and not `font` as the default: the project measures itself by pixel-exactness, the extracted font is
always there when BN6's pack is loaded (the frontend can't draw a sprite without the pack), and it is the look
players know. What a font file adds is the strings the extracted font can't draw. Making `font` the default would
trade the original look for consistency in every frame, to fix a problem BN6's own content doesn't have.

## 4. Content model

### 4.1 What stays as it is

- `name` and `description` in UTF-8 in the definitions, as the canonical text.
- `description`'s `\n` as the line breaks. They are also the line-break hint: no new field.
- The 8-glyph limit is BN6's box, not a rule of the content model. The content check can warn when a BN6 name
  isn't spellable in 8 glyphs of `compat/text.toml` ("this name will be drawn with the fallback font"), without a
  pack.

### 4.2 What content should say that it doesn't

- **The run message's words.** Today a navi's `run_message` is characters per line. To draw it at all, in any
  mode, it should be the text (`"line one\nline two"`), with the engine deriving the counts as it derives a
  description's line count. The agent drawing the custom screen will need this first.
- **The HUD's lines and "????".** "TIME UP!", "COUNTER HIT!", the delete lines and the hidden chip's name are
  game text. They should be strings in the content root (a small strings module or table), with their centring
  done by the frontend instead of by padding spaces. The pack's `texts` then only checks them.
- **Which fonts a pack or content root offers, and for which role.** Two roles exist: names (the cell font) and
  dialogue. BN6's pack offers its extracted fonts for both; another pack names font files. This is the
  frontend's data, never the engine's, like `compat/`: a `fonts` section of the pack's asset index and a small
  table in the content root that maps roles to fonts. No `define.*` and no place in `Content`.
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

- Pixel positions, cell sizes and font sizes of BN6's HUD: that is the frontend's BN6 layout (`hud.json` and
  `hud.rs`).
- The text mode and the user's font choice: a preference.
- Rasterizer settings, the fallback order beyond what a pack names, the shadow's shape.
- The digit pictures and the labels of §1.3.
- A per-string style. If a string ever needs emphasis or a colour, that is markup in the dialogue text, as the
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
5. **A bundled fallback font** under an open licence, committed with its licence text.
6. **Numbers and picture labels unchanged. Banners out of scope.**

### 6.2 Why, and the alternative I rejected

- It answers what a font file is actually needed for: strings the extracted font can't draw, and packs that have
  no extracted font. It does so without giving up a pixel of the original on BN6's content.
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
| 4 | Choose and commit the fallback font with its licence; the content check's warnings (a name outside the glyph table or over 8 glyphs, text not in normalization form C); docs | S, plus the choice |
| 5 | The HUD's lines and "????" as content strings; per-language strings tables outside the hash; the frontend's `--lang` | M (about 300, plus content) |
| 6 | Optional: the comparison's text mask (the frontend writes text rectangles; the script masks them) | S, in both repositories |
| 7 | Optional, a separate decision: output-resolution text. The renderer returns text items and a depth mask; the window and the PNG writer composite them; cosmic-text with supplied fonts only; a UI font bundled | L (800 to 1,200, some thirty new crates) |
| 8 | Optional, a separate decision: banners made from a string, for navis a pack adds. It needs a banner-style font drawn for the purpose | L |

Steps 1 to 4 are the feature. Step 5 is what makes translations possible. Each step leaves the reference mode
pixel-identical, and steps 1, 3 and 5 should each end with the frame comparison run.

### 6.4 Crates and files

- `crates/bn6-assets`: a `font` module with the bitmap font type; `Hud` gains the dialogue font (step 0) and, at
  step 5, loses `texts` as a source of words.
- `crates/bn6-content`: `font.rs` (the pack's fonts to bitmap fonts, the BDF reader, the `fontdue` loader behind
  a feature); `hud.rs` and `names.rs` for a `fonts` section of the asset index; a `hud.json` version bump when the
  dialogue font lands.
- `crates/bn6-extract`: `hud.rs` for the dialogue font and its widths (step 0).
- `crates/bn6-frontend`: `text.rs` becomes a `text/` module (layout, drawing, modes, the font stack; the 3x5
  status font moves beside it or is replaced by the bundled font); `hud.rs` and the custom screen's drawing call
  it; `main.rs` gets the options; `audit.rs` the fallback notes; `Cargo.toml` one dependency.
- `crates/bn6-content-check`: the warnings of step 4.
- `content/bn6`: the strings of step 5; `run_message` as text.
- `crates/bn6-battle`: only step 0's `run_message` reading (counts derived from text). Nothing else: the engine
  has no part in text.
- A new `assets/fonts/<family>/` (or the frontend crate's own `assets/`) for the committed font and its licence.
- `docs/frontend.md`, `docs/design/asset-formats.md` §4, and this document rewritten as built.
- In the verification workspace: the comparison passes `--text original`; optionally the mask.

### 6.5 Sequencing with the frontend work in progress

Another agent is fixing presentation differences in `crates/bn6-frontend` and is about to draw the custom
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
- **What can go ahead in parallel without touching the frontend**: step 2 (new files in `bn6-assets` and
  `bn6-content`), the content check's warnings, and choosing the font.

### 6.6 What the user must decide

1. **The aim.** Coverage and independence from the extracted font, with the original look kept (this
   recommendation), or crisp modern text at window resolution (step 7 becomes the feature)?
2. **The default mode**: `auto` (recommended), `font`, or `original`.
3. **The fallback font or fonts.** My suggestion is one family for both roles, Ark Pixel 12 px proportional in
   its Japanese build (OFL: Latin, kana, kanji), and to audition it in the hand window and a telop beside two
   cell fonts before committing: Terminus 8x16 bold (OFL) for Latin names in the original's cells, and k8x12 for
   Japanese in 8-pixel cells if its licence notice is acceptable. Whether to commit a CJK font now (a large file)
   or Latin now and CJK with the Japanese content is part of this.
4. **Whether numbers change.** Recommended: no, in every mode.
5. **Whether banners are in scope.** Recommended: no; step 8 if a pack ever adds navis.
6. **Whether translations are wanted**, which decides whether step 5's strings tables are built now or the HUD's
   lines just move to content.
7. **Where the committed font lives**: with the frontend (a fallback for any content) or in `content/bn6` (BN6's
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
- **The font candidates were not auditioned in the frame**, apart from the three pixel fonts of §8. Their licences
  and sizes are from their projects' pages; how each looks beside BN6's HUD is still to be seen (step 4).
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
- **Coverage of the content** (§1.4): every `define.chip` in `content/bn6` spelled against `compat/text.toml`.
- **Pixel fonts shipped as outlines are exact at their native size.** Three fonts (a 16 px proportional one, its
  8 px cut, and an 8x8 arcade font) rasterized with `fontdue` 0.9.4 and `ab_glyph` 0.2.32 at their native sizes:
  every coverage value was 0 or 255, every advance a whole number of pixels (7, 5 and 8), and the two crates'
  results were identical. The same fonts drawn at four times the size on a 960-pixel-wide canvas match the
  frame-resolution drawing scaled up, so an output-resolution layer adds nothing for them.
- **A UI font in the frame is not usable.** Noto Sans at 11 px gave a spread of some 150 distinct grey levels
  over one eight-letter name, different between the two rasterizers; drawn into the frame it is soft with
  antialiasing and uneven without. Noto Sans JP kanji at 10 and 12 px in the frame are unreadable. Both are sharp
  on the 960-pixel canvas, where they no longer look like the game.
- **Widths**: "GunDelS3 120" is 96 pixels in the original's cells, 96 in the 8x8 arcade font, 79 in the 16 px
  proportional pixel font and 71 to 74 in the UI fonts at 11 px. Seven katakana and a digit are 64 pixels in the
  original's cells and 78 in a 12 px full-width font.
- **Dependencies**: the spike built with `fontdue` (`ttf-parser`, `hashbrown`) and `ab_glyph`
  (`ab_glyph_rasterizer`, `owned_ttf_parser`) in about six seconds. `cargo tree` for cosmic-text 0.15 with its
  system-font feature off lists about forty packages.
