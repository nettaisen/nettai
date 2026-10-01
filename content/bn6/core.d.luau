-- The battle engine's content API, as Luau content sees it. A definitions
-- file for editors and the type checker (luau-lsp:
-- `--definitions=content/bn6/core.d.luau`); the engine implements it in
-- crates/bn6-luau/src/bind.rs over the Rust `CoreApi` trait
-- (crates/bn6-content-api), and docs/design/scripting.md describes it.
--
-- How content runs (content model v2, docs/design/content-model-v2.md).
-- Every module of a pack loads once, and while it loads it makes
-- definitions with `define.<registry>(spec)` (a chip, a kind, an action, an
-- effect...) and names assets with `asset.<kind>(name)`; a definition is its
-- spec table, frozen, and scripts pass it where the API takes one
-- (`battle.spawn(bomb.kind, pos)`, `me:set_attack(action, 2)`). The
-- definitions and their types are at the end of this file.
--
-- Until the migration ends, the pack's data also registers modules by
-- number (docs/design/content-pack.md), and the numeric API those modules
-- use stays, marked "Deprecated" below and counted per module by
-- bn6-content-check's ratchet (tests/deprecated.txt), which only shrinks:
--
--   objects/NAME/object.toml  [kind] pool, index, script   an object kind:
--       the module exports `state` and `update(me)`; with `actor_list_entry
--       = N`, also `actor_list_entry(spec)` for the actor lists' entries
--       of type N (the field objects a stage places)
--   chips/NNN-name/chip.toml  script = "chip.luau"          the chip's action:
--       `state` and `update(me, s)`; for a dimming chip (action 0x15)
--       `dimming_chip(user, spec)`, for a navi chip (0x1B) `navi_chip(user,
--       controller, spec)`, for an instant chip (0x1C) `instant_chip(user,
--       spec)`
--   navis/00-megaman/weapons/NN-name/weapon.toml            a weapon routine:
--       `setup(navi) -> action`, and `state`/`update` for its `action`,
--       `instant_chip(user, spec)` for its `instant_chip`
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
-- ("finish" is the fourth state of kinds that have one after "destroy":
-- for them "destroy" is a last running phase, and "finish" frees them.)
export type Lifecycle = "init" | "update" | "destroy" | "finish"
export type Shadow = "hidden" | "ground" | "with_sprite"
export type ActorType = "virus" | "navi" | "player"
export type Emotion = "normal" | "tired" | "full_synchro" | "angry" | "worn_out"
-- A side's SELECT or Cross special in progress (battle flag 0x40 mode).
export type SideSpecial = "none" | "select" | "cross"
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
    -- Hit even while the battle is dimmed (ElemTrap's trap).
    | "hit_while_dimmed"
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
    -- Gone from the field while its navi chip's navi acts.
    | "vanished"
export type Key = "a" | "b" | "select" | "start" | "right" | "left" | "up" | "down" | "r" | "l"
-- A navi's drag reaction step.
export type DragStep = "start" | "slide" | "recover"
-- What other objects ask of a field object (an obstacle).
export type ObstacleFlag =
    "destroy"
    | "flinch"
    | "pushed"
    | "thrown"
    | "encased"
    | "removed"
    | "vanish"
    | "absorbed"
    | "absorbed_by_0"
    | "absorbed_by_1"

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
    -- let it animate (the object's `no_sprite_update` off). Deprecated: a
    -- sprite by number ("CC-II"); use an asset (asset.sprite).
    function load(self, sprite: SpriteAsset | SpriteId): ()
    -- Start an animation from its first frame.
    function set_animation(self, anim: number): ()
    -- Advance the animation one tick (no gating).
    function step(self): ()
    -- The game's flip value: bit 0 horizontal, bit 1 vertical.
    function set_flip(self, flip: number): ()
end

-- An object's collision registration.
declare class Collision
    -- The region shape (a region definition; nil, none) and hit spark (a
    -- spark definition; nil, none).
    function set_region(self, region: Region?): ()
    function set_hit_effect(self, spark: Spark?): ()
    -- Region shape, as the engine's byte (`set_region` takes a
    -- definition). Deprecated: setting it to a number.
    region: number
    -- The panel the region is anchored on (`update_collision_panels`
    -- copies the object's).
    panel_x: number
    panel_y: number
    -- Hit spark (0xFF = none), as the engine's byte (`set_hit_effect` takes
    -- a definition). Deprecated: setting it to a number.
    hit_effect: number
    status_base: number
    -- Bug code (low byte) and argument (high byte).
    bugs: number
    hit_mod_base: number
    -- The damage it deals.
    self_damage: number
    -- The counter byte (bits 0-6 counter strength, bit 7 can't counter),
    -- which setup takes from the damage word's high half.
    counter_byte: number
    -- What the last resolution hit (some kinds clear it).
    hit_flags: number
    -- The damage taken this window (read-only).
    final_damage: number
    -- Which way the region's anchor last moved: 0 none, 1 up, 2 down, 3
    -- back, 4 forward, 5 other (read-only).
    direction: number
    -- The directions (bit 1 << the hitter's flip) a guard blocked hits
    -- from this window (read-only).
    guard_dirs: number
    -- The secondary elements (sword 0x80, cursor 0x40, wind 0x20, break
    -- 0x10) of what hit it this window (read-only).
    damage_elements: number
    -- Register on the region's panels, clearing the last results.
    function present(self): ()
    -- Unregister, resolving hits against what is registered there.
    function remove(self): ()
    function free(self): ()
    -- The damage taken this window in an element (0 null, 1 fire, 2 aqua,
    -- 3 elec, 4 wood; 5 the sixth slot).
    function element_damage(self, element: number): number
end

-- A handle to an object slot. Like the game's pointers, it can outlive the
-- object (and then names whatever takes the slot next).
declare class Object
    pool: Pool
    -- Its kind (read-only): compare with a kind definition
    -- (`me.kind == bomb.kind`).
    kind: Kind
    -- Deprecated: which behavior within the pool (read-only); use `kind`.
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
    -- A navi's drag reaction step (the byte other kinds use otherwise).
    drag_step: DragStep
    -- It holds a panel reservation (released when it is destroyed).
    holds_reservation: boolean
    sprite: Sprite
    collision: Collision
    -- The kind's declared state (cast it to the kind's State type).
    state: any
    -- Navis only: the declared state of the content action it is running
    -- (cast it to that action's State type), as another object reaches
    -- into the game's attack variables.
    attack_state: any

    -- Navis (actors) only. The attack in progress: its phase and whether
    -- the phase's entry ran; its variant (a chip's subtype) and chip (its
    -- number, 0 for none; a chip content defines is 0x200 + its handle,
    -- which reads back as that chip).
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
    -- The recovery a shot waits after firing: a word of the attack that
    -- outlives the action, so an action that waits it without writing it
    -- (a thrown obstacle) waits the last shot's.
    recovery: number
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
    -- The weapon routines the navi's buttons run (0xFF = none; the
    -- buster's and the charged shot's writable).
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
    -- `sub_8010DF6`: put on the overlays an actor record adds (a body
    -- overlay, SpoutMan's idle overlay, a beast head), kept in related2
    -- (CircusMan's second one kept by the engine); `arg` 1: their sprites
    -- step even while paused.
    function add_navi_parts(self, actor_type: ActorType, ai_index: number, arg: number): ()
    -- `sub_8011044`: take them off (at their next update).
    function remove_navi_parts(self, actor_type: ActorType, ai_index: number): ()

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
    -- `sub_801BC64`: `update_sprite`'s gating (dimming, holds), but paused
    -- or not.
    function update_sprite_even_paused(self): ()
    -- `object_updateSpritePaused`: the same, paused or not, but not while
    -- dimmed (and whatever `no_sprite_update` says).
    function update_sprite_while_paused(self): ()
    -- The sprite's attach point n, in pixels facing the object's way.
    function attach_point(self, n: number): (number, number)

    function set_coordinates_from_panel(self): ()
    function set_panel_from_coordinates(self): ()
    function update_collision_panels(self): ()
    -- `sub_80169BE`: shown unless the battle is dimmed; hidden if it is the
    -- remote side's and the local player is blind.
    function update_visibility(self): ()
    -- Onto the destination panel of a move (panel, reservation,
    -- coordinates, collision).
    function snap_to_future_panel(self): ()
    -- `object_setCollisionPanelsToCurrent`: the collision registration's
    -- panel becomes the object's (its move direction kept).
    function set_collision_panel(self): ()
    -- `object_highlightCurrentCollisionPanels`: highlight the panels the
    -- registration's region covers.
    function highlight_collision_panels(self): ()
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
    -- damage word): what it is and what it hits, as collision types
    -- (define.collision). Deprecated: collision types by index.
    function setup_collision(self, self_type: CollisionType | number, target_type: CollisionType | number, hit_mod: number): ()
    function reset_collision_types(self, self_type: CollisionType | number, target_type: CollisionType | number, hit_mod: number): ()
    -- The damage word a hit carries: the damage (with its flag bits) in
    -- the low half, the counter byte above (the object's damage and
    -- stamina).
    function set_damage_word(self, word: number): ()
    -- `object_spawnCollisionEffect`: the hit spark after a hit (one RNG
    -- draw when it shows).
    function hit_spark(self): ()
    -- `sub_801156A`: an object with HP takes this tick's damage (the sum
    -- of its hits by element): a guard spark if it blocked, the HP loss
    -- (not in mode 1), a white flash while hit, with a sound in modes 0
    -- and 2. -1 when its HP ran out, 1 when hit in another mode, else 0.
    function take_damage(self, mode: number): number

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
    -- `sub_80105F2`: the stance's own trap (the AntiDmg program's) caught a
    -- hit: its counter (action 0x47, or 0x48 for a sword hit) starts at the
    -- next tick with this attack's damage word, lockout and variant.
    function start_stance_counter(self): ()
    -- `sub_8011450`: restart the form overlay with the navi after an
    -- animation change.
    function refresh_form_overlay(self): ()
    -- `object_exitAttackState`: back to idle with animation 0.
    function exit_attack(self): ()
    -- `sub_801171C`: back to idle, the animation as it is.
    function end_attack(self): ()
    -- `object_setAttack0..5`: start an action (a definition); kind 1
    -- buster, 2 chip or charged shot, 3 special, 4 move. Deprecated: an
    -- action by number.
    function set_attack(self, action: Action | number, kind: number): ()
    -- What the navi runs: an action definition, one of the ruleset's own
    -- ("entry", "take_control", "deletion", "flinch", "paralysis", "drag",
    -- "freeze", "bubble", "idle", "move", "dimming_chip", "navi_chip",
    -- "instant_chip", "form_change", "cross_special"), or a number the
    -- ruleset doesn't name (a link navi's own action).
    function navi_action(self): Action | string | number
    function reset_attack_links(self): ()
    -- `sub_800FA54`: the held direction (1 up, 2 down, 3 back, 4 forward;
    -- 0 none).
    function held_direction(self): number
    -- `sub_800F964`: where a step toward dir would go, if it can.
    function step_target(self, dir: number): (number?, number?)
    -- `sub_80116AE`: start a step toward dir.
    function start_move(self, dir: number): ()
    -- `ho_8026554`: the panel the navi would attack (x, y) from in Beast
    -- Out lock-on mode `mode` (its own for mode 0 or a target off the
    -- field; (0, 0x7F) when no panel fits).
    function lockon_panel(self, x: number, y: number, mode: number): (number, number)
    -- `object_canMove`: not immobilized, sliding or moving.
    function can_move(self): boolean
    -- `sub_80DBB64`: put a junk look (`obstacle.junk_look`) on the sprite:
    -- the sprite, a shadow if the look has one, its animation and palette,
    -- flipped by the object's side unless the look keeps its own; false
    -- when the look is the table's none.
    function wear_junk_look(self, look: number): boolean
    -- `sub_800E2FC`: heal up to the maximum HP, with the recovery sparkle
    -- and sound; with `anti_recovery`, the opponent's armed AntiRecv turns
    -- it into damage instead (true when it did).
    function heal(self, amount: number, anti_recovery: boolean): boolean
    -- `sub_801265A`: the buster's damage.
    function buster_damage(self): number
    -- `sub_80127C0(0)`: fill the attack from the chip at the hand's cursor
    -- (its damage, bonuses and modifiers); returns the chip's action.
    function prepare_chip(self): number
    -- The attack state as content action `action`'s (zeroed unless that
    -- action last used it): how a weapon routine sets up the action it
    -- names before it starts. Cast it to the action's State type.
    function action_state(self, action: number): any
    -- The obstacles it absorbed, oldest first.
    function absorbed(self): { { kind: number, anim: number } }
    function push_absorbed(self, kind: number, anim: number): boolean
    function pop_absorbed(self): (number?, number?)

    -- Field objects (obstacles): whether another object asked this of it
    -- (the rest is the `obstacle` service).
    function obstacle_flag(self, flag: ObstacleFlag): boolean
end

-- A side's navi stats (read-only but the buster's levels, the custom
-- level, the hand-shrink bug's turn, the start weapons and the shoes).
declare class Navi
    -- Fighting in the sun.
    sun: boolean
    form: number
    navi: number
    navi_variant: number
    -- The base form's element.
    element: number
    -- The buster's attack level (writable: `SetBattleNaviStatsByte`).
    attack: number
    rapid: number
    charge: number
    mood: number
    beast_out_counter: number
    -- The navi's game: 0 Gregar, 1 Falzar.
    version: number
    max_base_hp: number
    chip_recovery: number
    -- NaviCust: the buster's spread shot, the charged shot's kind, and the
    -- buster's blank and charged slots (of 16).
    buster_shot: number
    charge_shot_kind: number
    buster_blanks: number
    buster_charged: number
    -- NaviCust bugs: the HP drain in the fight and while the custom
    -- screen is open (levels), and what a step leaves behind.
    hp_drain: number
    custom_drain: number
    panel_trail: number
    -- The form is a Beast form, Beast Over.
    beast: boolean
    beast_over: boolean
    custom_level: number
    -- NaviCust bug: the hand shrinks from this turn on (0 none).
    hand_shrink_turn: number
    -- The weapon routines rounds start with (0xFF none).
    charge_shot_routine: number
    back_special_routine: number
    float_shoes: boolean
    air_shoes: boolean
    undershirt: boolean
    -- `sub_800FE52`: how many kinds of NaviCust bug the navi has.
    bug_kinds: number
end

-- A one-tick hit region (`battle.hitbox`).
export type HitboxSpec = {
    panel_x: number,
    panel_y: number,
    element: number,
    z: number?,
    -- Region shape (a definition; deprecated: a number).
    region: Region | number,
    hit_effect: Spark | number,
    -- Collision types (definitions; deprecated: indices).
    target: CollisionType | number,
    self_type: CollisionType | number,
    -- The damage word: damage (with flag bits) and the counter byte.
    damage: number,
    stamina: number?,
    hit_mod: number?,
    status: number?,
    bug: number?,
    bug_arg: number?,
}

-- An afterimage (`battle.afterimage`): `anim` of `sprite` (or, without
-- one, a copy of the owner's sprite wearing its form's overlay), flipped
-- by the game's flip value, for `lifetime` ticks, blinking unless
-- `steady`; its shadow at its height unless said otherwise. It ends early
-- with its `tether`: when its side leaves the Beast forms, or when its
-- owner's action drops below 0x10.
export type AfterimageSpec = {
    sprite: (SpriteAsset | SpriteId)?,
    anim: number?,
    flip: number?,
    lifetime: number,
    color_shader: number?,
    palette: number?,
    shadow: Shadow?,
    steady: boolean?,
    tether: ("beast_form" | "attack")?,
}

-- A side's defensive-chip record (the chip by number, 0 for none; a chip
-- content defines is 0x200 + its handle).
export type LinkedChip = {
    chip: number,
    bonus: number,
    -- The damage word.
    damage: number,
    owner: Object?,
    object: Object?,
}

export type Panel = { kind: PanelType, alliance: number, home: number }
-- A panel flags word has every `require` bit and no `forbid` bit.
export type PanelCondition = { require: number, forbid: number }

-- A side's damage-carry record: the side's largest damage this tick and
-- last tick, the object that set the record, and the target that takes
-- last tick's damage on top of its own.
export type DamageCarry = { this_tick: number, previous: number, source: Object?, target: Object? }
export type Column = { home: number, timer: number }
export type NaviRecord = { actor_type: ActorType, ai_index: number }

-- How a form overlay's sprite steps once the navis are in (its Param3):
-- `object_updateSprite` (not while dimmed; it stops running while paused),
-- `object_updateSpriteTimestop`, or `sub_801BCD0` (paused or not).
export type FormOverlayStepping = "normal" | "while_dimmed" | "always"

-- A form overlay (`battle.form_overlay`): its sprite (Param1, Param2),
-- stepping (Param3), the offset to the owner's animation it plays
-- (Param4); a pixel nearer than its owner when `nudged` (ExtraVars[0]);
-- wearing the owner's palette when `owner_palette` (ExtraVars[1] 0xFF).
export type FormOverlaySpec = {
    sprite: SpriteAsset | SpriteId,
    -- "normal" by default.
    stepping: FormOverlayStepping?,
    anim_offset: number?,
    nudged: boolean?,
    owner_palette: boolean?,
}

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

-- What an instant chip's effect runs with (`sub_80EC39C`'s registers).
export type InstantChipSpec = {
    -- The user's panel and Z (16.16).
    panel_x: number,
    panel_y: number,
    z: number,
    element: number,
    params: { number },
    -- The damage word (damage | hit parameter << 16) plus the bonus's low
    -- byte.
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
    -- Custom screens opened so far.
    turn: () -> number,
    -- Battle flag 0x40: each player has a custom gauge (not in netbattles).
    per_player_gauges: () -> boolean,
    -- Battle flag 1: the fight is on (collision is live).
    fighting: () -> boolean,
    -- Sound for both players, or for one side's player (output only): a
    -- sound asset (asset.sound). Deprecated: a sound number.
    play_sound: (sound: Sound | number) -> (),
    play_sound_for: (side: number, sound: Sound | number) -> (),
    -- `camera_initShakeEffect_80302a8`: the camera shakes for `ticks` ticks
    -- at `magnitude` (0-3). Drawn only, but each shaking tick draws from
    -- the consoles' own RNGs, which ChpShufl's re-deal reads.
    shake_camera: (magnitude: number, ticks: number) -> (),
    navi: (side: number) -> Navi,
    emotion: (side: number) -> Emotion,
    -- Unless the navi's emotion is held.
    set_mood: (side: number, mood: number) -> (),
    -- `sub_802E4B8`: the side's SELECT or Cross special in progress.
    side_special: (side: number) -> SideSpecial,
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
    -- `sub_801DF8C`: the custom gauge fills `rate` a tick (full at 0x4000).
    set_gauge_rate: (rate: number) -> (),
    -- A side's slow and fast gauge timers (`sub_802E070`+0x3C, +0x3A).
    set_gauge_speed_ticks: (side: number, slow: number, fast: number) -> (),
    -- `sub_802E032`: add to a side's own custom gauge, up to full.
    add_side_gauge: (side: number, n: number) -> (),
    -- `sub_8010488`'s special-source branch: add to a bonus a side stores
    -- for its special chip (`index` 0: the Atk+ bonus a damaging chip
    -- spends; 1: the Navi+ bonus a navi chip spends).
    add_special_bonus: (side: number, index: number, n: number) -> (),
    bump_side_stat: (side: number, index: number, n: number) -> (),
    -- `sub_800AB3A`: a side's statistics counter.
    side_stat: (side: number, index: number) -> number,
    navi_record: (name_id: number) -> NaviRecord?,
    -- The damage-carry record of the side that takes the damage
    -- (`dword_203CFB0`; CopyDmg's).
    damage_carry: (side: number) -> DamageCarry,
    set_damage_carry: (side: number, record: DamageCarry) -> (),
    -- The r3 the object update loop (`object_800372A`) leaves for the
    -- object updating now: 4 × how many objects of the previous object's
    -- pool it passed before that one this tick. Routines that never set r3
    -- spawn with it as a position (register garbage that is knowable).
    loop_register: () -> number,
    -- Spawn an object of a kind into its pool's lowest free slot, its
    -- state zeroed; it runs later this tick, right after the spawner. nil
    -- if the pool is full. Deprecated: by pool and index.
    -- (Typed loosely past the kind: the deprecated form is `(pool, index,
    -- pos?, params?)`.)
    spawn: (kind: Kind | Pool, ...any) -> Object?,
    -- `sub_80033E4`: the same at the head of the update list (it first
    -- runs next tick, before everything else).
    spawn_first: (kind: Kind, pos: Vec3?) -> Object?,
    -- `sub_8003374` / `sub_800333C`: the same, at the end of the update
    -- list (it runs after everything else this tick).
    spawn_at_end: (kind: Kind, pos: Vec3?) -> Object?,
    -- Deprecated: a content kind by name (its folder under objects/); use
    -- `spawn(kind, pos)`.
    spawn_kind: (name: string, pos: Vec3?, params: { number }?) -> Object?,
    -- Deprecated: use `spawn_first(kind, pos)`, `spawn_at_end(kind, pos)`.
    spawn_kind_first: (name: string, pos: Vec3?, params: { number }?) -> Object?,
    spawn_kind_at_end: (name: string, pos: Vec3?, params: { number }?) -> Object?,
    -- The generic one-shot effect (effect object #0) an effect definition
    -- describes. Deprecated: an effect by number.
    effect: (pos: Vec3, effect: Effect | number, flip: number?, palette_add: number?, priority: number?) -> Object?,
    -- `sub_801BD3C`: the one-shot effect `id` on each field panel of hit
    -- region `region` around (x, y), turned the way side `side` faces, at
    -- height z (default 0); a whole-field region's (0x80 and up) from the
    -- bottom right, on the ground.
    region_effects: (x: number, y: number, region: Region | number, side: number, effect: Effect | number, z: number?) -> (),
    -- The one-tick hit region (attack object #3).
    hitbox: (owner: Object, spec: HitboxSpec) -> Object?,
    -- `sub_80E11E0`: a screen palette flash (effect object #0x0A) of a
    -- variant (0 white or red, 1 white over two layers) for `ticks`,
    -- optionally going on while dimmed or paused.
    palette_flash: (variant: number, ticks: number, while_dimmed: boolean?, while_paused: boolean?) -> Object?,
    -- A hit spark (effect object #4) a spark definition describes.
    -- Deprecated: a spark by number.
    spark: (owner: Object, pos: Vec3, spark: Spark | number) -> Object?,
    -- `sub_80C468C`: a form overlay (actor object #0x57) on `owner`,
    -- following it every tick.
    form_overlay: (owner: Object, spec: FormOverlaySpec) -> Object?,
    -- Subtype 18 (Otenko): the turn (from 0) a hand's chip at i was picked
    -- in, and adding to its Atk+ bonus.
    hand_turn: (side: number, i: number) -> number,
    add_hand_attack_bonus: (side: number, i: number, n: number) -> (),
    -- Subtype 8 (Wind and Fan): a side's wind (`sub_80E543C`) and who
    -- placed it; `sub_80E541A` makes `me` a side's wind (the one there
    -- before is destroyed at once); `sub_80E544C` makes it no side's.
    wind: (side: number) -> (Object?, WindSource),
    set_wind: (me: Object, side: number, source: WindSource) -> (),
    clear_wind: (me: Object) -> (),
    -- `sub_80E33FA`: a plain afterimage (effect object #0x28) of `owner`'s
    -- side at `pos`: a copy of a sprite that stays behind.
    afterimage: (owner: Object, pos: Vec3, spec: AfterimageSpec) -> Object?,
    -- `sub_8018810`: NameID `name_id`'s sprite attach point `point`, in
    -- pixels, facing the way `alliance` and `flip` say.
    attach_point: (name_id: number, point: number, alliance: number, flip: number) -> (number, number),
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
    -- A side's registered field objects (the obstacles it owns), in the
    -- registry's slot order.
    objects: (side: number) -> { Object },
    -- Every registered field object (the registry's eight slots: each
    -- side's three, then the stage's two), in slot order.
    all_objects: () -> { Object },
    column: (x: number) -> Column,
    set_column_timer: (x: number, ticks: number) -> (),
    set_alliance: (x: number, y: number, side: number) -> (),
    set_type: (x: number, y: number, kind: PanelType) -> (),
    -- `object_crackPanel`: crack a solid panel, or break a cracked,
    -- unoccupied one.
    crack: (x: number, y: number) -> boolean,
    -- `object_breakPanel_dup2`: break a solid panel, or crack it while
    -- something stands on it.
    break_panel: (x: number, y: number) -> boolean,
    -- `object_breakPanel_dup1`: break a solid panel, or crack it when
    -- something stands on it (not a neutral object); true only when it
    -- broke.
    shatter: (x: number, y: number) -> boolean,
    -- `object_breakPanel`: break a solid panel nothing stands on (true);
    -- leave any other alone.
    break_empty: (x: number, y: number) -> boolean,
    -- `object_isPanelSolid`: something can stand on it (not missing or
    -- broken).
    solid: (x: number, y: number) -> boolean,
    -- Drawn only.
    highlight: (x: number, y: number) -> (),
    -- Panel changes (dimming chip subtypes 2, 3, 5, 15 and 27).
    -- `object_panel_setPoison`: a solid panel turns to poison; whether it
    -- was solid.
    poison: (x: number, y: number) -> boolean,
    -- `object_setPanelTypeBlink`: this frame the panel is drawn as `kind`
    -- of `side` (drawn only).
    blink: (x: number, y: number, kind: PanelType, side: number) -> (),
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
    -- `sub_800BF16`: `side` starts a dimming with `controller` (nil if its
    -- spawn failed), used by `user`; `no_cut_in`: the other side can't cut
    -- in. For controllers no chip action registers (a trap springing).
    start: (side: number, no_cut_in: boolean, controller: Object?, user: Object) -> (),
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
    -- `sub_80E1332`: the user warps out (`out`) or back in.
    warp: (user: Object, out: boolean) -> (),
}

-- What an actor list's entry is placed with (`off_80073A0[type]`): an
-- object kind's `actor_list_entry(spec)`.
export type ActorListEntrySpec = {
    panel_x: number,
    panel_y: number,
    -- The entry's side (field objects take their panel's instead).
    side: number,
    -- The entry's argument (a rock's variant).
    variant: number,
}

export type ObstacleCrush = "breaks" | "destroys" | "spares_bodies"
-- Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework:
export type ObstacleHold = "after_appearing" | "always"
export type ObstaclePush = "forgets_damage" | "keeps_damage" | "any_hit" | "ignored"
export type WindSource = "obstacle" | "navi"
export type ObstacleRemoval = "broken" | "removed" | "vanished" | "absorbed"
-- How a removed field object's blink-out goes.
export type BlinkOut = "no" | "blinking" | "done"

-- The obstacle framework: the steps of a field obstacle's update (rocks,
-- cubes and the like, without actor data), which its update calls in its
-- routine's order: `take_hits`, the hit spark, `tick_lifetime`, `react`
-- (then the kind's own action table: its entries for actions 0 appear, 2
-- destroyed and 8 idle, and the shared ones below for 1, 3 to 7), the
-- sprite, and presenting its collision.
declare obstacle: {
    -- `setFieldBattleObject_800F614`: one of `side`'s field objects of
    -- `class` (0: two a side, 1: one); a third evicts the oldest (its HP
    -- drops to 0).
    register: (me: Object, side: number, class: number) -> (),
    -- `sub_800F656`: forget it in the field-object registry.
    unregister: (me: Object) -> (),
    -- `sub_801AD9E` and its variants: resolve this tick's hits. What a
    -- pushing hit (hit modifier 0x40) does: "forgets_damage" (the default,
    -- `sub_801AD9E`: pushed, its damage forgotten), "keeps_damage"
    -- (`sub_801AD12`: pushed, the damage lands), "any_hit" (`sub_801ADFA`:
    -- that, and any one side's hit but type 0x1000 pushes it a panel away),
    -- "ignored" (`sub_801AD6A`: never pushed).
    take_hits: (me: Object, push: ObstaclePush?) -> (),
    -- `sub_800F672`: the lifetime (`timer`): broken when it runs out or
    -- the battle is over, blinking for its last three seconds.
    tick_lifetime: (me: Object) -> (),
    -- `sub_801B394` ("breaks", the default: a body's, another obstacle's
    -- or a breaking hit's touch drops the HP to 0), `sub_801B4D4`
    -- ("destroys": destroyed, the HP as it is) or `sub_801B610`
    -- ("spares_bodies": bodies don't break it): damage, removal and status;
    -- the action the kind's table runs now, nil when a status routine ran.
    -- While dimmed it holds the obstacle still once it has appeared
    -- ("after_appearing", the default) or always ("always", `sub_801B750`).
    react: (me: Object, crush: ObstacleCrush?, hold: ObstacleHold?) -> number?,
    -- The shared table entries: `sub_80165B8` (1), the pushes (5:
    -- `sub_8017E26` slides up to six panels, not into bodies or obstacles
    -- nor, pulled back, into the puller's area; `sub_8017CC0` is knocked
    -- back as far as the hit says onto free panels it reserves), and the
    -- actor reactions nothing starts on an obstacle (3, 4, 6, 7: an
    -- error).
    return_to_idle: (me: Object) -> (),
    slide: (me: Object) -> (),
    knocked_back: (me: Object) -> (),
    flinch: (me: Object) -> (),
    paralyzed: (me: Object) -> (),
    frozen: (me: Object) -> (),
    bubbled: (me: Object) -> (),
    -- How it is leaving (for its destroyed action).
    removal: (me: Object) -> ObstacleRemoval,
    -- `sub_800F8CE`: blink out for 20 ticks when it vanishes.
    blink_out: (me: Object) -> BlinkOut,
    -- `sub_800F90E`: absorbed, it flies to the absorbing navi as obstacle
    -- kind `kind` (`data.objects.absorbed_sprites`).
    fly_to_absorber: (me: Object, kind: number) -> (),
    -- `sub_802EF5C`: the per-side target tracking some chips keep.
    release_tracking: (me: Object) -> (),
    -- A chip's requests: `sub_800F884` removes it, `sub_800F898` makes it
    -- blink out, `sub_800F8B0` has the absorber's side's navi absorb it.
    remove: (obj: Object) -> (),
    vanish: (obj: Object) -> (),
    absorb: (obj: Object, absorber: Object) -> (),
    -- `sub_80EFD74`: `absorber` pulls in every registered field object
    -- (but NameID 0xDA and those already leaving); each flies to its
    -- side's navi as an absorbed obstacle.
    absorb_all: (absorber: Object) -> (),
    -- It has a collision registration and isn't already leaving the field
    -- (removed, blinking out or absorbed): the test `sub_80C9EE6` and
    -- `sub_80EFD8C` make before taking an obstacle.
    present: (o: Object) -> boolean,
    -- DustMan's take (`sub_80BC100`): what field object `o` would look like
    -- thrown back as junk, an opaque look its taker keeps in a "u16" state
    -- field (`me:wear_junk_look`); nil for the objects DustMan leaves
    -- (`sub_800F486`).
    junk_look: (o: Object) -> number?,
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

-- Action 0x13's slash: its hit region and the effect that draws it.
export type SwordSlash = {
    region: number,
    hit_effect: number,
    target: number,
    self_type: number,
    hit_mod: number,
    status: number,
    bug: number,
    bug_arg: number,
    effect: number,
}

-- A sword's data (actions 0x13 and 0x49), in its chip's folder.
export type Sword = {
    -- The blade the user holds: an attachment kind.
    blade: number,
    -- Action 0x13's slash.
    slash: SwordSlash?,
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
    -- The HP a recovery chip restores (action 0x20).
    recovery: number?,
    sword: Sword?,
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

-- A kind of sword wave (attack object #0x96), by its first parameter.
export type SwordWave = {
    id: number,
    -- Collision types (what it is, what it hits) and hit modifier.
    self_type: number,
    target_type: number,
    hit_mod: number,
    region: number,
    sprite: SpriteId,
    anim: number,
    -- It animates, restarting its animation after the last frame.
    animates: boolean,
    -- It highlights the panels its region covers.
    highlight: boolean,
    -- Panel centers it passes before it ends.
    reach: number,
    ground_shadow: boolean,
    palette: number,
    status: number,
    -- 16.16 per tick, forward.
    speed: number,
}

-- A kind of boomerang (objects/boomerang): 16.16 pixels a tick along a row
-- and along the column, and whether it turns panels it crosses to grass.
export type BoomerangKind = { id: number, speed: number, turn_speed: number, grass: boolean }
-- What a projectile's hit does to the panel it hits: crack it
-- (`object_crackPanel`), break it (`object_breakPanel_dup2`), or make a
-- solid one a panel of a type (by the shot's side).
export type PanelHit = {
    effect: "crack" | "break" | "set_type",
    left_side: PanelType?,
    right_side: PanelType?,
}

-- A kind of projectile (attack object #0, objects/projectile), by its
-- first parameter: fire one with lib/projectile.luau.
export type ProjectileKind = {
    id: number,
    -- Collision types (what it is, what it hits) and hit modifier.
    self_type: number,
    target_type: number,
    hit_mod: number,
    -- Its element and secondary elements (lib/element.luau).
    element: string,
    secondary: { string }?,
    -- The hit spark (0xFF: none).
    hit_effect: number,
    -- Drawn with this sprite and animation (0 if nil), if any.
    sprite: SpriteId?,
    anim: number?,
    -- The status byte and bug its hits carry (nil or 0: none).
    status: number?,
    bug: number?,
    bug_arg: number?,
    hit_panel: PanelHit?,
    -- It bursts around what it hits, and over the last two columns when
    -- it leaves the field.
    bursts: boolean?,
    -- It sits a pixel lower down the field and higher on each panel.
    climbs: boolean?,
}

-- A kind of flying shot (attack object #0xB, objects/flying-shot), by its
-- first parameter.
export type FlyingShotKind = {
    id: number,
    self_type: number,
    target_type: number,
    hit_mod: number,
    element: string,
    secondary: { string }?,
    hit_effect: number,
    -- Its sprite and animation (0 if nil).
    sprite: SpriteId,
    anim: number?,
    shadow: boolean?,
    -- Pixels a tick forward, 16.16.
    speed: number,
    -- Panel centers it passes before it ends.
    range: number,
    -- The status byte its hits carry (nil or 0: none).
    status: number?,
    highlight: boolean?,
    -- A thrown obstacle: its look is its spawner's.
    obstacle: boolean?,
    -- Its spark shows over its panel's center.
    panel_spark: boolean?,
    -- The sound when it sets off after its wait.
    launch_sound: number?,
    -- The effect it leaves when its range runs out.
    end_effect: number?,
}
-- How a field object looks by its NameID (`byte_8021220`, NameIDs
-- 0xCD..=0xFF, as `sub_800F26C` gives it); no sprite is the table's none.
export type NameLook = { name_id: number, sprite: SpriteId?, anim: number, palette: number, shadow: boolean }

export type WeaponData = { id: number, name: string, action: number?, instant_chip: number?, script: string }

-- A shock wave's look and timing (attack object #0x16, by its first
-- parameter): ticks on a panel before the next rolls on, and the panel
-- type it leaves ("cracked" cracks the panel, "broken" breaks it).
export type ShockWave = { id: number, sprite: SpriteId, anim: number, ticks: number, panel: PanelType? }
-- An object kind a script implements.
export type ObjectKindData = { pool: Pool, index: number, script: string }

export type PackData = {
    chips: { [number]: ChipData },
    -- Hit-region shapes by region number: the panels' offsets `{ dx, dy }`
    -- from the anchor (dx facing right).
    regions: { [number]: { { number } } },
    navis: { [number]: NaviData },
    forms: { [number]: FormData },
    weapons: { [number]: WeaponData },
    -- Region shapes by region number (`PanelOffsetListsPointerTable`): the
    -- panels as {dx, dy}, dx toward the facing side, in the order hits and
    -- panel searches (`object_getPanelRegion`) take them.
    regions: { [number]: { { number } } },
    objects: {
        attachments: { [number]: AttachmentKind },
        rocks: { [number]: RockKind },
        absorbed_sprites: { [number]: SpriteId },
        body_overlays: { [number]: BodyOverlay },
        sun_beam_looks: { [number]: SpriteId },
        boomerangs: { [number]: BoomerangKind },
        projectiles: { [number]: ProjectileKind },
        flying_shots: { [number]: FlyingShotKind },
        sword_waves: { [number]: SwordWave },
        name_looks: { [number]: NameLook },
        kinds: { [string]: ObjectKindData },
        shock_waves: { [number]: ShockWave },
    },
    rules: {
        -- The buster's recovery in ticks (`byte_80209CC`), at
        -- rapid * 6 + min(open panels ahead, 5).
        buster_recovery: { [number]: number },
        -- The sine table (`math_sinTable`, which `math_cosTable` continues):
        -- 256 steps a turn, 1.0 = 0x100, over a turn and a half, so the
        -- cosine of step a is entry a + 64.
        sine: { [number]: number },
        -- Whole-field hit regions (`byte_8019C34`): region 0x80 + i covers
        -- every panel whose flags word has all of `require` and none of
        -- `forbid`.
        field_regions: { [number]: PanelCondition },
        -- MegaMan's palette in each Cross, by form (`byte_80203EA`).
        cross_palettes: { [number]: number },
    },
    -- Hit regions (`PanelOffsetListsPointerTable`) by region number: the
    -- panels a region covers, { dx, dy } from its panel with dx toward the
    -- facing side, in the game's order.
    regions: { [number]: { { number } } },
}

declare data: PackData

-- ---- Definitions and assets (docs/design/content-model-v2.md §3) -----------------------------

-- Assets, named while content loads (`asset.sprite("bomb")`; names are
-- compat/assets.toml's): one value per asset. (`SpriteAsset`, since
-- `Sprite` is an object's sprite.)
declare class SpriteAsset
    name: string
end
declare class Sound
    name: string
end
declare class Banner
    name: string
end
declare class Background
    name: string
end
declare class Mugshot
    name: string
end

declare asset: {
    sprite: (name: string) -> SpriteAsset,
    sound: (name: string) -> Sound,
    banner: (name: string) -> Banner,
    background: (name: string) -> Background,
    mugshot: (name: string) -> Mugshot,
}

-- A state layout: each field's type, "bool", "u8", "u16", "u32", "i8",
-- "i16", "i32", "object", "vec3", an array ("u8[18]"), a list of variant
-- names, or a reference: "kind", "action", "effect", "spark", "region",
-- "collision", "chip", "weapon", "record", "record:<type>" (records of one
-- type), "sprite", "sound", "banner", "background", "mugshot". A reference
-- field holds a definition or an asset, or nil.
export type StateSchema = { [string]: string | { string } }

-- An object kind: one behaviour; what differs between uses goes in its
-- state when spawned. `id` is qualified by its owner's folder when it lives
-- in one ("minibomb/held").
export type KindSpec = {
    id: string,
    pool: Pool,
    state: StateSchema?,
    update: (me: Object) -> (),
}
export type Kind = KindSpec

-- A navi action. Actions sharing a state table continue each other's
-- attack state (the original's actions of one number do). An action a
-- role or a weapon names needs an `id`; a chip's is derived.
export type ActionSpec = {
    id: string?,
    state: StateSchema?,
    args: any?,
    update: (me: Object, s: any) -> (),
    traits: { string }?,
}
export type Action = ActionSpec

export type WeaponSpec = {
    id: string,
    name: string,
    charge_ticks: { number },
    -- `off_80117D4[n]`: prepare the attack, name its action.
    setup: (navi: Object) -> Action | number,
    sticky: boolean?,
    traits: { string }?,
}
export type Weapon = WeaponSpec

-- A chip: its record, and exactly one use.
export type ChipSpec = {
    id: string,
    name: string,
    description: string?,
    codes: { string }?,
    element: string?,
    family: string?,
    class: string?,
    rarity: number?,
    mb: number?,
    damage: any?,
    hit_param: number?,
    lockout: number?,
    flags: { string }?,
    extra_flags: { string }?,
    beast: { lockon: any }?,
    library: { number: number, index: number, sort: number }?,
    slot_in_limit: number?,
    traits: { string }?,
    dark_substitute: any?,
    modifier: any?,
    program_advances: { any }?,
    -- Transitional (counted by the ratchet): the original's subtype and
    -- parameter bytes, for what reads them of the chip besides its own use
    -- (SlashCross's charged slash reads a sword's). `action` and `script`
    -- (a behaviour still a v1 module) come with step 5.
    legacy: { subtype: number?, params: { number }? }?,
    action: Action?,
    dimming: ((user: Object, spec: DimmingChipSpec) -> Object?)?,
    navi: ((user: Object, controller: Object, spec: NaviChipSpec) -> Object?)?,
    instant: ((user: Object, spec: InstantChipSpec) -> ())?,
}
export type Chip = ChipSpec

-- Navis, forms, stages, statuses and lock-on modes: their specs are the
-- generator's (§3.2, §3.7), typed field by field when the engine reads
-- them (steps 5 and 10 to 12). (`NaviDef`, since `Navi` is a side's stats.)
export type NaviDef = { id: string, [string]: any }
export type FormDef = { id: string, [string]: any }
export type StageDef = { id: string, [string]: any }
export type StatusDef = { id: string, [string]: any }
export type LockonDef = { id: string, [string]: any }

-- A one-shot effect's or hit spark's look.
export type EffectSpec = { sprite: SpriteAsset, anim: number?, palette: number? }
export type Effect = EffectSpec
export type Spark = EffectSpec

-- A hit region: panels around the anchor (`{ dx, dy }`, dx toward the
-- side's front), or the whole field's panels that meet a condition.
export type RegionSpec = { panels: { { number } }?, field: PanelCondition? }
export type Region = RegionSpec

-- A collision type: what an object is or what it hits, as a flag word per
-- side. `row_offset`: the offset the original's lookup of the type's row
-- leaves in a register (the row's index times 8), which a damage word's
-- bug code takes as its high byte. (`CollisionType`, since `Collision` is
-- an object's registration.)
export type CollisionTypeSpec = { id: string, side0: number, side1: number, row_offset: number? }
export type CollisionType = CollisionTypeSpec

-- What the ruleset needs from content by role (rules/roles.luau, once).
export type RolesSpec = {
    actions: {
        anti_damage_counter: Action?,
        anti_sword_counter: Action?,
        body_guard_counter: Action?,
    }?,
}

-- The definers: each records its spec (frozen) and returns it. They work
-- while content loads (a module's top level and what it calls).
declare define: {
    chip: (spec: ChipSpec) -> Chip,
    navi: (spec: NaviDef) -> NaviDef,
    form: (spec: FormDef) -> FormDef,
    weapon: (spec: WeaponSpec) -> Weapon,
    kind: (spec: KindSpec) -> Kind,
    action: (spec: ActionSpec) -> Action,
    stage: (spec: StageDef) -> StageDef,
    effect: (spec: EffectSpec) -> Effect,
    spark: (spec: EffectSpec) -> Spark,
    region: (spec: RegionSpec) -> Region,
    collision: (spec: CollisionTypeSpec) -> CollisionType,
    status: (spec: StatusDef) -> StatusDef,
    lockon: (spec: LockonDef) -> LockonDef,
    roles: (spec: RolesSpec) -> RolesSpec,
    -- Luau-only data with a handle and a type name, which a reference field
    -- ("record:<type>") can hold.
    record: <T>(record_type: string, spec: T) -> T,
}
