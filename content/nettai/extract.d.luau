-- The extraction recipes' API (docs/design/extraction-recipes.md §3), as a recipe sees it: the primitives
-- nettai-extract gives it (crates/nettai-extract/src/recipe/prims.rs) and the model it returns (its codec,
-- recipe/model.rs). Only modules under a pack's extract/ check against it, and they against nothing of the battle's
-- (core.d.luau): a recipe runs in a VM of its own, with these libraries and none of `asset`, `new`, `rules` or
-- `battle`, and requires only extract/ modules, its own pack's and its support packs'.
--
-- Recipes are the one place Luau holds the original's numbers (ROM addresses, chip numbers, sprite categories):
-- the extractor is the boundary, as compat is. Numbers are GBA addresses (0x08000000 on) and plain integers; a
-- recipe computes with `bit32` and `//`. Every primitive checks its bounds: a bad address is an error naming it.
--
-- A recipe (`extract/init.luau`) returns a table (`Recipe`): the game's ROMs by their header codes, and a function
-- from them (`Set`) to the model (`Extracted`). The host reads the codes without ROMs, finds the ROMs by them, runs
-- the function, then makes the pack: the check that every named asset is there, the letters, the element icons'
-- frame, the hand-drawn frames (extract/shapes/), the sound bank, the files.
--
-- The model's tables use its own field names (crates/nettai-assets); a field left out takes the model's default,
-- and an unknown one is an error naming its path. Lists are arrays; a list with holes is keyed by the game's number
-- from 0, with its length `n`; a list of named things is `{ { name = ..., value = ... } }`; a tuple is an array
-- (a place `{ x, y }`); an enum is its variant's name.

-- ---- ROMs and bytes ----------------------------------------------------------------------------------------

-- Bytes read from a ROM or decompressed: offsets from 0.
declare extern type Bytes with
    function len(self): number
    function u8(self, at: number): number
    function u16(self, at: number): number
    function u32(self, at: number): number
    function sub(self, from: number, len: number): Bytes
end

-- One of the game's ROMs (an image the handles share). Every read is recorded with the recipe's line for
-- `nettai-extract --reads`.
declare extern type Rom with
    -- Its header's code ("BR6E"), as the recipe's `roms` states it.
    code: string
    -- Its place in the recipe's `roms`, 1 to 4: what a place reads an address by (@exelib/extract/place).
    slot: number
    function u8(self, at: number): number
    function u16(self, at: number): number
    -- A word: a pointer word is recorded as one.
    function u32(self, at: number): number
    -- Whether `at` is a ROM address inside this ROM.
    function holds(self, at: number): boolean
    function bytes(self, at: number, len: number): Bytes
    -- Up to `max` bytes, an open read (a parser takes what it needs of it).
    function span(self, at: number, max: number): Bytes
    -- A BIOS LZ77 (0x10) block, decompressed; nil if it isn't one.
    function lz77(self, at: number): Bytes?
end

-- The game's ROMs, by code (`set.BR6E`): every one the recipe's `roms` states, always all of them.
export type Set = { [string]: Rom }

declare bytes: {
    -- Bytes from hex digits (white space between them ignored).
    hex: (text: string) -> Bytes,
}

-- ---- Tiles, palettes, maps ---------------------------------------------------------------------------------

-- 8x8 tiles, a palette index a pixel.
declare extern type Tiles with
    function count(self): number
    -- These tiles (from 0), in this order.
    function pick(self, tiles: { number }): Tiles
    function sub(self, first: number, count: number): Tiles
end

-- A background map entry: its tile, its flips, its palette.
export type MapEntry = { tile: number, hflip: boolean, vflip: boolean, palette: number }

-- Map entries in the GBA's text-mode format, decoded.
declare extern type Map with
    function count(self): number
    -- Entry `i` (from 0).
    function entry(self, i: number): MapEntry
end

-- Sixteen 15-bit colors.
export type Palette = { number }

declare gba: {
    -- 4bpp tiles: `len` bytes from `at` (a ROM address, or an offset into Bytes).
    tiles: (from: Rom | Bytes, at: number, len: number) -> Tiles,
    join: (parts: { Tiles }) -> Tiles,
    -- A palette's colors as stored (whether bit 15 counts is the recipe's: @exelib/extract/colors).
    palette: (from: Rom | Bytes, at: number) -> Palette,
    palettes: (from: Rom | Bytes, at: number, count: number) -> { Palette },
    -- `count` halfword entries; `first_tile` is taken off each entry's tile, wrapping in 16 bits.
    map: (from: Rom | Bytes, at: number, count: number, first_tile: number?) -> Map,
}

-- ---- Sprites, animations, fonts ----------------------------------------------------------------------------

-- A sprite archive's sheet: its animations' frames, tiles, palettes and parts, placed in the model as it is.
declare extern type Sheet with
end

declare sprite: {
    -- A sprite list's entry's archive: with bit 31, LZ77 (its first word, the size, dropped); else an open read of
    -- 0x80000 bytes. Nil: none there.
    archive: (rom: Rom, entry: number) -> Bytes?,
    sheet: (archive: Bytes, category: number, index: number) -> Sheet?,
    -- A portrait (the chatbox's speaker): its one frame's mini-animations as its animations.
    portrait: (archive: Bytes, category: number, index: number) -> Sheet?,
}

-- A background's graphics animation (palettes, tiles or a palette shift on a schedule).
declare extern type GfxAnim with
end

declare gfx: {
    -- A background's animation list at `list`, palette addresses counted from `palette_buffer`.
    anims: (rom: Rom, list: number, palette_buffer: number) -> { GfxAnim },
}

-- The dialogue font (the chatbox's).
declare extern type DialogueFont with
end

declare font: {
    -- `count` glyphs of 0x60 bytes (16x12, the left pixel in each byte's low nibble) at `glyphs`, their advances
    -- at `advances` (`advance_bytes` apart: 1 in EXE6, 4 in EXE5 and EXE4, the low byte read), glyph k drawing
    -- chars[k].
    dialogue: (rom: Rom, glyphs: number, advances: number, count: number, advance_bytes: number, chars: { string }) -> DialogueFont,
}

-- A file of the game's compat/, by its path in the game's pack (`compat/assets.toml`): TOML as tables (read once,
-- frozen).
declare function data(path: string): any

-- ---- The model ------------------------------------------------------------------------------------------

-- A list of named things, in order.
export type Named<T> = { { name: string, value: T } }

export type BannerLayout = {
    x: number?,
    y: number?,
    -- 0 plain, 1 with a number, 2 and 4 hold until removed, 3 a telop's (no glyphs of its own).
    kind: number?,
    glyphs: Tiles?,
    number_at: { number }?,
}

export type Chatbox = {
    tiles: Tiles?,
    palette: Palette?,
    -- By kind (the message box, the description box), its four opening steps.
    boxes: { { Map } }?,
    arrow: Tiles?,
    text_palette: Palette?,
}

export type TelopTiles = { banner: number?, names: { number }?, signs: number?, pause_loads_banner_palette: boolean? }
export type TelopLook = { places: { { number } }?, glyphs: Tiles?, palette: Palette?, tiles: TelopTiles? }

-- Where the HUD's code puts its pieces, where games differ (a field left out: EXE6's).
export type HudLayout = {
    pause: { number }?,
    hp_number_priority: number?,
    hud_priority: number?,
    message: { number }?,
    judge_from_hold: boolean?,
    chatbox_text: { number }?,
    chatbox_arrows: { { number } }?,
    telop: TelopLook?,
}

export type HudLettering = {
    font: Tiles?,
    font_chars: { string }?,
    dialogue_font: DialogueFont?,
    texts: { { number } }?,
    -- The banners whose words differ, keyed by banner id / 4 from 0, with the base's count `n`.
    banners: { [number]: BannerLayout, n: number }?,
    banner_palette: Palette?,
    waiting: Tiles?,
    waiting_palette: Palette?,
    gauge_tiles: Tiles?,
    chatbox_text: { number }?,
}

export type Hud = {
    tiles: Tiles?,
    first_tile: number?,
    gauge_tiles: Tiles?,
    gauge_first_tile: number?,
    hp_palettes: { Palette }?,
    gauge_palette: Palette?,
    hp_box: Map?,
    gauge_frame: Map?,
    font: Tiles?,
    font_chars: { string }?,
    enemy_digits: { Tiles }?,
    enemy_palette: Palette?,
    chip_icons: { { key: string, tiles: Tiles? } }?,
    hidden_icon: Tiles?,
    icon_palette: Palette?,
    -- Each face: { tiles, palette }.
    mugshots: { { any } }?,
    counts: { Tiles }?,
    count_box: Tiles?,
    mugshot_boxes: { Tiles }?,
    navi_mugshots: { { tiles: Tiles, palettes: { Palette } } }?,
    navi_box: Tiles?,
    pause: Tiles?,
    texts: { { number } }?,
    banners: { BannerLayout }?,
    banner_digits: Tiles?,
    banner_palette: Palette?,
    waiting: Tiles?,
    waiting_palette: Palette?,
    warning: Tiles?,
    warning_palette: Palette?,
    dialogue_font: DialogueFont?,
    chatbox: Chatbox?,
    layout: HudLayout?,
    language: string?,
    languages: Named<HudLettering>?,
}

-- A cursor's place and its corners: two frames of four, each { y, x, hflip, vflip }.
export type CursorPlace = { x: number?, y: number?, corners: { { { any } } }? }

export type CustomLayout = {
    column_cells: number?,
    turn_limit: number?,
    turn_limit_palette: number?,
    name: number?,
    art: number?,
    code: number?,
    element: number?,
    digits: number?,
    slots: number?,
    column_icons: number?,
    name_bar: number?,
    form_names: number?,
    slot_blank: number?,
    detail_blank: number?,
    empty_palette: number?,
    ok_cursor: CursorPlace?,
    regular: { number }?,
    column_columns: { number }?,
    column_row: number?,
    form_names_at: { number }?,
}

export type Picture = { tiles: Tiles?, palette: Palette? }
export type SlotPictures = { ok: Picture?, ok_picked: Picture?, other: Picture? }
export type ButtonPlace = { x: number?, y: number?, first_tile: number?, palette: number? }

export type ButtonPictures = {
    width: number?,
    height: number?,
    tiles: Tiles?,
    sets: ("each" | "other" | "unavailable")?,
    hidden: number?,
    cursor: CursorPlace?,
    picture: Picture?,
    palettes: { Palette }?,
    uses_digit: boolean?,
    held_at: { number }?,
    icons: Tiles?,
    icon_palette: Palette?,
    icon_palettes: Named<Palette>?,
    place: ButtonPlace?,
}

export type MapPatch = { x: number?, y: number?, width: number?, height: number?, palette: number?, by_column: boolean? }
export type PatchList = { first_tile: number?, patches: { MapPatch }? }
export type WindowLabel = { name: string?, first: number?, last: number?, top: number?, bottom: number?, letters: { number }? }
export type ChipArt = { key: string, picture: Picture?, version: string? }
export type Emblem = { navi: string, tiles: Tiles?, palette: Palette? }
export type VersionPictures = { buttons: Named<ButtonPictures>?, form_names: Tiles?, form_name_palettes: { Palette }? }
export type Versioned<T> = { base_version: string, base: T, versions: Named<T>? }
export type ButtonLettering = { tiles: Tiles?, picture: Picture? }
export type CustomLettering = { pictures: SlotPictures?, form_names: Named<Tiles>?, buttons: Named<ButtonLettering>? }
export type ElementSprite = { x: number?, y: number?, palette: Palette? }

export type WindowEmblem = {
    x: number?,
    y: number?,
    width: number?,
    height: number?,
    first_tile: number?,
    tiles: Tiles?,
    frames: { Map }?,
    -- Keyed by step from 0, with their count `n`.
    steps: { [number]: number, n: number }?,
}

export type CustomScreen = {
    layout: CustomLayout?,
    labels: { WindowLabel }?,
    buttons: Named<ButtonPictures>?,
    window_tiles: Tiles?,
    column_cells: Tiles?,
    turn_limit: Tiles?,
    name_bar: Tiles?,
    window_maps: { Map }?,
    window_patches: PatchList?,
    form_list_maps: { Map }?,
    form_list_patches: PatchList?,
    frame_palettes: { Palette }?,
    icon_palette: Palette?,
    gray_palette: Palette?,
    other_palette: Palette?,
    chip_art: { ChipArt }?,
    pictures: SlotPictures?,
    codes: Tiles?,
    elements: Tiles?,
    element_colors: { { number } }?,
    digits: Tiles?,
    slot_codes: Tiles?,
    empty_icon: Tiles?,
    versioned: Versioned<VersionPictures>?,
    cursor: Tiles?,
    form_list_cursor: Tiles?,
    form_list_cursor_palette: Palette?,
    emblems: { Emblem }?,
    regular: Tiles?,
    advance_name_colors: { { number } }?,
    languages: Named<CustomLettering>?,
    element_sprite: ElementSprite?,
    cursor_palette: Palette?,
    window_emblem: WindowEmblem?,
}

export type PaletteAnim = {
    slot: number?,
    -- Each frame: { palette, how many frames it shows }.
    frames: { { any } }?,
    initial_timer: number?,
}

export type Field = {
    tiles: Tiles?,
    first_tile: number?,
    palettes: { Palette }?,
    first_palette: number?,
    palette_anims: { PaletteAnim }?,
    panel_types: { string }?,
    panels: { Map }?,
    front_edges: { Map }?,
    highlights: { Map }?,
}

export type Speeding = { step: { number }?, top: { number }?, from_battle: boolean? }

export type Background = {
    tiles: Tiles?,
    first_tile: number?,
    map: Map?,
    map_width: number?,
    map_height: number?,
    palette: Palette?,
    scroll: { number }?,
    speeding: Speeding?,
    anims: { GfxAnim }?,
}

export type Graphics = {
    sprites: { Sheet }?,
    field: Field?,
    -- Keyed by the battle's background number from 0, with their count `n`.
    backgrounds: { [number]: Background, n: number }?,
    hud: Hud?,
    custom: CustomScreen?,
}

-- The names the pack's assets are written under, by the game's numbers.
export type AssetNames = {
    -- By category, then index.
    sprites: { [number]: { [number]: string } }?,
    songs: { [number]: string }?,
    backgrounds: { [number]: string }?,
    mugshots: { [number]: string }?,
    banners: { [number]: string }?,
    -- A chip's key by its number (its icon and picture are written under it).
    chips: { [number]: string }?,
    -- A navi's key by its number (its emblem is written under it).
    navis: { [number]: string }?,
    glyphs: { string }?,
    dialogue_glyphs: { string }?,
    language_glyphs: { [string]: { glyphs: { string }?, dialogue_glyphs: { string }? } }?,
}

-- Which ROM the sound bank is read from, and which ROMs are versions of it (their songs that play otherwise come
-- in as the version's own, under its name).
export type Sound = {
    from: Rom,
    base_version: string?,
    versions: { { name: string, from: Rom } }?,
}

export type Extracted = {
    names: AssetNames,
    graphics: Graphics,
    sound: Sound,
}

-- What a recipe's extract/init.luau returns: nothing else.
export type Recipe = {
    -- The game's ROMs by their header codes (the four letters and digits at 0xAC), every one required (a complete
    -- Legacy Collection's), in the order the recipe's places read an address by (a ROM's slot, from 1): the base
    -- version's US ROM, the other's, then their Japanese ones.
    roms: { string },
    -- The ROMs, every one of `roms` (`set.BR6E`), to the model.
    extract: (set: Set) -> Extracted,
}
