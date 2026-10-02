# Text rendering: the extracted pixel fonts, and font files

**Status (2026-10-02): built.** The user chose crisp text at the window's resolution (option B of §2.1, which §6
had rejected as the default) with a bundled open-licence font, as the default; §9 "As built" says what was built
and how, and records the decisions. §10 is the languages built on it (`--lang en|ja`) and where display text lives
(no definition holds any: `locales/<lang>.toml`). §0 to §8 are the investigation as written before it, unchanged.

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

- **BN6 draws battle text with two fonts, and a lot of what looks like text is pictures.** The 8x16 font (224
  glyphs, fixed 8-pixel advance, a face colour and a shadow colour) draws chip names, telops, the other player's
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
character of their own had bracketed names: `[EX]`, `[SP]`, `[RV]`, `[BX]`, `[FZ]`, the button marks `[A]`, `[B]`,
`[L]`, `[R]`, `[bat]`, `[z]` (since 2026-10-02 each is a character: §10.5). The extractor writes the same table into
`hud.json` as `font_chars`.

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

- `crates/nettai-assets`: a `font` module with the bitmap font type; `Hud` gains the dialogue font (step 0) and, at
  step 5, loses `texts` as a source of words.
- `crates/nettai-content`: `font.rs` (the pack's fonts to bitmap fonts, the BDF reader, the `fontdue` loader behind
  a feature); `hud.rs` and `names.rs` for a `fonts` section of the asset index; a `hud.json` version bump when the
  dialogue font lands.
- `crates/bn6-extract`: `hud.rs` for the dialogue font and its widths (step 0).
- `crates/nettai-frontend`: `text.rs` becomes a `text/` module (layout, drawing, modes, the font stack; the 3x5
  status font moves beside it or is replaced by the bundled font); `hud.rs` and the custom screen's drawing call
  it; `main.rs` gets the options; `audit.rs` the fallback notes; `Cargo.toml` one dependency.
- `crates/nettai-content-check`: the warnings of step 4.
- `content/bn6`: the strings of step 5; `run_message` as text.
- `crates/nettai-battle`: only step 0's `run_message` reading (counts derived from text). Nothing else: the engine
  has no part in text.
- A new `assets/fonts/<family>/` (or the frontend crate's own `assets/`) for the committed font and its licence.
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

## 9. As built (2026-10-02)

### 9.1 The user's decisions

The user's request: "implement native font rendering for places in the ui with dynamic text". On the questions of
§6.6:

1. **The aim: crisp text at the window's resolution.** Option B of §2.1: a vector font drawn at the output's
   resolution over the scaled 240x160 frame. Step 7 of §6.3 became the feature; the `auto` mode and the pixel
   fallback font of §6.1 were not built.
2. **The default mode: the font.** `--text font|original`, default `font`. `original` is today's exact pixel
   rendering, the reference the frame comparison and the tests use.
3. **The font: a bundled open-licence font**, committed with the frontend with its licence text, so the text looks
   the same everywhere. The user chose **Murecho** ("murecho supports jp text and looks mostly like your current
   font"), over my first pick, Saira (§9.4); the description text's weight is the user's too (300).
4. **Numbers, banners and other pictures stay pixel art** (§6.6 4 and 5).
5. **The font lives with the frontend**: `crates/nettai-frontend/fonts/murecho/`, with `OFL.txt`. `--font PATH`
   puts another TrueType or OpenType file in its place.
6. Translations (§4.3) and the HUD's lines as content strings (§4.2) were not part of it. Translations came
   next: §10.

### 9.2 What is text and what stays a picture

Every string below comes from content or varies; each is drawn in the original's box for it.

| String | Role | Box (the original's) | In front of it (depth) | Fades |
|---|---|---|---|---|
| The next chip's name (the hand window) | cell | its glyphs' cells from (0, 144); the damage digits stay after them | the HUD layer's | the HUD layer's, the screen's |
| The chip window's name (custom screen) | cell | the window's 8 name cells where its map puts them, cut to the window's columns while it slides | the HUD layer's | as above (Beast Out's darkening, a Cross's whitening) |
| A telop, "????" for a hidden one | cell | the name's cells, centred; the digits after them | its sprite parts' (front layer, bucket 6) | the sprites', the screen's; the banners' squash |
| The chip the other player just used | cell | as a telop, without the squash | its sprite parts' | as a telop |
| The Program Advance animation's names and codes | cell | 8 cells for the name, the 9th for the code | the HUD layer's | the HUD layer's |
| The enemy names (round's first screen) | cell | their glyphs' cells on BG0, ending at column 30 | BG0's | BG0's (the HUD palettes') |
| The HUD's lines: the turn timer's seconds, "TIME UP!", "COUNTER HIT!" | cell | the cells of the pack's line without its padding spaces | the HUD layer's | the HUD layer's |
| "VS" between the judge's numbers | cell | 2 cells | the HUD layer's | the HUD layer's |
| A description's and the run message's lines (the chatbox) | dialogue | the 192-pixel line, 12 rows, every 14 rows from (51, 108); cut where the line buffer's sprites end, plus 3 rows for a third line's descenders | its 18 sprite parts' (front layer, bucket 3: the key-wait arrow and the portrait are in front) | the sprites', the screen's |

Not text, so unchanged: the Crosses' names in the Cross window (the pack's pictures, by each Cross's game); the
custom screen's slots and picked column, which show icons and the code strips' letters, no names; chip codes,
the damage digits, HP numbers, the judge's numbers, banners, "Cstmzing...", "PAUSE", the "????" beside the mugshot;
the frontend's own status text (`text.rs`).

### 9.3 How it works

- **Collecting** (`textlayer.rs`): the places that draw text ask a `TextSink`. In the font mode, for a string the
  font has every character of, they leave its glyphs out of the frame and hand the sink a `TextItem` (the words,
  the role, the box, the alignment, a clip, the face and shadow colours of the palette the original draws it with,
  the squash, how many units are printed). A string the font lacks a character of is drawn whole in the game's
  font, as in the original mode (a string never mixes two fonts). What the glyphs leave behind is the original's
  background: blank HUD-layer cells, the chip window's name cells in the window's colour 8, blank sprite parts in
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
- **Colours and fades**: the face and the shadow are the palette's indices 1 and 2 (the chip window's 9 and 10, as
  its name's pixels are shifted by 8; the chatbox's index 1 and no shadow), through `apply_fade` as the frame's
  own pixels go. So a dimming leaves text as it leaves the HUD, Beast Out's fade darkens the custom screen's names,
  a Cross's choice and the FlashBomb's flash whiten them, and the screen fades take everything.
- **The squash**: an item's `vscale` is the banners' `n / 256` texture step; the text is scaled vertically by
  256 / n about the box's middle and cut to the box's 16 rows, as the sprite is.
- **Drawing** (`present.rs`, `vfont.rs`): the window (`app.rs`) and the PNG writer (`headless.rs`) both go through
  `present`: the frame scaled to the output, then each item laid out and rasterized at the output's scale and
  blended over it, pixel by pixel through the item's clip and the depth keys of the frame pixel under it.
- **The scaling policy**: the picture takes the largest whole multiple of 240x160 that fits the output, centred
  on black, so every frame pixel is the same square whatever the size; only an output smaller than 240x160 gets a
  fractional, shrunk picture. The window is resizable and follows this; text is drawn at the same placement and
  scale. Headless PNGs are `--png-scale` times 240x160, exactly.
- **The rasterizer**: swash 0.2 (§2.2): shaping with the font's kerning (in the frame's pixels, independent of the
  output's scale), antialiased outlines (8-bit coverage), variable fonts' axes, hinting for glyphs under 40 pixels
  an em (1x and 2x; above that the outlines are left as drawn). Glyph images are cached by glyph, size, axes,
  scales and a quarter-pixel horizontal phase; layouts by string, role and box. Eight packages came with it (swash,
  skrifa, read-fonts, font-types, zeno, yazi, bytemuck, bytemuck_derive); cosmic-text wasn't needed.

### 9.4 Layout

- **Roles.** The 8x16 font's strings (`Role::Cell`): capitals 10 frame pixels high (the original's take rows 4
  to 13 of the 16), the baseline 14 rows down, weight 700, a shadow in the palette's shadow colour half a frame
  pixel right and down (the original's is a pixel; half reads better at 3x and 4x). The dialogue font's
  (`Role::Dialogue`): capitals 9 pixels high, the baseline 10.5 rows down, weight 300, no shadow.
- **Fitting.** The box is the original's: the cells its glyphs take (one a character the pack's font lacks, up to
  the place's room), or where nothing follows a name (the chip window, the Program Advance's names) the whole of
  its cells. A string that fits is drawn at its natural width and never stretched; a cell string leaves a pixel
  free at the box's right, as the original's glyphs do before the digits. A string too wide is, in order: drawn in
  narrower cuts down to the width axis's "condensed" (75) for a font that has the axis; squeezed horizontally down
  to 85%, or 70% for a string with kana or kanji (their glyphs are an em wide where the 8x16 font's cells are about
  half that); then made smaller, centred on its capitals. It never leaves its box. With the bundled font, 123 of
  the 342 chip names are squeezed in their own cells, and three (Magnum, TmhkMan[EX], TmhkMan[SP]) are also made
  3% smaller.
- **Alignment**: left, as the original's glyphs start their cells; a telop's name centred in its cells.
- **Marks**: a mark is one character (§10.5). One the font has is the font's (Ⓡ, ✕, ○); else the layer draws it:
  a button (Ⓐ, Ⓑ) as its letter at 68% of the size in a ring centred on the capitals, a stacked mark (EX, SP) as
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
  of every name and description in `content/bn6` is in it, and of the 442 characters the pack's two fonts draw
  only 伊, 祐 and 綾 (story characters' names, which no battle string uses) are not. Japanese works as is: no
  shaping is needed, swash's shaper handles it, and the fitting's 70% squeeze sizes kana to the original's cells.
  A string with a kanji it lacks is drawn in the game's font (whose kanji are fewer still).
- **Weights**: one variable file, 100 to 900. Names, telops and the HUD's lines use 700 (bold, the weight the
  8x16 font's glyphs have); the chatbox uses 300, the user's choice, near the dialogue font's thin strokes.
- **The look**: a clean, slightly squared humanist sans with open shapes and a large x-height, legible at the
  10-pixel capitals of the HUD; it sits well beside BN6's pixel art.
- **Size**: 1.4 MB of the frontend's 13 MB release binary. Not subset: every glyph a Japanese chip name or description
  could need stays in.
- **Before it**: I had picked Saira (OFL), a squarish technical sans with a width axis, so that a name too wide
  for its cells condensed within its family; auditioned in the HUD and the chatbox at 4x beside Barlow Semi
  Condensed, Exo 2, Inter, Oxanium and Russo One. It has no kana or kanji. The user chose Murecho, which looks
  close to it and covers Japanese; without a width axis, the squeeze takes the axis's place.
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
- **Tests** (`cargo test -p nettai-frontend`): the depth mask; text hidden where a sprite in front won and faded
  with its layer; a font-mode telop as an item over blank parts where the original's glyph parts were; a HUD line
  without its padding; layout that never leaves its box, isn't stretched, and squeezes before shrinking; kana and
  kanji laid out; the placement policy; marks (a stacked mark in one cell, a button in its ring: §10.5). The font
  tests check layout and metrics, not pixels: the rasterizer's arithmetic is floating point (§2.2).
- **Looked at**, headless at 4x: the hand, the chip window, telops through their squash and stretch, a hidden
  telop, the other player's chip, a description and the run message (printing too), the Program Advance's names,
  "COUNTER HIT!", the seconds, "TIME UP!" and "VS", Beast Out's darkening of the enemy name, a Cross's whitening,
  Japanese names and a description (from a content copy with Japanese words), and the window (`NETTAI_WINDOW_SHOT`
  keeps its last picture). `--objects` lists each frame's items with how many of their box's pixels something in
  front covers.

### 9.7 Left open

- Text at 1x output is soft, as any vector text at that size (§8); the HUD wants 3x or more.
- A HiDPI screen: minifb reports the window's size in points, so the text is drawn at that resolution and the
  system scales it up.
- `--audit` reports what the pack's fonts lack, in either mode; a string the vector font lacks (drawn in the
  game's font) isn't reported.
- The HUD's lines as content strings (§4.2): not done; they stay the pack's text script, per language (§10).
  Translated strings tables (§4.3) are built: §10.

## 10. Languages and display text (as built, 2026-10-02)

The user's requests: "add localization support so you can play either with japanese or english text"; then "the
english text should also be extracted into locales/en.toml. the actual luau code etc should be free from text, all
text should be stored via the locales file"; "why does ChipSpec even have description_lines? why not count it from
the locale string?"; and to call it strings or display text, not "words".

The frontend's `--lang en|ja` (default `en`) shows a battle's text in English, as the US games do, or in Japanese,
as the Japanese games (EXE6 Falzar and Gregar) do, in either text mode. No definition holds display text: every
string is a content root's `locales/<lang>.toml`.

### 10.1 What a battle shows as text, and where each comes from

| What | English | Japanese |
|---|---|---|
| A chip's name (the hand window, a telop, the other player's chip, the chip window, the Program Advance's names) | `locales/en.toml` `[chips]` | `locales/ja.toml` `[chips]`, the Japanese ROMs' chip names (キャノン, ハクシャク for Count) |
| A chip's description (R on the custom screen) | `en.toml` | `ja.toml`, the Japanese ROMs' descriptions |
| A Cross's description | `en.toml` `[forms]` | `ja.toml` `[forms]` |
| The no-running message (L) | `en.toml` `[navis]` `run_message` | `ja.toml` `[navis]` `run_message` |
| The enemy names (a round's first custom screen) | `en.toml` `[navis]` `name`, the ROM's name for the navi's NameID (ChrgeMan, GrndMan, TmhkMan, ProtoMan) | `ja.toml` `[navis]` `name` (ロックマン, キラーマン, アクアマン, ブルース...) |
| A Cross's name (the frontend's own text: live play's terminal summary, the plain-text screen's Cross window) | `en.toml` `[forms]` | (English) |
| A patch card's name (gen-content checks them; the frontend's `--cards` messages; no screen shows them yet) | `en.toml` `[patch-cards]`, the fan translation's | `ja.toml` `[patch-cards]`, the Japanese ROMs' card names |
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
  description), `[navis]` (name, run_message), `[forms]` (a Cross's name and description), `[patch-cards]` (a
  patch card's name). What reads each is §10.6. The engine's
  `content::strings::Strings` is one table; nettai-content's `locale` reads them. A definition holds none: the define
  phase refuses a `name`, `description` or `description_lines` field (core.d.luau's specs have none).
- **The content's own language** (`locale::OWN`, English for BN6) is part of the content: the loader puts its table
  in `Content::strings`, and the define phase counts what the battle reads of it into the records: a description's
  lines (`ChipData::description_lines`, `FormData::description_lines`) and a no-running message's characters per line
  and which of them move the speaker's mouth (`RunMessage::counts`, `talking`). The hash covers the records, so it
  covers that shape; the strings themselves are presentation, out of it (renaming every chip changes nothing; a
  description with another number of lines changes the content). Nothing in the definitions duplicates the strings.
- **Another language's table is a frontend's alone.** It changes neither the battle nor `Content::hash()`, so two
  players can each read their own language in one netbattle. A translated description may have another number of
  lines than the own one; the battle keeps the own one's timing.
- **`en.toml` is the US ROMs' strings** (written once from the definitions' former text, which was the US ROM's,
  but for what the content names itself: the chips the US release cut and named otherwise or not at all, Count's
  and Django's; the Crosses' names; the patch cards' names; the three Giga chips' descriptions, which no ROM has). **`ja.toml` is
  the Japanese ROMs'**, written once by the verification workspace's `gen-content locale-draft ja`. Both are people's
  since, and `gen-content check` compares them with the ROMs (one check for both languages, `locale::check_table`):
  every chip's name and description, every Cross's description, every navi's name (by its NameID) and no-running
  message, and that the two Japanese ROMs agree. The DblBeast, Gregar and Falzar chips' scripts print a gift's text
  the save keeps (`FF 01 nn`); `ja.toml`'s are the text Tango's EXE6 netplay saves hold, with a comment saying so,
  and the check only asks that the font can draw them.
- **The Japanese ROMs' text encoding** is compat/text.toml's `[jp]`: what each byte of their 8x16 font and dialogue
  font draws (written once from the fonts: each glyph whose bitmap is one of the US fonts' is that glyph's character,
  the rest read off the fonts; the verification workspace's tools/jp/locale/textjp.py). The extractor writes the
  Japanese fonts' charmaps with it; gen-content decodes the Japanese text with it and checks that every Japanese chip
  name encodes back to the ROM's bytes.
- **Pictures with text, and the fonts, are assets**: the pack's lettering in another language, extracted from the
  Japanese ROMs as `-ja` files (asset-formats.md §4, "Languages"; bn6-extract `lettering`).
- **The HUD's lines stay the pack's text script** (§4.2 is still open): the Japanese ROMs' are the same English
  words, which the Japanese font draws with the same pictures.

### 10.3 How the frontend uses them

- `--lang ja` loads the pack's graphics, swaps the Japanese lettering into the HUD's and the custom screen's fields
  (`Bundle::in_language`; nothing that draws them changed), and loads `locales/ja.toml`. A language the content root
  has no table of, or the pack no lettering in, stops the frontend with what there is.
- The places that draw content's text (hud.rs, custom.rs, chatbox.rs) ask the frame's `TextSink` for it
  (`strings.rs`, `DisplayText`): the language's string, else the content's own, else the key; `--audit` lists what
  the language's table lacks. The frontend's own text (live play's status line, folder listings) is the content's own
  strings.
- **`--text original --lang ja`** draws the Japanese text in the Japanese fonts as the Japanese games do: a string is
  encoded with the Japanese font's characters, as the English is with the US font's. **The font mode** draws it with
  Murecho, which has the kana and kanji, and the dialogue font's symbols (✕, ○: the Japanese descriptions'
  "攻撃力✕2"); the marks it lacks the text layer draws (§10.5).
- **Timing stays the content's own.** A translated description shows its own lines, whole, in the proportion of the
  own description's lines printed (16 of the 410 Japanese chip descriptions, and HeatCross's and SpoutCross's, have
  two lines where the English has three); a translated no-running message prints in the proportion of the own
  message's characters printed, so it ends when that would. A Japanese console times its chatbox by its own text, so
  those boxes take keys a tick or more apart from the engine's; nothing else in the battle reads text.
- **A Japanese console's HUD code** (docs/engine/jp-differences.md §5), drawn on a Japanese console's screen
  (`Renderer::console_region`): its custom screen's close also starts the chip window's HUD task, so the next chip's
  name shows from the screens' exchange through the turn's banner (`HudState`); "Cstmzing..." is as wide as its
  picture. One fix the Japanese consoles showed: a palette flash of variant 0 leaves the HUD layer as the
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

- **Where they are written**: compat/text.toml (`"\uE002"`, as TOML escapes: a Private Use Area character shows as
  nothing in most editors), locales/{en,ja}.toml (`"Count\uE002"`, `"Press Ⓐ\nto burn a..."`), gen-content's
  built-in charmap and the verification workspace's tools/jp/locale/textjp.py. The battle's strings use Ⓐ, Ⓑ, ✕,
  U+E002 and U+E003; the others are in the encoding only.
- **The original mode** draws the same pixels as before: the charmaps (`font_chars` and the dialogue font's `chars`
  in `hud.json`, written from compat/text.toml by the extractor) map the characters to the same glyph numbers. A
  pack written before (hud.json version 6) has the bracketed names, so the frontend asks for it to be extracted
  again (version 7).
- **The font mode**: a mark the font has is drawn by the font (Ⓡ, ✕, ○ with Murecho). One it lacks the text layer
  draws (`vfont::mark`): a button as its letter at 68% of the size, centred in an antialiased ring 1.15 times the
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
- `[weapons]` (124 names: 69 navi and form weapons', 55 patch card weapons') was read by nothing (no screen
  shows a weapon's name; `Strings::weapon` had no caller): the table, `WeaponStrings` and its check are gone.
- `[patch-cards]` (`PatchCardStrings`): the patch cards' names, by card key, which gen-content checks (the English
  ones there, the Japanese ones the ROM's); no screen of the frontend shows them yet. (Until the cards became an
  engine definition kind, 2026-10-02, they were records and their names a `[records]` table.)
