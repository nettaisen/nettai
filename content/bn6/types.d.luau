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
    -- The action's key when no chip definition holds it (its compat key).
    id: string?,
    -- The sequences by direction flip (0, 1), each a list of joypad words
    -- ending in 0.
    sequences: { [number]: { { number } } },
    -- The action of the chip each sequence gives, and Sword's (what A let
    -- go of, or the time running out, gives).
    picks: { Action },
    sword: Action,
    -- A random pick is `picks` by `rng % random`, a Sword past them.
    random: number,
    -- The pick that doesn't turn into the charged sword (action 0x41).
    no_charged: Action,
    -- Transitional (counted by the ratchet): the picks' chips' numbers
    -- (`picks`' in order, and Sword's), which the attack's chip takes
    -- (their records' subtype and parameters are what the Beast rush and
    -- SlashCross's charged sword read) until those chips are definitions.
    legacy: { chips: { number }, sword: number },
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

-- A countdown bomb (chips/timebom/countdown, a row of the original's
-- `byte_80CD8AC`): the animations it rises and stands with, its HP, its
-- NameID (the field objects' look, a number until identities come), the
-- class of field-object slot it takes (0: one of its side's two; 1: its
-- side's one), what it is and what it reacts to. `strong_shake`: its blast
-- shakes the camera a second time, harder (TimeBom+). The rows no chip
-- sets: `allows_bodies`, it may appear under a body; `bursts_when_broken`,
-- broken it blows up, and its countdown is only the last 15 ticks, after
-- which it blinks away; `linked`, its spawner keeps it in a slot and it
-- reserves its panel once it stands.
export type CountdownVariant = {
    appear_anim: number,
    idle_anim: number,
    hp: number,
    name: number,
    class: number,
    collision: { is: CollisionType, hits: CollisionType },
    strong_shake: boolean?,
    allows_bodies: boolean?,
    bursts_when_broken: boolean?,
    linked: boolean?,
}

-- ---- The standard chip actions (content model v2, step 8g) --------------
-- Each family's builder spec: what differs between its chips, the
-- original's parameter bytes by name. `id`: the action's key when no chip
-- definition holds it (its compat key).

-- AquaNdl (chips/aquandl, the original's action 0x32): the needles'
-- palette.
export type AquaNeedleSpec = {
    id: string?,
    palette: number,
}

-- H-Burst (chips/h-burst, action 0x34): the bursts its shot makes where
-- it hits.
export type HyperBurstSpec = {
    id: string?,
    bursts: number,
}

-- A rolling log (chips/rlnglog/log): a big one also covers the panel
-- above it; the ticks a hit that takes HP stops it; its HP; thrown in an
-- arc onto the panel ahead (else it drops in from above).
export type RollingLog = {
    big: boolean,
    stop_ticks: number,
    hp: number,
    thrown: boolean,
}

-- RlngLog (chips/rlnglog, action 0x36): the logs it throws.
export type RollingLogSpec = {
    id: string?,
    log: RollingLog,
}

-- What AirSpin sends from the panel ahead (chips/airspin): the whirlwind
-- top that rolls and spins there (its palette, its spins), or the seeking
-- whirlwind no chip sends.
export type AirSpinSender = {
    send: (user: Object, x: number, y: number, element: number, damage: number) -> Object?,
    palette: number?,
    spins: number?,
}

-- AirSpin (chips/airspin, action 0x38).
export type AirSpinSpec = {
    id: string?,
    whirlwind: AirSpinSender,
}

-- DolThdr (chips/dolthdr, action 0x3E): the thunder's ticks, and whether
-- the navi holds the buster arm rather than the doll (no chip does).
export type DollThunderSpec = {
    id: string?,
    ticks: number,
    holds_arm: boolean?,
}

-- WindRack (chips/windrack, action 0x3F): the rack the navi swings.
export type WindRackSpec = {
    id: string?,
    rack: AttachmentLook,
}

-- An electric pulse (chips/elcpuls/pulse): its hit modifier and status,
-- its ticks, its palette and the bug its hits carry (none by default).
export type ElecPulse = {
    hit_mod: number,
    status: number,
    ticks: number,
    palette: number,
    bug: number?,
    bug_arg: number?,
}

-- ElcPuls (chips/elcpuls, action 0x42): the pulse, and the ticks the navi
-- holds it out.
export type ElecPulseSpec = {
    id: string?,
    pulse: ElecPulse,
    hold: number,
}

-- An aura head (chips/aurahed/head): its palette and speed (16.16 a
-- tick), its stronger hit (hit modifier 3, else 1), and whether it runs to
-- the field's far edge (else 3 panels).
export type AuraHead = {
    palette: number,
    speed: number,
    strong: boolean,
    far: boolean,
}

-- AuraHed (chips/aurahed, action 0x43): the head it fires; StreamHd's
-- `stream` fires five, counting their palettes up, the last strong.
export type AuraHeadSpec = {
    id: string?,
    head: AuraHead,
    stream: boolean?,
}

-- MagCoil (chips/magcoil, action 0x44): the magnet's ticks.
export type MagCoilSpec = {
    id: string?,
    ticks: number,
}

-- A dragon (lib/dragons, a row of the original's tables by the chip's
-- subtype): its speed (16.16 a tick), palette, hit modifier and spark,
-- the panel it leaves behind (none: false), and its body segments'
-- delays.
export type DragonVariant = {
    speed: number,
    palette: number,
    hit_mod: number,
    spark: Spark,
    panel: PanelType | false,
    delays: { number },
}

-- A dragon chip (lib/dragons, action 0x51).
export type DragonSpec = {
    id: string?,
    dragon: DragonVariant,
}

-- A honey bee (chips/rskyhny/bee): its palette and its speeds across and
-- along the field (16.16 a tick).
export type HoneyBee = {
    palette: number,
    speed: number,
    turn_speed: number,
}

-- RskyHny (chips/rskyhny, action 0x39): the bees; `drags` (no chip) the
-- first release clears the navi's drag step instead of sending a bee.
export type HoneySpec = {
    id: string?,
    bee: HoneyBee,
    drags: boolean?,
}

-- The sun beam's look (chips/gundels/beam): its sprite and palette.
export type SunBeam = {
    sprite: SpriteAsset,
    palette: number,
}

-- GunDelSol (chips/gundels, action 0x37): the gun the navi holds, how
-- long it fires, the beam in the shade and in the sun, and whether its
-- hits cover two columns (GunDelEX).
export type GunDelSolSpec = {
    id: string?,
    gun: AttachmentLook,
    firing_ticks: number,
    beam: SunBeam,
    beam_in_sun: SunBeam,
    wide: boolean?,
}

-- ---- DblBeast's beasts (chips/dblbeast/beast) -------------------------------

-- A beast's state: the controller waiting on it, the damage word its hits
-- add theirs to, the next target, the targets, a missed attack, the
-- enemy's bodies' panels (packed y << 4 | x), its attachments.
export type DblBeastState = {
    controller: Object?,
    base: number,
    next: number,
    count: number,
    missed: boolean,
    targets: { number },
    held: Object?,
    held2: Object?,
}

-- An attack's pattern: the panels to attack from around a target, as
-- {flip, dx, dy} (dx toward the enemy).
export type DblBeastPattern = { { number } }

-- A beast's hit: on the panel `ahead` panels in front (or a given one), its
-- region, spark, collision type and damage (added to the base word), and
-- height.
export type DblBeastHit = {
    ahead: number,
    region: Region,
    spark: Spark,
    is: CollisionType,
    damage: number,
    z: number?,
}
