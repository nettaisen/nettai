-- The battle engine's content API, as Luau content sees it. A definitions
-- file for editors and the type checker (luau-lsp:
-- `--definitions=content/bn6/core.d.luau`); the engine implements it in
-- crates/bn6-luau/src/bind.rs over the Rust `CoreApi` trait
-- (crates/bn6-content-api), and docs/design/scripting.md describes it.
--
-- How content runs. A content pack's scripts are modules next to the data
-- they implement; the data registers them (docs/design/content-pack.md):
--
--   objects/NAME/object.toml  [kind] pool, index, script   an object kind:
--       the module exports `state` and `update(me)`
--   chips/NNN-name/chip.toml  script = "chip.luau"          the chip's action:
--       `state` and `update(me, s)`; for a dimming chip (action 0x15)
--       `dimming_chip(user, spec)`, for a navi chip (0x1B) `navi_chip(user,
--       controller, spec)`
--   navis/00-megaman/weapons/NN-name/weapon.toml            a weapon routine:
--       `setup(navi) -> action`, and `state`/`update` for its `action`
--
-- Functions keep nothing between calls: what an object or action keeps
-- lives in its declared `state` (typed fields the engine stores and
-- snapshots), or in the engine's own fields below. Module-level variables
-- can't be assigned and module tables are frozen.
--
-- Every call acts at once: an object spawned now runs later this tick,
-- right after its spawner; a hit resolves when the later of two objects
-- removes its collision. Nothing is batched.
--
-- Numbers: every engine value is an integer. Scripts compute with plain
-- numbers (exact up to 2^53); when a number goes into the engine it must be
-- an integer (a fraction is an error) and a field store wraps it to the
-- field's width, as the game's byte and halfword stores do. So a u16 timer
-- set to -1 reads back 0xFFFF. Positions are `Vec3` values: three i32 in
-- 16.16 fixed point, wrapping on + and -. Use `//` (floor division) or the
-- `int` library, never `/`.
--
-- Names, not numbers: flags, timers, buttons, panel types and lifecycle
-- states are strings ("using_action", "paralyze", "a", "cracked",
-- "destroy"); the engine's bit values and offsets stay in the engine. The
-- original's numbers that are the game's own data (sound ids, animation
-- numbers, collision types, hit regions, panel flag words it matches as
-- whole words) are the scripts' constants, named where they're defined.

export type Pool = "actor" | "attack" | "effect"
export type Lifecycle = "init" | "update" | "destroy"
export type Shadow = "hidden" | "ground" | "with_sprite"
export type ActorType = "virus" | "navi" | "player"
export type Emotion = "normal" | "tired" | "full_synchro" | "angry" | "worn_out"
export type PanelType =
    "missing"
    | "broken"
    | "normal"
    | "cracked"
    | "poison"
    | "holy"
    | "grass"
    | "ice"
    | "volcano"
    | "road_up"
    | "road_down"
    | "road_left"
    | "road_right"
-- Status flags of an object with collision data.
export type StatusFlag =
    "guard"
    | "invisible"
    | "submerged"
    | "invulnerable"
    | "air_shoe"
    | "float_shoe"
    | "moving"
    | "dead"
    | "flashing"
    | "flinching"
    | "paralyzed"
    | "sliding"
    | "blind"
    | "immobilized"
    | "confused"
    | "frozen"
    | "super_armor"
    | "undershirt"
    | "move_complete"
    | "drag"
    | "anger"
    | "using_action"
    | "affected_by_ice"
    | "bubbled"
-- Status timers of an object with collision data.
export type StatusTimer =
    "paralyze" | "confuse" | "blind" | "immobilize" | "flash" | "submerged" | "invulnerable" | "freeze" | "bubble"
-- A navi's action requests.
export type Request =
    "buster"
    | "charged_shot"
    | "chip"
    | "charged_chip"
    | "back_special"
    | "forced_charged_shot"
    | "revert_form"
    | "anti_damage_triggered"
    | "anti_sword_triggered"
    | "cut_in"
    | "turn_l"
    | "turn_r"
    | "form_change"
    | "body_guard_triggered"
    | "alt_chip"
    | "a_held"
    | "b_held"
    | "stun_strike"
    | "select_special"
    | "cross_change"
    | "cross_death"
    | "mode9_a"
    | "cross_special"
    | "volley"
    | "weakness_hit"
-- A navi's state bits.
export type NaviStateFlag =
    "controllable"
    | "chip_in_progress"
    | "form_change"
    | "reverting_form"
    | "no_charge"
    | "can_turn"
    | "trap_armed"
    | "changing_cross"
    | "cross_knockout"
    | "crossed"
    | "volley"
    | "uninterruptible"
    | "cross_breaking"
    | "form_change_sprite_held"
    | "heat_trap"
export type Key = "a" | "b" | "select" | "start" | "right" | "left" | "up" | "down" | "r" | "l"

-- A sprite: "CC-II" in hex, its folder in the pack's graphics/sprites.
export type SpriteId = string

-- A 16.16 fixed-point vector (x right, y down the field, z up). Immutable.
declare class Vec3
    x: number
    y: number
    z: number
    function __add(self, other: Vec3): Vec3
    function __sub(self, other: Vec3): Vec3
end

declare Vec3: {
    -- From raw 16.16 components (each wrapped to i32).
    new: (x: number, y: number, z: number) -> Vec3,
    -- From whole pixels.
    px: (x: number, y: number, z: number) -> Vec3,
    zero: Vec3,
}

-- How an object's sprite is drawn, and its animation.
declare class Sprite
    palette: number
    hflip: boolean
    vflip: boolean
    shadow: Shadow
    -- Drawn solid white (a hit flash).
    white: boolean
    color_shader: number
    -- Blended at alpha/16; nil = not blended.
    alpha: number?
    -- Mosaic blocks of n + 1 pixels; nil = none.
    mosaic: number?
    -- Priority against the background layers (0 = frontmost).
    priority: number
    -- Parts not drawn: bit 31 - i hides part i.
    hidden_parts: number
    -- The animation playing (read-only).
    animation: number
    -- The current frame's flags (0x80 last, 0x40 loop), on the tick the
    -- frame ends (read-only).
    frame_flags: number
    -- The animation has played to its end (read-only).
    finished: boolean
    -- `sprite_load`: load a sprite (resets the animation and the look) and
    -- let it animate (the object's `no_sprite_update` off).
    function load(self, id: SpriteId): ()
    -- Start an animation from its first frame.
    function set_animation(self, anim: number): ()
    -- Advance the animation one tick (no gating).
    function step(self): ()
    -- The game's flip value: bit 0 horizontal, bit 1 vertical.
    function set_flip(self, flip: number): ()
end

-- An object's collision registration.
declare class Collision
    -- Region shape.
    region: number
    -- Hit spark effect (0xFF = none).
    hit_effect: number
    status_base: number
    -- Bug code (low byte) and argument (high byte).
    bugs: number
    hit_mod_base: number
    -- The damage it deals.
    self_damage: number
    -- What the last resolution hit (read-only).
    hit_flags: number
    -- The damage taken this window (read-only).
    final_damage: number
    -- Register on the region's panels, clearing the last results.
    function present(self): ()
    -- Unregister, resolving hits against what is registered there.
    function remove(self): ()
    function free(self): ()
end

-- A handle to an object slot. Like the game's pointers, it can outlive the
-- object (and then names whatever takes the slot next).
declare class Object
    pool: Pool
    -- Which behavior within the pool (read-only).
    index: number
    -- Setting it changes only the lifecycle state (the game's byte store);
    -- `set_lifecycle` also starts from the first action and phase.
    lifecycle: Lifecycle
    -- The object's action and phase; their meaning is the kind's.
    action: number
    phase: number
    phase_init: number
    panel_x: number
    panel_y: number
    -- The panel a move is going to.
    future_panel_x: number
    future_panel_y: number
    -- 0 = left side, 1 = right side.
    alliance: number
    -- Facing is alliance ~ flip; see `facing`.
    flip: number
    -- +1 facing right, -1 facing left (read-only).
    facing: number
    -- The requested animation, and the one the sprite has.
    anim: number
    anim_loaded: number
    element: number
    timer: number
    timer2: number
    hp: number
    max_hp: number
    -- The damage word: damage (with flag bits) in the low half, the high
    -- half (the counter byte in its low byte) in stamina.
    damage: number
    stamina: number
    name_id: number
    -- Holds an object's sprite (and more, by kind).
    prevent_anim: number
    pos: Vec3
    vel: Vec3
    related1: Object?
    related2: Object?
    -- The slot is in use (read-only).
    active: boolean
    visible: boolean
    run_while_paused: boolean
    -- Keeps updating while the battle is dimmed.
    run_while_dimmed: boolean
    no_sprite_update: boolean
    sprite: Sprite
    collision: Collision
    -- The kind's declared state (cast it to the kind's State type).
    state: any

    -- Navis (actors) only. The attack in progress: its phase and whether
    -- the phase's entry ran; its variant (a chip's subtype) and chip.
    step: number
    step_init: number
    variant: number
    chip: number
    attack_element: number
    attack_damage: number
    -- The attack's hitboxes' counter byte.
    hit_param: number
    charged: number
    -- Lockout the attack applies when it ends.
    attack_lockout: number
    -- The Atk+ / cross bonus.
    extra: number
    special_source: number
    -- Which set_attack kind started it (read-only).
    attack_kind: number
    -- 1 while the action runs inside a form's action wrapper.
    beast_lockon: number
    -- A per-action word some actions keep.
    marker: number
    -- The actor data (read-only unless noted).
    actor_type: ActorType
    ai_index: number
    -- The sprite overlay attached for the current attack (writable).
    overlay: Object?
    -- Writable links: the Beast Out lock-on marker, the charge glow, the
    -- Full Synchro aura.
    lockon_marker: Object?
    charge_glow: Object?
    full_synchro_aura: Object?
    -- The charge: level 0 none, 1 charging, 2 full; source 0 none, 1 A, 2 B.
    charge_level: number
    charge_source: number
    charge_counter: number
    -- Writable: a buffered auto-step's direction, the chip lockout, the
    -- B+Back special's cooldown.
    buffered_move: number
    chip_lockout: number
    back_special_cooldown: number
    -- The weapon routines the navi's buttons run (0xFF = none).
    buster_routine: number
    charge_shot_routine: number
    back_special_routine: number
    a_charge_routine: number
    alt_a_charge_routine: number
    mode9_a_routine: number

    -- Spawn parameter n (1 to 4: the game's Param1..Param4).
    function param(self, n: number): number
    function set_param(self, n: number, value: number): ()
    -- Enter a lifecycle state from its first action and phase (the game's
    -- word store).
    function set_lifecycle(self, state: Lifecycle): ()
    -- Switch to an action from its first phase.
    function set_action(self, action: number): ()
    -- Free the slot now.
    function free(self): ()
    -- `object_genericDestroy`: release panel reservations and collision,
    -- then free.
    function destroy(self): ()
    -- `sub_8011044`: what an object with a navi's NameID takes down when it
    -- goes (for most, its form overlay in related2).
    function death_hook(self, name_id: number): ()

    -- `object_setAnimation`: request an animation, restarting it even if
    -- it is current.
    function set_animation(self, anim: number): ()
    -- `object_updateSprite`: load a requested animation and step the
    -- sprite, unless paused, dimmed (and not running while dimmed) or held.
    function update_sprite(self): ()
    -- `object_updateSpriteTimestop`: the same, skipped only while paused.
    function update_sprite_while_dimmed(self): ()
    -- `sub_801BCD0`: the same, paused or not.
    function step_sprite(self): ()
    -- `object_updateSpritePaused`: the same, paused or not, but not while
    -- dimmed (and whatever `no_sprite_update` says).
    function update_sprite_while_paused(self): ()
    -- The sprite's attach point n, in pixels facing the object's way.
    function attach_point(self, n: number): (number, number)

    function set_coordinates_from_panel(self): ()
    function set_panel_from_coordinates(self): ()
    function update_collision_panels(self): ()
    -- Onto the destination panel of a move (panel, reservation,
    -- coordinates, collision).
    function snap_to_future_panel(self): ()
    function reserve_panel(self, x: number, y: number): boolean
    function unreserve_panel(self, x: number, y: number): boolean
    function release_reservations(self): ()
    -- `sub_800E618`: could it step onto (x, y)?
    function can_step(self, x: number, y: number): boolean
    -- `sub_800E680`: could it stand on (x, y), whichever side owns it?
    function can_stand_any_side(self, x: number, y: number): boolean

    function status(self, flag: StatusFlag): boolean
    function set_status(self, flag: StatusFlag, on: boolean): ()
    function status_timer(self, timer: StatusTimer): number
    function set_status_timer(self, timer: StatusTimer, ticks: number): ()

    function create_collision(self): boolean
    -- Set up the registration from the object (side, panel, element,
    -- damage word) with collision types by index.
    function setup_collision(self, self_type: number, target_type: number, hit_mod: number): ()
    function reset_collision_types(self, self_type: number, target_type: number, hit_mod: number): ()
    -- `object_spawnCollisionEffect`: the hit spark after a hit (one RNG
    -- draw when it shows).
    function hit_spark(self): ()
    -- `object_highlightCurrentCollisionPanels`: highlight the panels its
    -- collision region covers (drawn only).
    function highlight_collision_panels(self): ()

    -- Navis only.
    function attack_param(self, n: number): number
    function set_attack_param(self, n: number, value: number): ()
    function request(self, flag: Request): boolean
    function set_request(self, flag: Request, on: boolean): ()
    function navi_state(self, flag: NaviStateFlag): boolean
    function set_navi_state(self, flag: NaviStateFlag, on: boolean): ()
    -- The buttons, and the record kept while the battle is dimmed.
    function held(self, key: Key): boolean
    function pressed(self, key: Key): boolean
    function released(self, key: Key): boolean
    function dimmed_held(self, key: Key): boolean
    function dimmed_pressed(self, key: Key): boolean
    function dimmed_released(self, key: Key): boolean
    -- `object_setDefaultCounterTime`.
    function open_counter_window(self): ()
    -- `sub_801056A`, after each phase of an attack.
    function check_reactive_abort(self): ()
    -- `object_exitAttackState`: back to idle with animation 0.
    function exit_attack(self): ()
    -- `sub_801171C`: back to idle, the animation as it is.
    function end_attack(self): ()
    -- `object_setAttack0..5`: start an action; kind 1 buster, 2 chip or
    -- charged shot, 3 special, 4 move.
    function set_attack(self, action: number, kind: number): ()
    function reset_attack_links(self): ()
    -- `sub_800FA54`: the held direction (1 up, 2 down, 3 back, 4 forward;
    -- 0 none).
    function held_direction(self): number
    -- `sub_800F964`: where a step toward dir would go, if it can.
    function step_target(self, dir: number): (number?, number?)
    -- `sub_80116AE`: start a step toward dir.
    function start_move(self, dir: number): ()
    -- `object_canMove`: not immobilized, sliding or moving.
    function can_move(self): boolean
    -- `sub_801265A`: the buster's damage.
    function buster_damage(self): number
    -- The obstacles it absorbed, oldest first.
    function absorbed(self): { { kind: number, anim: number } }
    function push_absorbed(self, kind: number, anim: number): boolean
    function pop_absorbed(self): (number?, number?)
end

-- A side's navi stats (read-only).
declare class Navi
    -- Fighting in the sun.
    sun: boolean
    form: number
    navi: number
    navi_variant: number
    -- The base form's element.
    element: number
    attack: number
    rapid: number
    charge: number
    mood: number
    beast_out_counter: number
    max_base_hp: number
    chip_recovery: number
    -- NaviCust: the buster's spread shot, the charged shot's kind, and the
    -- buster's blank and charged slots (of 16).
    buster_shot: number
    charge_shot_kind: number
    buster_blanks: number
    buster_charged: number
    -- The form is a Beast form, Beast Over.
    beast: boolean
    beast_over: boolean
end

-- A one-tick hit region (`battle.hitbox`).
export type HitboxSpec = {
    panel_x: number,
    panel_y: number,
    element: number,
    z: number?,
    -- Region shape.
    region: number,
    hit_effect: number,
    -- Collision type indices.
    target: number,
    self_type: number,
    -- The damage word: damage (with flag bits) and the counter byte.
    damage: number,
    stamina: number?,
    hit_mod: number?,
    status: number?,
    bug: number?,
    bug_arg: number?,
}

-- A side's defensive-chip record.
export type LinkedChip = {
    chip: number,
    bonus: number,
    -- The damage word.
    damage: number,
    owner: Object?,
    object: Object?,
}

export type Panel = { kind: PanelType, alliance: number, home: number }
export type Column = { home: number, timer: number }
export type NaviRecord = { actor_type: ActorType, ai_index: number }

-- What a dimming chip's controller is spawned with: the user's attack.
export type DimmingChipSpec = {
    element: number,
    params: { number },
    -- The damage word (damage | hit parameter << 16).
    damage: number,
    -- The chip, and the bonus the telop shows with it.
    chip: number,
    bonus: number,
}

-- What a navi chip's navi is spawned with.
export type NaviChipSpec = {
    panel_x: number,
    panel_y: number,
    element: number,
    params: { number },
    -- The damage word, bonus included.
    damage: number,
}

declare battle: {
    -- The battle is dimmed (battle flag 4): objects that don't run while
    -- dimmed stand still.
    dimmed: () -> boolean,
    paused: () -> boolean,
    -- `battle_isBattleOver`: a side has no navi left, or time is up.
    over: () -> boolean,
    -- The same as the routines that read its Z flag see it: true only once
    -- time is up (a KO reads false).
    time_up: () -> boolean,
    link: () -> boolean,
    mode: () -> number,
    panel_pattern: () -> number,
    -- Every navi is in (the intro's bit 2).
    navis_in: () -> boolean,
    -- Presentation only: the side the simulation's perspective is.
    local_side: () -> number,
    -- Sound for both players, or for one side's player (output only).
    play_sound: (id: number) -> (),
    play_sound_for: (side: number, id: number) -> (),
    navi: (side: number) -> Navi,
    emotion: (side: number) -> Emotion,
    -- Unless the navi's emotion is held.
    set_mood: (side: number, mood: number) -> (),
    player: (side: number) -> Object?,
    -- A side's combatants still in, in slot order.
    alive_actors: (side: number) -> { Object },
    -- `GetRNG2` (one draw), `GetPositiveSignedRNG2` (bit 31 cleared).
    rng: () -> number,
    rng_positive: () -> number,
    -- `AddRandomVarianceToTwoCoords`: x and z by up to mask/2 pixels.
    jitter: (mask: number, pos: Vec3) -> Vec3,
    -- A side's hand: the chip at i (0xFFFF = none), and the cursor.
    hand_chip: (side: number, i: number) -> number,
    hand_cursor: (side: number) -> number,
    advance_hand: (side: number) -> (),
    linked: (side: number) -> LinkedChip,
    set_linked: (side: number, record: LinkedChip) -> (),
    clear_linked: (side: number) -> (),
    fill_custom_gauge: () -> (),
    bump_side_stat: (side: number, index: number, n: number) -> (),
    navi_record: (name_id: number) -> NaviRecord?,
    -- Spawn into the pool's lowest free slot; it runs later this tick,
    -- right after the spawner. nil if the pool is full.
    spawn: (pool: Pool, index: number, pos: Vec3?, params: { number }?) -> Object?,
    -- The same for a content kind, by name (its folder under objects/).
    spawn_kind: (name: string, pos: Vec3?, params: { number }?) -> Object?,
    -- The generic one-shot effect (effect object #0).
    effect: (pos: Vec3, id: number, flip: number?, palette_add: number?, priority: number?) -> Object?,
    -- The one-tick hit region (attack object #3).
    hitbox: (owner: Object, spec: HitboxSpec) -> Object?,
    -- A hit spark (effect object #4).
    spark: (owner: Object, pos: Vec3, id: number) -> Object?,
}

declare field: {
    valid: (x: number, y: number) -> boolean,
    -- A panel's center, 16.16.
    center: (x: number, y: number) -> (number, number),
    -- A panel's cached flags word (0 off the field).
    flags: (x: number, y: number) -> number,
    -- The flags word has every `require` bit and no `forbid` bit.
    check: (x: number, y: number, require: number, forbid: number) -> boolean,
    panel: (x: number, y: number) -> Panel?,
    column: (x: number) -> Column,
    set_column_timer: (x: number, ticks: number) -> (),
    set_alliance: (x: number, y: number, side: number) -> (),
    set_type: (x: number, y: number, kind: PanelType) -> (),
    -- `object_crackPanel`: crack a solid panel, or break a cracked,
    -- unoccupied one.
    crack: (x: number, y: number) -> boolean,
    -- `object_breakPanel`: break a solid, unoccupied panel, cracked or not.
    break_panel: (x: number, y: number) -> boolean,
    -- `object_breakPanel_dup1`: break a solid panel, or crack it when
    -- something stands on it (not a neutral object); true only when it
    -- broke.
    shatter: (x: number, y: number) -> boolean,
    -- `object_isPanelSolid`: something can stand on it (not missing or
    -- broken).
    solid: (x: number, y: number) -> boolean,
    -- Drawn only.
    highlight: (x: number, y: number) -> (),
}

-- The dimming service: the steps of a dimming chip's controller, which its
-- update calls in its routine's order.
declare dimming: {
    -- `object_timefreezeBegin` (the controller's init).
    begin: (controller: Object) -> (),
    -- `object_dimScreen`.
    dim_screen: (controller: Object) -> (),
    -- `object_drawChipName`: the telop, the wait for a cut-in, then the
    -- effect (the next action).
    show_telop: (controller: Object) -> (),
    -- `sub_800BBA8`: a hidden chip's telop.
    show_hidden_telop: (controller: Object) -> (),
    -- A navi chip's: `sub_800BDB2` (AntiNavi) and `sub_800BA8A` (its telop).
    check_anti_navi: (controller: Object, chip: number) -> (),
    show_navi_telop: (controller: Object, chip: number) -> (),
    -- `object_undimScreen`.
    undim_screen: (controller: Object) -> (),
    -- `object_timefreezeEnd` (the controller's destroy state).
    finish: (controller: Object) -> (),
    -- `sub_80E1352` / `sub_80E13DC`: a navi chip's user vanishes and comes
    -- back.
    hide_user: (user: Object) -> (),
    show_user: (user: Object) -> (),
}

declare navi_chip: {
    -- The navi is done: its controller moves on.
    navi_left: (controller: Object) -> (),
}

-- Exact integer helpers.
declare int: {
    -- Wrap to a width.
    u8: (x: number) -> number,
    u16: (x: number) -> number,
    u32: (x: number) -> number,
    i8: (x: number) -> number,
    i16: (x: number) -> number,
    i32: (x: number) -> number,
    -- 32-bit shifts: arithmetic right, left (wrapping), logical right.
    asr: (x: number, n: number) -> number,
    shl: (x: number, n: number) -> number,
    lsr: (x: number, n: number) -> number,
    -- Division truncating toward zero (the ARM's), and its remainder.
    tdiv: (a: number, b: number) -> number,
    tmod: (a: number, b: number) -> number,
    -- Wrapping 32-bit product.
    mul32: (a: number, b: number) -> number,
}

-- ---- The pack's data -----------------------------------------------------
--
-- `data` is the content pack's data, frozen, as the pack's files write it
-- (docs/design/content-pack.md): field names as in the files, enums as
-- names, sprites as "CC-II", keyed by the entities' ids. Only the fields
-- scripts read are typed here; the rest are there too.

-- What an attachment looks like and where it sits on its owner.
export type AttachmentKind = {
    -- The attachment's number (its first spawn parameter).
    id: number,
    sprite: SpriteId,
    palette: number,
    -- Pixels raised by (subtracted from y and z).
    lift: number,
    -- The owner's sprite attach point it follows (nil: its origin).
    attach_point: number?,
}

-- Which sun beam look (data.objects.sun_beam_looks) and palette GunDelSol
-- shows.
export type SunBeamLook = { look: number, palette: number }

-- GunDelSol's data, in its chip's folder.
export type GunDelSol = {
    -- Ticks of hits under the beam.
    firing_ticks: number,
    -- The sun beam in the shade and in the sun.
    beam: SunBeamLook,
    beam_in_sun: SunBeamLook,
    -- The gun attached to the user.
    gun: AttachmentKind,
}

-- A chip's record and its own data.
export type ChipData = {
    id: number,
    name: string,
    element: string,
    family: string,
    class: string,
    flags: { string },
    hit_param: number,
    action: number,
    subtype: number,
    beast_lockon: boolean,
    params: { number },
    lockout: number,
    lockon_mode: number,
    damage: number,
    gun_del_sol: GunDelSol?,
    script: string?,
}

export type NaviData = {
    id: number,
    name: string,
    sprite: SpriteId,
    element: string,
    buster_bonus: number,
    merge_height: number,
}

export type FormData = {
    id: number,
    name: string,
    sprite: SpriteId,
    element: string,
    buster_bonus: number,
}

export type RockKind = {
    id: number,
    anim: number,
    hp: number,
    element: string,
    debris_palette: number,
    break_sound: number,
    name_id: number,
}

export type BodyOverlay = { id: number, sprite: SpriteId, in_front: { boolean } }

export type WeaponData = { id: number, name: string, action: number?, script: string }

-- A shock wave's look and timing (attack object #0x16, by its first
-- parameter): ticks on a panel before the next rolls on, and the panel
-- type it leaves ("cracked" cracks the panel, "broken" breaks it).
export type ShockWave = { id: number, sprite: SpriteId, anim: number, ticks: number, panel: PanelType? }

-- An object kind a script implements.
export type ObjectKindData = { pool: Pool, index: number, script: string }

export type PackData = {
    chips: { [number]: ChipData },
    navis: { [number]: NaviData },
    forms: { [number]: FormData },
    weapons: { [number]: WeaponData },
    objects: {
        attachments: { [number]: AttachmentKind },
        rocks: { [number]: RockKind },
        absorbed_sprites: { [number]: SpriteId },
        body_overlays: { [number]: BodyOverlay },
        sun_beam_looks: { [number]: SpriteId },
        kinds: { [string]: ObjectKindData },
        shock_waves: { [number]: ShockWave },
    },
    rules: {
        -- The buster's recovery in ticks (`byte_80209CC`), at
        -- rapid * 6 + min(open panels ahead, 5).
        buster_recovery: { [number]: number },
    },
    -- Hit regions (`PanelOffsetListsPointerTable`) by region number: the
    -- panels a region covers, { dx, dy } from its panel with dx toward the
    -- facing side, in the game's order.
    regions: { [number]: { { number } } },
}

declare data: PackData
