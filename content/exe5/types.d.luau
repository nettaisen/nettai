-- Types shared across the exe5 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `.zed/generate-luau-definitions.py`; see README.md): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API is in content/nettai/core.d.luau.)

-- A flying shot's variant (objects/flying_shot: one of the 16-byte rows at
-- 0x080C5D0C, which each user defines where it fires it).
export type FlyingShotVariant = {
    -- Its collision (the row's first three bytes: what it is, what it hits,
    -- the hit modifier).
    is: CollisionType,
    hits: CollisionType,
    hit_mod: number,
    -- Its element byte.
    element: number,
    -- Its hit spark (0xFF: none).
    spark: Spark?,
    sprite: SpriteAsset,
    anim: number,
    -- It lights the panels it covers (its second parameter).
    highlight: boolean,
    -- The panel centers it passes before it ends (its third parameter).
    range: number,
    -- It casts a shadow on the ground (else its first part is drawn with
    -- the sprite).
    shadow: boolean,
    -- The status its hits carry (`object_setCollisionStatusEffect1`).
    status: Status?,
    -- 16.16 a tick, forward.
    speed: number,
}

-- KnightMan's wrecking ball's look (chips/knightmn/ball: its variant, the
-- original's fourth parameter): its sprite, its height and its radius; and a
-- swing of it: how far it turns a tick, its turns, its status and its look.
export type KnightBallLook = { sprite: SpriteAsset, z: number, radius: number }
export type KnightBallSwing = { speed: number, turns: number, status: Status?, look: KnightBallLook }

-- A bomb of CrakBom's kind (chips/crakbom/crakbom: the original's Param1, its
-- kind): its burst on each panel of the column (effects 0x50 to 0x53 by the
-- kind's low bits) and its landing's sound (0x080C932C), its hit (0x080C93C8:
-- hit modifier, status, bug), and whether it cracks the column (CrakBom's).
export type CrakBomVariant = {
    burst: Effect,
    sound: Sound,
    hit_mod: number,
    status: Status?,
    bug: number,
    cracks: boolean,
}

-- LarkMan's swoop (chips/larkman/swoop: its variant, the original's Param1):
-- its speed (0x080DD858's rows), its turn's step (0x080DDB7C's rows: 16.16
-- of 256 steps), the ticks its turn lasts less (0x080DDB9C's rows) and more;
-- whether it turns with WildBird's own animations (0x080DDC44), and whether
-- its navi's command powers it (0x080DDCD0).
export type LarkSwoopVariant = {
    speed_x: number,
    speed_y: number,
    turn_step: number,
    shorter: number,
    longer: number,
    wild_turn: boolean,
    commanded: boolean,
}

-- A magnet's variant (chips/magnetmn/magnet: the original's fourth
-- parameter, 0x080D2DE0 and 0x080D2DEC): its hit modifier and status; whether
-- it waits for MagnetMan to be done sending (else it goes at once), and
-- whether it acts and animates while the battle is dimmed.
export type MagnetVariant = { hit_mod: number, status: Status?, waits_for_navi: boolean, runs_while_dimmed: boolean }

-- A Meddy capsule's variant (chips/meddy/capsule: the original's first
-- parameter): its palette (0x080C5FE8), its body's and its burst's hit
-- modifier (0x080C5FF4, 0x080C62A8), its status (0x080C6010) and its
-- explosions (the effect table's row 0, its palette raised by 0x080C62C4's);
-- and its line (its second parameter: 0x080C628C, 0x080C6298, 0x080C633C):
-- its burst's region, the panels it explodes on, its target's marker style
-- and its animation as it sits.
export type CapsuleVariant = { palette: number, hit_mod: number, status: Status?, explosion: Effect }
export type CapsuleLine = { region: Region, panels: { { number } }, style: number, anim: number }

-- A button a soul adds to the custom screen (a `ButtonSpec` of the souls
-- part's, which rules/souls/custom makes of it): where it sits, and its
-- functions, which get the rules' state of the side (`s`: the fields
-- the soul's `custom.state` declares among them). It is on the screen while
-- the side's navi is in the soul, outside battle mode 1.
export type SoulButton = {
    slot: number,
    cells: number?,
    uses: number?,
    right: number?,
    left: number?,
    state: ((side: number, s: any) -> ("selectable" | "unavailable"))?,
    pressed: (side: number, s: any) -> (),
    chip: ((side: number, s: any) -> Chip?)?,
}
-- A window a soul adds (a `WindowSpec` of rules/souls's): its update,
-- each tick it is up: whether it stays up.
export type SoulWindow = {
    view: WindowView?,
    update: (side: number, s: any) -> boolean,
}

-- What a soul adds to the custom screen (a soul's form's `custom`; EXE5's
-- screen asks the soul MegaMan is in, NaviStats +0x2C, in each of these
-- places): the fields it keeps in the rules' state, its buttons and
-- windows by name (a button's is the key of its look in a pack), and its
-- part of the screen's routines, each called while the side's navi is in the
-- soul.
export type SoulCustom = {
    -- What it keeps of one screen: as every screen deals, whatever soul the
    -- navi is in, each field is back as a fresh state has it, by its
    -- declared type (0, false, no definition: the original zeroes the
    -- screen's record as it opens, 0x08022CA2).
    state: StateSchema?,
    buttons: { [string]: SoulButton }?,
    windows: { [string]: SoulWindow }?,
    -- As the screen opens: how many chips it deals (0x08025BE4; none: the
    -- framework's hand).
    hand_size: ((side: number, s: any) -> number?)?,
    -- As the screen lays its slots out (0x08023CF8, after its deal: where
    -- its buttons are, so not in battle mode 1), on the side's console
    -- (`battle.console_rng_positive`).
    deal: ((side: number, s: any) -> ())?,
    -- OK built the hand (0x08024EC0).
    confirmed: ((side: number, s: any) -> ())?,
    -- The turn's start, before either side's change of form (0x08011DDC).
    turn_opened: ((side: number, s: any) -> ())?,
}

-- EXE5's fields of the engine's forms: the rules' extensions
-- (`extends`, rules/souls), which the engine's specs take untyped
-- (`[string]: any`). A module that wants them typed casts to these. A
-- soul's: its navi's image at the change and what it adds to the custom
-- screen (above); its sword's arm, the blade's animation (lib/swords,
-- 0x080EC038); its charged sword warping behind the enemy whatever the
-- charge (lib/stepsword, 0x08012538); VarSwrd's wait for its sequence
-- while A is up (chips/varswrd, 0x080ED570); the cameras' shake as MegaMan
-- emerges in it (rules/souls/change, 0x0801212C).
export type Exe5FormFields = {
    -- A soul (rules/souls), named by its form's id: the family of the chips
    -- given up for it.
    soul: { family: ChipFamily }?,
    image: SoulImage?,
    custom: SoulCustom?,
    blade_anim: number?,
    steps_behind: boolean?,
    var_sword_waits: boolean?,
    emerge_shake: boolean?,
    -- Its Chaos Unison (rules/souls/chaos_charge): the weapon its charge
    -- fires, and its own row of the charge's cycle (MeddySoul's; none: the
    -- chaos level's, at most 2).
    chaos: { weapon: Weapon, cycle: number? }?,
}
