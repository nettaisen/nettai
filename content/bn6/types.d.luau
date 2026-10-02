-- Types shared across the bn6 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/bn6/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API is in content/nettai/core.d.luau.)

-- Where an owner keeps an attached object: its actor overlay slot, its
-- first related object, or its content state's `held` or `held2` field
-- (lib/slot.luau).
export type Slot = "overlay" | "related" | "held" | "held2"

-- What a projectile's hit does to the panel it hits: crack it
-- (`object_crackPanel`), break it (`object_breakPanel_dup2`), or make a
-- solid one a panel of a type (by the shot's side).
export type PanelHit = {
    effect: "crack" | "break" | "set_type",
    left_side: PanelType?,
    right_side: PanelType?,
}

-- A projectile's variant (objects/projectile, the original's `off_80C4C78`
-- rows): what it is and hits, its hit modifier, its element and secondary
-- elements, its hit spark (nil: none), its look (none: not drawn), the
-- status and bug its hits carry (nil or 0: none), what its hit does
-- to the panel, whether it bursts (around what it hits, and over the last
-- two columns when it leaves the field), whether it climbs (a pixel lower
-- down the field and higher on each panel). One a navi's stats can name
-- (a shot program) has a key.
export type ProjectileVariant = {
    id: string?,
    collision: { is: CollisionType, hits: CollisionType },
    hit_mod: number,
    element: string,
    secondary: { string }?,
    spark: Spark?,
    sprite: SpriteAsset?,
    anim: number?,
    status: Status?,
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

-- What a flying shot is launched with (objects/flying-shot): its variant,
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

-- A bullet's variant (objects/bullet, the original's `byte_80C68D4` rows):
-- the region its burst hits on the panel it found, the burst's hit
-- modifier and status (nil: none), the effect it shows (over
-- the region, or once at the bullet when `single`), and its sound.
export type BulletVariant = {
    region: Region,
    hit_mod: number,
    effect: Effect,
    single: boolean?,
    status: Status?,
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

-- An attachment's look (objects/attachment): its sprite, its palette, the
-- pixels it sits raised, the owner's attach point it sits at (none: on the
-- owner's position); `by_owner`: at a fixed offset by the owner's NameID
-- instead (the original's row 0xF).
export type AttachmentLook = {
    sprite: SpriteAsset,
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
    status: Status?,
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
    -- What SlashCross's charged slash makes of it (a step sword dashes in
    -- first).
    charged: ChargedSlash?,
}

-- What SlashCross's charged slash is, of one sword
-- (navis/megaman/forms/slashcross/slashes): the sword wave it sends,
-- and a second one unseen under it (CrosSwrd's); the blade the navi
-- raises (none: it swings its arm alone, without one); the swing's sound;
-- the slash that follows it (DblDream's second); and the lock-on mode the
-- Beast Out rush takes for it (rules/lockon).
export type ChargedSlash = {
    wave: SwordWaveVariant,
    hidden_wave: SwordWaveVariant?,
    blade: AttachmentLook?,
    sound: Sound,
    again: ChargedSlash?,
    lockon: Lockon,
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

-- A rock's variant (chips/rockcube/rock, a row of the original's
-- `byte_80CF934`): its standing animation, HP, element, the palette of the
-- debris it breaks into, the sound it breaks with, and its identity (what
-- it is taken for: a rock, an ice block).
export type RockVariant = {
    -- Its key (`rockcube/rock/cube`): stages name their rocks' variants.
    id: string?,
    anim: number,
    hp: number,
    element: string,
    debris_palette: number,
    break_sound: Sound,
    identity: Identity,
}

-- How a rock comes onto the field (chips/rockcube/rock, the original's
-- third parameter): rising out of its panel (its sprite's first animation), there
-- at once, falling from above, or placed when the round starts (standing
-- from the start, and running while the battle is paused until it stands).
export type RockEntrance = "rise" | "instant" | "fall" | "placed"

-- What a rock is spawned with (chips/rockcube/rock): its variant, the
-- class of field-object slot it takes (0, the default: one of its side's
-- two; 1: its side's one), and its entrance ("rise" by default).
export type RockSpec = {
    variant: RockVariant,
    class: number?,
    entrance: RockEntrance?,
}

-- ---- The dimming chips (chips/wind, chips/guardian, ...) -------------------

-- A Wind's or Fan's fan (chips/wind/fan): its palette; `pulls`: it blows
-- Fan's pulling gust from the far column rather than Wind's pushing one
-- from the enemy's front column; ticks from becoming its side's wind to
-- the first gust; HP; its identity (what it is taken for) and how it looks
-- absorbed (objects/absorbed-obstacle's `look`).
export type FanVariant = {
    palette: number,
    pulls: boolean,
    wait: number,
    hp: number,
    identity: Identity,
    absorbed: AbsorbedLook,
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
-- the ticks between its drains, HP, lifetime, and identity (what it is
-- taken for).
export type AnubisVariant = {
    anim: number,
    period: number,
    hp: number,
    lifetime: number,
    identity: Identity,
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

-- A patch card (the Japanese games' Modification Card; a record of type
-- "patch-card", rules/patch-cards): its MB and its effects
-- (rules/patch-cards/cards.luau's constructors). Its name is the locales'
-- (locales/<lang>.toml's `records`, by its key).
export type PatchCardEffect = {
    kind: string,
    amount: number?,
    element: ElementName?,
    weapon: Weapon?,
    variant: ProjectileVariant?,
    barrier: Barrier?,
    gauge: string?,
    on: boolean?,
    trail: string?,
    level: number?,
    blanks: number?,
    charged: number?,
    status: string?,
    -- The card shows it as a bug.
    bug: boolean?,
}
export type PatchCard = {
    -- Its key ("patch-card/<name>"): compat/records.toml's `patch_cards`
    -- give the original's numbers.
    id: string,
    mb: number,
    effects: { PatchCardEffect },
}

-- A barrier (lib/barriers): `sub_801A7CC`'s row (how it behaves, its HP,
-- the damage a hit must reach to wear it, its timer (0xFFFF: none), the
-- element that breaks it), and its visual (`byte_80E0A14`'s look: sprite,
-- animation, palette, the owner's attach point it sits at, the sprite
-- parts it hides; LifeAur's aura hides part 6 but for the local MegaMan),
-- with its sounds as it rises (and grows back) and as it goes down.
export type Barrier = {
    -- The key a navi's stats name it by (its first barrier).
    id: string?,
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
-- its identity (what it is taken for) and palette,
-- how long it plays and pauses, its tune, and the effect it has every
-- third tick of playing (`off_80D4298`).
export type Instrument = {
    identity: Identity,
    palette: number,
    play_ticks: number,
    pause_ticks: number,
    tune: Sound,
    effect: (me: Object) -> (),
}

-- A sword wave (navis/megaman/forms/slashcross/sword_wave, a row of the
-- original's `byte_80D7F4C`): what it is and hits, its hit modifier and
-- region, its look, the panel centers it passes before it ends and its
-- speed (16.16 a tick, forward), the status and bug its hits carry
-- (none by default). `animates`: it restarts its animation after the last
-- frame and steps it; `highlight`: it lights the panels its region
-- covers; `ground_shadow`: its shadow stays on the ground; `steps`: its
-- sprite also steps after each update (the charged shot's own wave).
export type SwordWaveVariant = {
    collision: { is: CollisionType, hits: CollisionType },
    hit_mod: number,
    region: Region,
    sprite: SpriteAsset,
    anim: number,
    palette: number,
    reach: number,
    speed: number,
    status: Status?,
    bug: number?,
    bug_arg: number?,
    animates: boolean?,
    highlight: boolean?,
    ground_shadow: boolean?,
    steps: boolean?,
}

-- What an element pillar is spawned with (objects/element-pillar; the
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
    identity: Identity,
    class: number,
    collision: { is: CollisionType, hits: CollisionType },
    strong_shake: boolean?,
    allows_bodies: boolean?,
    bursts_when_broken: boolean?,
    linked: boolean?,
}

-- AirRaid's plane (chips/airraid, a row of the original's `byte_80D34C0`):
-- its lifetime in ticks, its bombs, its palette and HP.
export type AirRaidPlane = { lifetime: number, bombs: number, palette: number, hp: number }

-- ColorPt's and DblPoint's points (chips/colorpt): their palette (four a
-- step) and the Atk+ bonus each brings (the chips' parameters).
export type ColorPoint = { palette: number, bonus: number }

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
export type TornadoSpec = { variant: TornadoVariant, status: Status?, ticks: number }

-- A thunder ball (chips/thunder/ball): fast or not, the panels it drifts
-- (12 makes DarkThnd's look and sound), the status its hits inflict and
-- their bug (none by default).
export type ThunderBall = { fast: boolean?, panels: number, status: Status?, bug: number? }

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

-- SpoutMan's geyser (chips/spoutman/geyser): whether it acts while the
-- battle is dimmed (the navi chip's), how long it erupts, and the column
-- its spawner aims it at.
export type Geyser = {
    while_dimmed: boolean?,
    ticks: number,
    target: number,
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

-- A panel strike (objects/panel-strike): whether its burst cracks an
-- uncracked panel, the ticks its panel blinks first, its hit modifier, and
-- whether it bursts only on a solid panel.
export type PanelStrike = {
    cracks: boolean?,
    warning: number,
    hit_mod: number,
    solid_only: boolean?,
}

-- BassAnly's dark ball (chips/bassanly/shot): its level (0 to 3: its
-- speeds), its slant (-1 up, 1 down), and how it homes.
export type DarkBallHoming = "none" | "diagonal" | "seeking"
export type DarkBall = {
    level: number,
    slant: number,
    homing: DarkBallHoming,
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

-- How a gust blows (objects/gust, the original's first parameter):
-- `pulls` turns it back toward its side, a weaker push; `unseen` hides it
-- and lets it blow through any panel.
export type GustHow = { pulls: boolean?, unseen: boolean? }

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
