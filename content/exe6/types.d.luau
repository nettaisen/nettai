-- Types shared across the exe6 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/exe6/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API is in content/nettai/core.d.luau.)

-- A flying shot's variant (objects/flying_shot, the original's
-- `byte_80C6038` rows): what it is and hits, its hit modifier, element and
-- secondary elements, hit spark (nil: none), look and shadow, speed (16.16
-- a tick, forward), range (panel centers it passes before it ends), the
-- status its hits carry; it lights its panel (`highlight`), is a
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
    status: Status?,
    highlight: boolean?,
    obstacle: boolean?,
    panel_spark: boolean?,
    launch_sound: Sound?,
    end_effect: Effect?,
}

-- What a flying shot is launched with (objects/flying_shot): its variant,
-- the ticks it waits before it sets off, whether it falls, its palette,
-- the damage word, and a thrown obstacle's kind | animation << 4.
export type FlyingShot = {
    variant: FlyingShotVariant,
    wait: number?,
    falls: boolean?,
    palette: number?,
    damage: number,
    -- A thrown obstacle: its look and animation (the original's ExtraVars
    -- word, kind | animation << 4).
    obstacle: AbsorbedLook?,
    obstacle_anim: number?,
}

-- The state of VarSwrd's and NeoVari's actions (lib/swords/vari.luau).
export type VariSwordState = {
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
    -- The chip each sequence gives, and Sword (what A let go of, or the
    -- time running out, gives): the attack becomes the pick's (its chip,
    -- which the Beast rush reads for its lock-on, and its action).
    picks: { Chip },
    sword: Chip,
    -- A random pick is `picks` by `rng % random`, a Sword past them.
    random: number,
    -- The pick that doesn't turn into SlashCross's charged slash.
    no_charged: Chip,
    -- What SlashCross's A-charge makes of the chip: `"own"`, its own
    -- action, charged (its pick becomes the charged slash).
    charged: "own",
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
    status: Status,
    final_status: Status?,
    palette: number,
    strong: boolean?,
    own_side: boolean?,
}

-- ---- The field objects (chips/rockcube/rock, objects/boulder) ---------------

-- What an element pillar is spawned with (objects/element_pillar; the
-- original's four parameters): lightning (it paralyzes and cracks its
-- panel) rather than flames; how long it stands; the ticks it is left once
-- its owner's action is over; `back`: it stands 8 pixels ahead of its
-- panel's center, less 7 for each step back; `owner_state`: its owner is no
-- navi, and it lasts through that action of the owner's own (by default
-- the navi action its owner runs as it sends the pillar).
export type ElementPillar = {
    lightning: boolean?,
    ticks: number,
    after: number,
    back: number?,
    owner_state: number?,
}

-- AirRaid's plane (chips/airraid, a row of the original's `byte_80D34C0`):
-- its lifetime in ticks, its bombs, its palette and HP.
export type AirRaidPlane = { lifetime: number, bombs: number, palette: number, hp: number }

-- A shock wave's look and timing (chips/wavearm/wave, a row of the
-- original's `byte_80C6B00`): its animation, ticks on a panel before the
-- next rolls on, the panel type it leaves ("cracked" cracks the panel,
-- "broken" breaks it, or cracks it under something); `quick`: it goes 4
-- ticks after sending the next; `steers`: it turns toward an enemy's row.
export type ShockWaveVariant = { anim: number, ticks: number, panel: PanelType?, quick: boolean?, steers: boolean? }

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
    status: Status?,
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

-- ---- The navi chips' navis (content model v2: the earlier navis) ---------

-- A thunderbolt (chips/elecman/thunder): whether it acts while the battle
-- is dimmed (the navi chip's; one that doesn't cracks its panel), and
-- whether it also hits the eight panels around it.
export type Thunderbolt = {
    while_dimmed: boolean?,
    spreads: boolean?,
}

-- A sword wave (chips/slashman/wave): its wind-up and swing in ticks, its
-- speed (16.16 a tick); whether it hits its panel as it winds up, whether
-- its owner can steer it with B, whether it acts while the battle is
-- dimmed.
export type SlashWave = {
    wind_up: number,
    swing: number,
    speed: number,
    hits_early: boolean?,
    steers: boolean?,
    while_dimmed: boolean?,
}

-- A train car (chips/chrgeman/car): whether it is the navi chip's (it
-- waits before it rolls, and acts while the battle is dimmed), its place
-- in the train, the ticks it waits, and its owner's action it is tied to
-- (none: it never checks).
export type ChargeCar = {
    while_dimmed: boolean?,
    place: number,
    wait: number,
    owner_action: number?,
}

-- SpoutMan's water ball and its splashes (chips/spoutman/ball, splash):
-- whether it acts while the battle is dimmed (the navi chip's), and
-- whether it cracks the panel it lands on.
export type WaterBall = {
    while_dimmed: boolean?,
    cracks: boolean?,
}

-- A mark of SpoutMan's geyser (chips/spoutman/mark): the geyser's column
-- (else the trail to it), how long it shows, and whether it counts down
-- while the battle is dimmed.
export type GeyserMark = {
    column: boolean?,
    ticks: number,
    while_dimmed: boolean?,
}

-- A fire blast (chips/blastman/fire): its speed (16.16 a tick), which way
-- it crosses the field, and whether it acts while the battle is dimmed
-- (the navi chip's).
export type FireBlastDirection = "left" | "right" | "up" | "down"

export type FireBlast = {
    speed: number,
    direction: FireBlastDirection,
    while_dimmed: boolean?,
}

-- A panel strike (objects/panel_strike): whether its burst cracks an
-- uncracked panel, the ticks its panel blinks first, its hit modifier, and
-- whether it bursts only on a solid panel.
export type PanelStrike = {
    cracks: boolean?,
    warning: number,
    hit_mod: number,
    solid_only: boolean?,
}

-- SunMoon's meteor (chips/sunmoon/meteor): its look, what it is and hits,
-- whether its landing shakes the camera, and what it does to a panel that
-- holds something.
export type SunMeteor = {
    sprite: SpriteAsset,
    anim: number,
    collision: { is: CollisionType, hits: CollisionType },
    shake: boolean,
    panel: "none" | "crack" | "break",
}

-- What a heat flame is spawned with (chips/heatman/flame): HeatMan's
-- flames and BurnSqr's.
export type HeatFlame = {
    -- It acts and steps its sprite while dimmed (Param1 4: the navi
    -- chip's).
    while_dimmed: boolean,
    -- How long it lasts (Param2).
    ticks: number,
    -- What it is: an attack, or (Param3 set) a thrown attack.
    is: CollisionType,
}

-- What a riding hit is spawned with (navis/slashman/riding_hit): its
-- collision types, hit modifier and spark, and whether it goes on after
-- hitting a body.
export type RidingHit = {
    is: CollisionType,
    hits: CollisionType,
    hit_mod: number,
    spark: Spark?,
    keeps_going: boolean,
}

-- What EraseMan's beam is spawned with (chips/eraseman/beam).
export type EraseBeam = {
    -- The slash's aim: 0 up-forward, 1 forward, 2 down-forward.
    aim: number,
    -- How long it stands.
    ticks: number,
    -- A navi's own segment (EDeletBm's).
    navis: boolean,
}

-- How a dust storm blows (lib/instant/dust_storm, the original's Param2 and
-- Param3): for `ticks` ticks, while its user runs `tied` (nil: untied, its
-- hits landing while dimmed; Count's rain).
export type DustStorm = { ticks: number, tied: Action? }

-- How Count's lance strikes (chips/count/lance, the original's
-- Param2 and Param3): at once and while dimmed (`while_dimmed`, his chip's:
-- the game's Param3 2), or after `warn_ticks` ticks of its panel
-- highlighted, standing still while dimmed.
export type CountLance = {
    while_dimmed: boolean,
    warn_ticks: number,
}

-- What a whirlwind is spawned with (TenguCross Beast's, its form's
-- whirlwind module): whether its hits come all at once, how long it stands
-- and its animation.
export type Whirlwind = { at_once: boolean, ticks: number, anim: number }

-- What a surge of water is spawned with (SpoutCross Beast's, its form's
-- surge module): how long it lasts and how often its region comes back, in
-- ticks.
export type AquaSurge = { ticks: number, beat: number }

-- What a drip shower is spawned with (navis/spoutman/drip_shower): its
-- turns and the ticks a quarter lasts; and whether it lasts only while its
-- owner stays in his action.
export type DripShower = { turns: number, quarter_ticks: number, follows_action: boolean }

-- How GroundMan's rocks drop (chips/grndman/rock): the ticks its panel
-- blinks first, whether it acts while dimmed, whether it cracks the panel
-- it lands on, and silently.
export type GroundRockDrop = { warning: number, while_dimmed: boolean, cracks: boolean, silent: boolean }

-- The looks of the EXE6 modules other games reuse (each module's `make_kind`).

-- EXE6's fields of the engine's chips, forms and navis: its rules'
-- extensions (`extends`), which the engine's specs take untyped
-- (`[string]: any`). A module that wants them typed casts to these.
--
-- A chip's: its Beast Out lock-on mode, and whether it goes through the
-- Beast rush (`rush = false`: a mode without the rush, as some records
-- have; rules/beast's); a dark chip's substitute without bug frags,
-- and what its use adds to the user's HP bug (rules/dark_chips's).
export type Exe6ChipFields = {
    beast: { lockon: Lockon?, rush: boolean? }?,
    dark_substitute: Chip?,
    hp_bug: number?,
}

-- A form's: its kind (rules/forms/kind), the navi a Cross (and one in
-- Beast Out) is made with, and the animation a change into a Cross lets the
-- navi go of (rules/forms's; whose game's form it is, the Beast's
-- roar, is its `version`, the engine's field); a Cross's form in Beast Out, the Cross special's buster volley
-- in it (the original stores the form's number where it meant 6), and
-- whether its charged sword runs inside the Beast Out rush (SlashCross in
-- Beast Out; rules/beast's); the custom screen dealing one more chip
-- for each screen spent in it, up to three (ChargeCross), or the scrap
-- button (DustCross; rules/cross's).
export type Exe6FormFields = {
    kind: ("cross" | "beast" | "cross_beast" | "beast_over")?,
    cross_of: NaviDef?,
    cross_release_anim: number?,
    beast: FormDef?,
    special_volley: number?,
    charged_sword_rush: boolean?,
    extra_chips: boolean?,
    scrap_button: boolean?,
}

-- A navi's forms by game: the Crosses by their number on the custom
-- screen, Beast Out and Beast Over (its `forms.gregar`, `forms.falzar`),
-- which EXE6's rules and exe6-compat read.
export type FormSet = { crosses: { FormDef }, beast_out: FormDef?, beast_over: FormDef? }
