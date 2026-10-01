-- Types shared across the bn6 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/bn6/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API and data types are in core.d.luau.)

-- Where an owner keeps an attached object: its actor overlay slot, its
-- first related object, or its content state's `held` field
-- (lib/slot.luau).
export type Slot = "overlay" | "related" | "held"

-- What the link navis' chips (action 0x0A, chips/190-heatpres) keep in the
-- attack's variables: one schema for the ten navis' routines.
export type LinkChipState = {
    -- The phase timer (AIAttackVars+0x10).
    timer: number,
    -- A destination panel (AIAttackVars+0x16/0x17).
    target_x: number,
    target_y: number,
    -- The object the routine keeps (AIAttackVars+0x30).
    held: Object?,
    -- A speed, 16.16 (AIAttackVars+0x34), or a second object there.
    speed: number,
    held2: Object?,
}

-- What a projectile (attack object #0, objects/projectile) is fired with
-- (lib/projectile.luau).
export type ProjectileShot = {
    -- The kind (data.objects.projectiles): its first parameter.
    kind: number,
    -- The damage word: the damage (with its flag bits) | the counter
    -- byte << 16.
    damage: number,
    -- Its height, 16.16 (the buster's shots fly at 24 pixels, 0x18_0000).
    z: number,
    -- Rows it also moves on its first step: 1 down, -1 up (its second
    -- parameter). None by default.
    row: number?,
}

-- The state of VarSwrd's and NeoVari's actions (lib/swords/vari.luau).
export type VariSwordState = {
    -- Ticks left to enter a sequence (AIAttackVars+0x12).
    countdown: number,
    -- The sequences as the navi faces: its direction flip.
    flip: number,
    -- How far into each sequence the navi is, in bytes (two per step).
    progress: { number },
}

-- What differs between VarSwrd and NeoVari (lib/swords/vari.luau).
export type VariSwordSpec = {
    -- The sequences by direction flip (0, 1), each a list of joypad words
    -- ending in 0.
    sequences: { [number]: { { number } } },
    -- The chip each sequence gives.
    choices: { number },
    -- A random pick is `choices` by `rng % random`, a Sword past them.
    random: number,
    -- The chip that doesn't turn into the charged sword (action 0x41).
    no_charged: number,
}

-- An attachment's look (objects/attachment): its sprite (a sprite asset,
-- or a "CC-II" string from the pack data's rows until their users
-- convert), its palette, the pixels it sits raised, the owner's attach
-- point it sits at (none: on the owner's position); `by_owner`: at a fixed
-- offset by the owner's NameID instead (row 0xF).
export type AttachmentLook = {
    sprite: SpriteAsset | string,
    palette: number,
    lift: number?,
    attach_point: number?,
    by_owner: boolean?,
}

-- The bombs and seeds (lib/bombs, docs/design/content-model-v2.md §5.1).
-- A thrower throws its thing from `pos` with the attack's damage word;
-- `variant` is what it throws (kept as data, so the canonical tree shows
-- what each chip throws).
export type Thrower = {
    throw: (user: Object, pos: Vec3, damage: number) -> Object?,
    variant: any?,
}

-- A throw (lib/bombs/throw): what the navi holds while winding up (its
-- look, animation and palette offset) and what throws it.
export type ThrowSpec = {
    -- The action's key when no chip definition holds it (its compat key).
    id: string?,
    held: AttachmentLook,
    held_anim: number?,
    held_palette: number?,
    thrower: Thrower,
}

-- A thrown bomb (lib/bombs/bomb): its collision, hit modifier, palette and
-- animation; the region it blasts where it lands (none: no blast); what it
-- leaves the tick after.
export type BombVariant = {
    collision: { is: CollisionType, hits: CollisionType },
    hit_mod: number,
    palette: number,
    anim: number,
    blast: Region?,
    after: ((me: Object) -> ())?,
}

-- A thrown seed (lib/bombs/seed): its element and palette, the panels it
-- grows, the effect on each and its sound.
export type SeedVariant = {
    element: number,
    palette: number,
    panel: PanelType,
    effect: Effect,
    sound: Sound,
}

-- A sword's hit (lib/swords): its region on the panel ahead, what it is and
-- hits, its hit modifier, and the status and bug it inflicts (none by
-- default).
export type SwordHit = {
    region: Region,
    collision: { is: CollisionType, hits: CollisionType },
    hit_mod: number,
    status: number?,
    bug: number?,
    bug_arg: number?,
}

-- A sword's slash (lib/swords/slash, the original's action 0x13): the blade
-- it holds, its hit and the slash drawn over the panel ahead (with a
-- palette offset), its swing's sound; DblDream swings twice, CrosSwrd adds
-- a single-panel hit, StepSwrd steps two panels ahead first.
export type SlashSpec = {
    -- The action's key when no chip definition holds it (its compat key).
    id: string?,
    blade: AttachmentLook,
    hit: SwordHit,
    effect: Effect,
    effect_palette: number?,
    sound: Sound,
    swings: number?,
    second_hit: SwordHit?,
    step: boolean?,
}

-- A machine sword's strike (lib/swords/strike, the original's action 0x49):
-- the blade it holds, which opposing navis it slashes where they stand
-- (paralyzed ones, ones on a grass, ice or volcano panel, or either), and
-- the slash's palette offset.
export type StrikeSpec = {
    blade: AttachmentLook,
    finds: "paralyzed" | "on_element_panels" | "either",
    effect_palette: number,
}

-- A FlashBomb (chips/flshbom/bomb): ticks set down, HP, the flash's status
-- and a second hit's as it goes (0 none), its palette; `strong`: the flash
-- hits with damage 20 and counter 0; `own_side`: it flashes its own side.
export type FlashBombVariant = {
    ticks: number,
    hp: number,
    status: number,
    final_status: number,
    palette: number,
    strong: boolean?,
    own_side: boolean?,
}

-- A panel change (objects/panel-changer, a row of the original's
-- `byte_80E272C`): which panels (all, the changer's row, the panel in
-- front of it, its own) meeting its side's condition become `panel` (on
-- side 1 `side1_panel` if set: the roads point the other way) after
-- `ticks`, showing as it meanwhile (every tick, or with `flicker` every
-- other four), with a tick sound every fourth tick and a sound as they
-- change (nil: none).
export type PanelChange = {
    panel: PanelType,
    side1_panel: PanelType?,
    panels: "all" | "row" | "front" | "own",
    flicker: boolean,
    ticks: number,
    sides: { [number]: PanelCondition },
    tick_sound: Sound?,
    change_sound: Sound?,
}

-- A barrier (lib/barriers): `sub_801A7CC`'s row (how it behaves, its HP,
-- the damage a hit must reach to wear it, its timer (0xFFFF: none), the
-- element that breaks it), and its visual (`byte_80E0A14`'s look: sprite,
-- animation, palette, the owner's attach point it sits at, the sprite
-- parts it hides; LifeAur's aura hides part 6 but for the local MegaMan),
-- with its sounds as it rises (and grows back) and as it goes down.
export type Barrier = {
    behavior: BarrierBehavior,
    hp: number,
    threshold: number,
    timer: number,
    weak: string?,
    look: {
        sprite: SpriteAsset,
        anim: number,
        palette: number,
        attach_point: number,
        hidden_parts: number,
    },
    hidden_unless_local_megaman: boolean?,
    sound: Sound,
    down_sound: Sound?,
}

-- An instrument (lib/instruments, a row of the original's `byte_80D4078`):
-- its identity (its NameID, a number until identities come) and palette,
-- how long it plays and pauses, its tune, and the effect it has every
-- third tick of playing (`off_80D4298`).
export type Instrument = {
    identity: number,
    palette: number,
    play_ticks: number,
    pause_ticks: number,
    tune: Sound,
    effect: (me: Object) -> (),
}

-- AirRaid's plane (chips/airraid, a row of the original's `byte_80D34C0`):
-- its lifetime in ticks, its bombs, its palette and HP.
export type AirRaidPlane = { lifetime: number, bombs: number, palette: number, hp: number }

-- ColorPt's and DblPoint's points (chips/colorpt): their palette (four a
-- step) and the Atk+ bonus each brings (the chips' parameters).
export type ColorPoint = { palette: number, bonus: number }
