-- Types shared across the exe4 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `.zed/generate-luau-definitions.py`; see README.md): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API is in content/nettai/core.d.luau.)

-- An EXE4 patch card (patch_cards/<card>): what every game's card is (@exelib's `PatchCardSpec`, EXE4's without an
-- MB) and its catalog slot: the slot's number as the save and the setup count it, 0 to 5, which the card writes by
-- its name (rules/patch_cards/cards' `SLOT_0A` to `SLOT_0F`, the labels the game shows: a card belongs in one slot,
-- and a slot holds one card). The slot is the printed card's (a scanned card's own data names it: no ROM holds a
-- table of them; tools/exe4/gen_patch_cards.py, bn6battle-verify, whose check holds each to its number), a fact of
-- the card for the tools that list and file cards. Nothing in a battle reads it: the reload runs whatever card each
-- slot of the setup holds (rules/patch_cards).
export type SlottedPatchCard = {
    id: string?,
    slot: number,
    effects: { any },
}

-- What a patch card's effect works on in the reload besides the stats (rules/patch_cards): the HP the NaviCust's
-- programs and the cards add.
export type PatchCardWork = { hp: number }

-- A ring (objects/ring, attack object 0x6D: one of the 3-byte rows at 0x080DAFB0, which each thrower states): its
-- sheet, its speed in pixels a tick, its hits' status (none: none), its sound as it sets off, its hit modifier, and
-- the panel type its hit leaves (SandRing's sand; none: none).
export type RingVariant = {
    sprite: SpriteAsset,
    speed: number,
    status: Status?,
    sound: Sound,
    hit_mod: number,
    sand: PanelType?,
}

-- A breath chip's breath (chips/breath, attack object 0x36): its own panel was hit (+0x60), and the column's three
-- ahead (+0x64, +0x68, +0x6C: level, up, down); its sound's count (+0x22).
export type BreathState = {
    hit: boolean,
    ahead: boolean,
    above: boolean,
    below: boolean,
    count: number,
}

-- A flying shot's variant (objects/flying_shot: one of the 16-byte rows at 0x080CE5B0, which each user defines where
-- it fires it).
export type FlyingShotVariant = {
    -- Its collision (the row's first three bytes: what it is, what it hits, the hit modifier).
    is: CollisionType,
    hits: CollisionType,
    hit_mod: number,
    -- Its element byte.
    element: number,
    -- Its hit spark (0xFF: none).
    spark: Spark?,
    sprite: SpriteAsset,
    anim: number,
    -- It casts a shadow on the ground (else its first part is drawn with the sprite).
    shadow: boolean,
    -- It lights the panels it covers (its second parameter).
    highlight: boolean,
    -- The panel centers it passes before it ends (its third parameter).
    range: number,
    -- 16.16 a tick, forward.
    speed: number,
    -- What shows at the panel's center as its range runs out (row 5's: effect row 0x56).
    end_effect: Effect?,
}

-- The NaviCust's bug counts (rules/navicust's), which a card raises or clears.
export type PatchCardCounts = {
    add: (bug: string, n: number) -> (),
    clear: (bug: string) -> (),
}

-- A button a soul adds to the custom screen (a `ButtonSpec` rules/souls makes of it): where it sits, and A on it, which
-- gets the rules' state of the side (`s`: the fields the soul's `custom.state` declares among them). It is on the
-- screen while the side's navi is in the soul.
export type SoulButton = {
    slot: number,
    cells: number?,
    -- Its uses on a screen, for a button that counts them (`custom.button_uses`).
    uses: number?,
    pressed: (side: number, s: any) -> (),
}
-- A window a soul adds (a `WindowSpec` of rules/souls's): its update, each tick it is up: whether it stays up.
export type SoulWindow = {
    view: WindowView?,
    update: (side: number, s: any) -> boolean,
}

-- What a soul does at the custom screen (a soul's form's `custom`; EXE4's screen keeps the navi's soul as it opens,
-- 0x0801DC82: its +0x0F, and reads it in each of these places): the fields it keeps in the rules' state, its buttons
-- and windows by name (a button's is the key of its look in a pack), and its part of the screen's routines, each
-- called while the side's navi is in the soul.
export type SoulCustom = {
    state: StateSchema?,
    buttons: { [string]: SoulButton }?,
    windows: { [string]: SoulWindow }?,
    -- As the screen deals (0x0801DD0C), on the side's console (`battle.console_rng_positive`).
    deal: ((side: number, s: any) -> ())?,
    -- As OK builds the hand (0x0801F034).
    confirmed: ((side: number, s: any) -> ())?,
    -- On the hand OK built, as it is installed: its cursor at its first chip and no charge bonus on it yet
    -- (0x0801F1B0, 0x0801F1E2: what the build does to it by the soul).
    built: ((side: number) -> ())?,
}
