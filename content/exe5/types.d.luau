-- Types shared across the exe5 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/exe5/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API is in content/nettai/core.d.luau.)

-- A soul's navi's image as MegaMan unites with it (rules/souls/image, actor
-- object 0x2A; a soul's form's `image`): the navi's sprite, and what the
-- image puts on as it comes in and takes off as it goes (0x0800EDBC's and
-- 0x0800EEBE's routines by soul: GyroMan's propeller, NapalmMan's cannon,
-- Colonel's cape, KnightMan's ball and chain, NumberMan's face; none: it
-- wears nothing).
export type SoulImage = {
    sprite: SpriteAsset,
    put_on: ((image: Object) -> ())?,
    take_off: ((image: Object) -> ())?,
}

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
    image: SoulImage?,
    custom: SoulCustom?,
    blade_anim: number?,
    steps_behind: boolean?,
    var_sword_waits: boolean?,
    emerge_shake: boolean?,
}
