-- Types shared across the bn6 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/bn6/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API and data types are in core.d.luau.)

-- Where an owner keeps an attached object: its actor overlay slot, its
-- first related object, or its content state's `held` or `held2` field
-- (lib/slot.luau).
export type Slot = "overlay" | "related" | "held" | "held2"

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

-- A projectile's variant (objects/projectile, the original's `off_80C4C78`
-- rows): what it is and hits, its hit modifier, its element and secondary
-- elements, its hit spark (nil: none), its look (none: not drawn), the
-- status byte and bug its hits carry (nil or 0: none), what its hit does
-- to the panel, whether it bursts (around what it hits, and over the last
-- two columns when it leaves the field), whether it climbs (a pixel lower
-- down the field and higher on each panel).
export type ProjectileVariant = {
    collision: { is: CollisionType, hits: CollisionType },
    hit_mod: number,
    element: string,
    secondary: { string }?,
    spark: Spark?,
    sprite: SpriteAsset?,
    anim: number?,
    status: number?,
    bug: number?,
    bug_arg: number?,
    hit_panel: PanelHit?,
    bursts: boolean?,
    climbs: boolean?,
}

-- A flying shot's variant (objects/flying-shot, the original's
-- `byte_80C6038` rows): what it is and hits, its hit modifier, element and
-- secondary elements, hit spark (nil: none), look and shadow, speed (16.16
-- a tick, forward), range (panel centers it passes before it ends), the
-- status byte its hits carry; it lights its panel (`highlight`), is a
-- thrown obstacle (its look is the obstacle's), sparks over its panel's
-- center (`panel_spark`), sounds as it sets off after a wait, leaves an
-- effect when its range runs out.
export type FlyingShotVariant = {
    collision: { is: CollisionType, hits: CollisionType },
    hit_mod: number,
    element: string,
    secondary: { string }?,
    spark: Spark?,
    sprite: SpriteAsset,
    anim: number?,
    shadow: boolean?,
    speed: number,
    range: number,
    status: number?,
    highlight: boolean?,
    obstacle: boolean?,
    panel_spark: boolean?,
    launch_sound: Sound?,
    end_effect: Effect?,
}

-- What a flying shot is launched with (objects/flying-shot): its variant,
-- the ticks it waits before it sets off, whether it falls, its palette,
-- the damage word, and a thrown obstacle's kind | animation << 4.
export type FlyingShot = {
    variant: FlyingShotVariant,
    wait: number?,
    falls: boolean?,
    palette: number?,
    damage: number,
    obstacle: number?,
}

-- A bullet's variant (objects/bullet, the original's `byte_80C68D4` rows):
-- the region its burst hits on the panel it found, the burst's hit
-- modifier and status byte (nil or 0: none), the effect it shows (over
-- the region, or once at the bullet when `single`), and its sound.
export type BulletVariant = {
    region: Region,
    hit_mod: number,
    effect: Effect,
    single: boolean?,
    status: number?,
    sound: Sound?,
}

-- A cannon (lib/cannon.luau, the original's action 0x14): the arm the navi
-- holds, the projectile it fires, and whether it leaves afterimages as it
-- charges (the GigaCans).
export type CannonSpec = {
    -- The action's key when no chip definition holds it (its compat key).
    id: string?,
    arm: AttachmentLook,
    shot: ProjectileVariant,
    afterimages: boolean?,
}

-- A wide wave (chips/widesht/wave.luau, the original's attack #0x3B by
-- kind): its speed (16.16 pixels a tick); whether it goes through what it
-- hits (with the hit modifier it's fired with), holds its spawner's attack
-- and waits before sliding, or trails afterimages; what it is (a collision
-- type), the bug it gives and its palette.
export type WaveVariant = {
    speed: number,
    pierces: boolean,
    holds: boolean,
    trails: boolean,
    collision: CollisionType,
    bug: number?,
    palette: number?,
}

-- A sonic boom (lib/swords/sonic_boom.luau): its hit modifier, whether it
-- goes through what it hits.
export type SonicBoom = { hit_mod: number, pierces: boolean }

-- TankCan's shell (chips/tankcan/shell.luau): its hit's modifier, whether
-- its blast at the field's edge cracks the panels, how far the blast's
-- damage word shifts right.
export type ShellSpec = { hit_mod: number, cracks: boolean, shift: number }

-- What a projectile (objects/projectile) is fired with
-- (lib/projectile.luau).
export type ProjectileShot = {
    -- What it is.
    variant: ProjectileVariant,
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

-- ---- The dimming chips (chips/wind, chips/guardian, ...) -------------------

-- A Wind's or Fan's fan (chips/wind/fan): its palette; `pulls`: it blows
-- Fan's pulling gust from the far column rather than Wind's pushing one
-- from the enemy's front column; ticks from becoming its side's wind to
-- the first gust; HP; its NameID (the field objects' look) and its kind
-- among absorbed obstacles.
export type FanVariant = {
    palette: number,
    pulls: boolean,
    wait: number,
    hp: number,
    name: number,
    absorbed_kind: number,
}

-- CircGun's gun (chips/circgun/gun; the original's chip parameters): the
-- ticks between its steps less one, its shots, and its look (the palette;
-- look 1's shots hit as another attacker with a bug).
export type CircGunVariant = {
    period: number,
    shots: number,
    look: number,
}

-- An Anubis statue (chips/anubis/statue, `byte_80CF0C0`): its animation,
-- the ticks between its drains, HP, lifetime, and NameID (the field
-- objects' look).
export type AnubisVariant = {
    anim: number,
    period: number,
    hp: number,
    lifetime: number,
    name: number,
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

-- A shock wave's look and timing (chips/wavearm/wave, a row of the
-- original's `byte_80C6B00`): its animation, ticks on a panel before the
-- next rolls on, the panel type it leaves ("cracked" cracks the panel,
-- "broken" breaks it, or cracks it under something); `quick`: it goes 4
-- ticks after sending the next; `steers`: it turns toward an enemy's row.
export type ShockWaveVariant = { anim: number, ticks: number, panel: PanelType?, quick: boolean?, steers: boolean? }

-- A tornado's look and rhythm (chips/tornado/tornado, a row of the
-- original's `byte_80CA064`): its palette, the ticks between hits, the bug
-- its hits carry (nil: none); `plain`: it takes its panel's variant.
export type TornadoVariant = { palette: number, rearm: number, bug: number?, plain: boolean? }

-- What a tornado is spawned with: its variant, the status its hits
-- inflict (0: none) and its ticks.
export type TornadoSpec = { variant: TornadoVariant, status: number, ticks: number }

-- A thunder ball (chips/thunder/ball): fast or not, the panels it drifts
-- (12 makes DarkThnd's look and sound), the status its hits inflict and
-- their bug (none by default).
export type ThunderBall = { fast: boolean?, panels: number, status: number?, bug: number? }

-- How a yoyo flies (chips/yoyo/yoyo): GreatYo's (three that move
-- together), the panels it rolls out and its spinning rounds.
export type YoyoSpec = { great: boolean?, panels: number, rounds: number }

-- A hockey puck (chips/airhocky/puck, a row of the original's
-- `byte_80C9818`): the steps it lasts (0: no limit), how it bounces (1:
-- off its own side too once across, 0: not), the ticks per step, and
-- whether it leaves an afterimage trail.
export type HockeyPuckVariant = { steps: number, own_side: number, ticks: number, trail: boolean? }

-- How an iron shell slides (chips/ironshl/shell): its palette and speed
-- (16.16 a tick), how many times it bumps at the far column, and whether
-- it is ParaShl's (four panels, then back).
export type IronShell = { palette: number, speed: number, bumps: number, para: boolean? }

-- What a flame burns like (lib/burner/flame): its ticks, the panels it
-- spreads forward, whether its first spread takes three rows (`wide`),
-- whether it cracks its panel as it goes out, and its look: its animation
-- (0 by default) and whether it starts on its panel rather than at its
-- owner's hand.
export type Flame = { ticks: number, spread: number, wide: boolean?, cracks: boolean?, anim: number?, on_panel: boolean? }

-- A burn (lib/burner/burn, the original's action 0x27): the burner the navi
-- holds and the flame it lights (the pose lasts the flame's ticks); `id`,
-- the action's key when no chip definition holds it.
export type BurnSpec = { id: string?, burner: AttachmentLook, flame: Flame }

-- A Reflector's shield's look (chips/rflectr/shield, a row of the
-- original's `byte_80C9664`): the sprite, its animation while up, the
-- animation and ticks of its fade, and its palette.
export type ReflectorShieldLook = { sprite: SpriteAsset, anim: number, fade_anim: number, fade_ticks: number, palette: number }

-- A Reflector's guard (chips/rflectr/guard, the original's action 0x2B):
-- how long it guards and the shield it holds up; the counterattack the
-- first blocked hit sends (`wave`: the Reflectors' wave; `shot`: the
-- buster's projectile; none); `heedless`: it heeds nothing it blocks (the
-- NaviCust Shield's). `id`: the action's key when no chip holds it.
export type ReflectorGuard = {
    id: string?,
    ticks: number,
    look: ReflectorShieldLook,
    counter: ("wave" | "shot")?,
    heedless: boolean?,
}
