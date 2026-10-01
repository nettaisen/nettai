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
-- What is left of the numeric API is the `legacy` markers of the navis,
-- the forms and the body overlays (marked "Deprecated" below), which
-- bn6-content-check's ratchet counts per module (tests/deprecated.txt) and
-- only lets shrink.
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
    -- A field object being carried to be thrown.
    | "carried"
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
    function load(self, sprite: SpriteAsset): ()
    -- `sub_800F29C` with `sub_800FC9E` or `sub_800F26C`, then `load`: the
    -- sprite `like` is drawn with (a player navi's by its side's navi and
    -- form; a field object's by its look): a stand-in for a chip's user.
    function load_like(self, like: Object): ()
    -- The sprite `owner`'s NameID record gives it: a player's own by its
    -- navi stats (`sub_800FC9E`), another object's its NameID look
    -- (`sub_800F26C`); loaded as `load` does.
    function load_look_of(self, owner: Object): ()
    -- Start an animation from its first frame.
    function set_animation(self, anim: number): ()
    -- Advance the animation one tick (no gating).
    function step(self): ()
    -- `sub_80030BA`: where part n (from 0) of the current frame sits, in
    -- pixels from the object, unflipped; (0, 0) when the frame has fewer
    -- parts. Objects attach to them.
    function part_offset(self, n: number): (number, number)
    -- The game's flip value: bit 0 horizontal, bit 1 vertical.
    function set_flip(self, flip: number): ()
end

-- An object's collision registration.
declare class Collision
    -- The region its hits cover (a region definition; nil, none) and the
    -- spark they show (a spark definition; nil, none): the fields, and
    -- the same as methods.
    function set_region(self, region: Region?): ()
    function set_hit_effect(self, spark: Spark?): ()
    region: Region?
    -- The panel the region is anchored on (`update_collision_panels`
    -- copies the object's).
    panel_x: number
    panel_y: number
    hit_effect: Spark?
    -- The primary element its hits carry and the secondary elements (sword
    -- 0x80, cursor 0x40, wind 0x20, break 0x10): setup takes them from the
    -- object's element byte; `sub_8019F8C` sets them again.
    element: number
    secondary_element: number
    -- The status its hits carry (rules/status), if any.
    status_base: Status?
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
    -- The hit modifiers of what hit it this window (HitModifierFinal; 0x3C
    -- the pushes, 0x40 a pushing hit; read-only).
    hit_mod_final: number
    -- Its barrier (read-only): none, up, or popped (blown away by wind);
    -- the barrier's HP byte; the hit modifier that popped it.
    barrier: BarrierState
    barrier_hp: number
    barrier_pop_hit_mod: number
    -- Register on the region's panels, clearing the last results.
    function present(self): ()
    -- Unregister, resolving hits against what is registered there.
    function remove(self): ()
    function free(self): ()
    -- The damage taken this window in an element (0 null, 1 fire, 2 aqua,
    -- 3 elec, 4 wood; 5 the sixth slot).
    function element_damage(self, element: number): number
    -- `sub_801A4DC`: the objects whose collisions hit it this window, in
    -- slot order.
    function hit_by(self): { Object }
end

-- A handle to an object slot. Like the game's pointers, it can outlive the
-- object (and then names whatever takes the slot next).
declare class Object
    pool: Pool
    -- Its kind (read-only): compare with a kind definition
    -- (`me.kind == bomb.kind`).
    kind: Kind
    -- Setting it changes only the lifecycle state (the game's byte store);
    -- `set_lifecycle` also starts from the first action and phase.
    lifecycle: Lifecycle
    -- The object's action and phase; their meaning is the kind's. (A navi
    -- has no action byte: `navi_action`, `set_attack`.)
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
    -- What it is taken for (nil: a virus, as the original's NameID 0): a
    -- navi's or a form's identity, a field object's.
    identity: Identity?
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
    -- the phase's entry ran.
    step: number
    step_init: number
    -- The attack's chip, its definition (nil when the attack has
    -- none): what a weapon that uses the chip asks for the chip's own
    -- parts (`navi.attack_chip.action`), and what an action that becomes
    -- another chip's sets.
    attack_chip: Chip?
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
    -- The lock-on mode the attack's own action asks the Beast Out rush for
    -- (the charged sword's, by its slash); nil: the chip's.
    rush_lockon: Lockon?
    -- A per-action word some actions keep.
    marker: number
    -- The absorbed obstacle a throw carries (the marker word, as
    -- DustCross's throws keep it): its look and animation.
    thrown_look: AbsorbedLook?
    thrown_anim: number
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
    -- The weapons the navi's buttons run (nil: none; the buster's and the
    -- charged shot's writable).
    buster_weapon: Weapon?
    charge_shot_weapon: Weapon?
    back_special_weapon: Weapon?
    -- Writable: the Beast Out counter is spent (AIData+0x32).
    beast_out_spent: boolean
    -- Writable: the barrier's visual (AIData+0x60), which hides and shows
    -- with the navi (its state's `shown`).
    barrier_visual: Object?

    -- `sub_800F26C`: the look its NameID gives is `sprite` (an object's
    -- NameID look; a navi's or form's NameID gives its sprite). An error
    -- for a NameID the content has no look for.
    function name_look_is(self, sprite: SpriteAsset): boolean

    -- Enter a lifecycle state from its first action and phase (the game's
    -- word store).
    function set_lifecycle(self, state: Lifecycle): ()
    -- Switch to an action from its first phase (not a navi's).
    function set_action(self, action: number): ()
    -- Free the slot now.
    function free(self): ()
    -- `object_genericDestroy`: release panel reservations and collision,
    -- then free.
    function destroy(self): ()
    -- `sub_8010DF6`: put on the overlays an actor record adds (a body
    -- overlay, SpoutMan's idle overlay, a beast head), kept in related2
    -- (CircusMan's second one kept by the engine); `arg` 1: their sprites
    -- step even while paused.
    function add_navi_parts(self, actor_type: ActorType, ai_index: number, arg: number): ()
    -- `sub_8011044`: take them off (at their next update).
    function remove_navi_parts(self, actor_type: ActorType, ai_index: number): ()
    -- `sub_8010DF6` with `owner`'s NameID record: put on the parts that
    -- record adds (a navi's body overlay), kept in related2; with
    -- `keep_stepping`, a part that came steps even while paused and dimmed.
    -- With `paused_stepping`, `sub_8010DF6`'s r2 is 1 rather than the
    -- record's first byte (the dimming chips' stand-ins): the parts step
    -- even while paused.
    function add_parts_of(self, owner: Object, keep_stepping: boolean?, paused_stepping: boolean?): ()
    -- `sub_8011044` with `owner`'s NameID record: take them off.
    function remove_parts_of(self, owner: Object): ()

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
    -- `sub_801BC24`: load a requested animation (without stepping it), else
    -- step the sprite; `update_sprite`'s gating but for holds.
    function load_or_step_sprite(self): ()
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
    -- Every status flag off (the collision's whole status word cleared).
    function clear_statuses(self): ()
    function status_timer(self, timer: StatusTimer): number
    function set_status_timer(self, timer: StatusTimer, ticks: number): ()

    function create_collision(self): boolean
    -- Set up the registration from the object (side, panel, element,
    -- damage word): what it is and what it hits, as collision types
    -- (define.collision).
    function setup_collision(self, self_type: CollisionType, target_type: CollisionType, hit_mod: number): ()
    function reset_collision_types(self, self_type: CollisionType, target_type: CollisionType, hit_mod: number): ()
    -- The damage word a hit carries: the damage (with its flag bits) in
    -- the low half, the counter byte above (the object's damage and
    -- stamina).
    function set_damage_word(self, word: number): ()
    -- Replace the whole header flag byte (the flags above are its bits: in
    -- use 0x01, `visible` 0x02, `run_while_paused` 0x04, `no_sprite_update`
    -- 0x08, `run_while_dimmed` 0x10, `holds_reservation` 0x20; 0x40 and
    -- 0x80 nothing reads): what a spawner's bug leaves (the boulder's reads
    -- the byte from the wrong address). Everything else sets flags by name.
    function set_header_flags(self, flags: number): ()
    -- `object_spawnCollisionEffect`: the hit spark after a hit (one RNG
    -- draw when it shows).
    function hit_spark(self): ()
    -- `sub_801156A`: an object with HP takes this tick's damage (the sum
    -- of its hits by element): a guard spark if it blocked, the HP loss
    -- (not in mode 1), a white flash while hit, with a sound in modes 0
    -- and 2. -1 when its HP ran out, 1 when hit in another mode, else 0.
    function take_damage(self, mode: number): number
    -- `sub_801A7CC`: raise a barrier (its collision data).
    function raise_barrier(self, spec: BarrierSpec): ()

    -- Navis only.
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
    -- hit: its counter (the roles' `anti_damage_counter`, or
    -- `anti_sword_counter` for a sword hit) starts at the next tick with
    -- this attack's damage word and lockout.
    function start_stance_counter(self): ()
    -- `sub_8011450`: restart the form overlay with the navi after an
    -- animation change.
    function refresh_form_overlay(self): ()
    -- `object_exitAttackState`: back to idle with animation 0.
    function exit_attack(self): ()
    -- `sub_801171C`: back to idle, the animation as it is.
    function end_attack(self): ()
    -- `object_setAttack0..5`: start an action (a definition); kind 1
    -- buster, 2 chip or charged shot, 3 special, 4 move.
    function set_attack(self, action: Action, kind: number): ()
    -- What the navi runs: an action definition, or one of the ruleset's own
    -- ("entry", "take_control", "deletion", "flinch", "paralysis", "drag",
    -- "freeze", "bubble", "idle", "move", "dimming_chip", "navi_chip",
    -- "instant_chip", "form_change", "cross_special").
    function navi_action(self): Action | string
    function reset_attack_links(self): ()
    -- `sub_800FA54`: the held direction (1 up, 2 down, 3 back, 4 forward;
    -- 0 none).
    function held_direction(self): number
    -- `sub_800F964`: where a step toward dir would go, if it can.
    function step_target(self, dir: number): (number?, number?)
    -- `sub_80116AE`: start a step toward dir.
    function start_move(self, dir: number): ()
    -- `ho_8026554`: the panel the navi would attack (x, y) from in Beast
    -- Out lock-on mode `mode` (rules/lockon; its own for no mode or a
    -- target off the field; (0, 0x7F) when no panel fits).
    function lockon_panel(self, x: number, y: number, mode: Lockon?): (number, number)
    -- `object_canMove`: not immobilized, sliding or moving.
    function can_move(self): boolean
    -- `sub_80DBB64`: put a junk look (`obstacle.junk_look`) on the sprite:
    -- the sprite, a shadow if the look has one, its animation and palette,
    -- flipped by the object's side unless the look keeps its own; false
    -- when the look is the table's none.
    function wear_junk_look(self, look: Identity): boolean
    -- CrosOver's MegaMan (`sub_80BDBC8`): take the user's identity when the
    -- user is MegaMan or one of his forms (true), else MegaMan's own: its
    -- sprite (a ground shadow, animation 0 loaded by the next sprite
    -- update) and the side's form's palette.
    function wear_navi_image(self, user: Object): boolean
    -- MstrCros's Crosses (`sub_80BE7BC`), Darkness's Dark MegaMan
    -- (`sub_80BF710`): take the identity of MegaMan in form `form` (by
    -- number, as the navi stats give it; 0 his base form): its sprite (a
    -- ground shadow, animation 0 loaded by the next sprite update) and
    -- palette.
    function wear_megaman_image(self, form: number): ()
    -- The parts of that identity (`sub_8010DF6` with r2 1, or
    -- `sub_8011044`).
    function navi_image_parts(self, on: boolean): ()
    -- `sub_800E2FC`: heal up to the maximum HP, with the recovery sparkle
    -- and sound; with `anti_recovery`, the opponent's armed AntiRecv turns
    -- it into damage instead (true when it did).
    function heal(self, amount: number, anti_recovery: boolean): boolean
    -- `sub_801265A`: the buster's damage.
    function buster_damage(self): number
    -- `sub_80127C0(0)`: fill the attack from the chip at the hand's cursor
    -- (its damage, bonuses and modifiers; `attack_chip` is then the chip).
    function prepare_chip(self): ()
    -- The attack state as content action `action`'s (zeroed unless that
    -- action last used it): how a weapon routine sets up the action it
    -- names before it starts. Cast it to the action's State type.
    function action_state(self, action: Action): any
    -- The obstacles it absorbed, oldest first: their looks and
    -- animations.
    function absorbed(self): { { look: AbsorbedLook, anim: number } }
    function push_absorbed(self, look: AbsorbedLook, anim: number): boolean
    function pop_absorbed(self): (AbsorbedLook?, number?)

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
    -- NaviCust: the buster shot's and the charged shot's programs (the
    -- projectile variant a shot is on a lucky draw; nil: none), and the
    -- buster's blank and charged slots (of 16).
    buster_shot: ProjectileVariant?
    charge_shot_kind: ProjectileVariant?
    -- The damage a B+Back special takes from the navi's stats (ProtoMan's
    -- reflecting guard).
    back_special_damage: number
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
    -- The weapons rounds start with (nil: none).
    charge_shot_weapon: Weapon?
    back_special_weapon: Weapon?
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
    -- The region it covers and the spark its hits show (definitions; nil,
    -- none).
    region: Region?,
    hit_effect: Spark?,
    -- Its collision types: what it reaches, what it is.
    target: CollisionType,
    self_type: CollisionType,
    -- The damage word: damage (with flag bits) and the counter byte.
    damage: number,
    stamina: number?,
    hit_mod: number?,
    status: Status?,
    bug: number?,
    bug_arg: number?,
}

-- An afterimage (`battle.afterimage`): `anim` of `sprite` (or, without
-- one, a copy of the owner's sprite wearing its form's overlay), flipped
-- by the game's flip value, for `lifetime` ticks, blinking unless
-- `steady`; its shadow at its height unless said otherwise. It ends early
-- with its `tether`: when its side leaves the Beast forms, or when its
-- owner stops attacking.
export type AfterimageSpec = {
    sprite: SpriteAsset?,
    anim: number?,
    flip: number?,
    lifetime: number,
    color_shader: number?,
    palette: number?,
    shadow: Shadow?,
    steady: boolean?,
    tether: ("beast_form" | "attack")?,
}

-- A side's defensive-chip record (the chip; nil: no record).
export type LinkedChip = {
    chip: Chip?,
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

-- How a form overlay's sprite steps once the navis are in (its Param3):
-- `object_updateSprite` (not while dimmed; it stops running while paused),
-- `object_updateSpriteTimestop`, or `sub_801BCD0` (paused or not).
export type FormOverlayStepping = "normal" | "while_dimmed" | "always"

-- A form overlay (`battle.form_overlay`): its sprite (Param1, Param2),
-- stepping (Param3), the offset to the owner's animation it plays
-- (Param4); a pixel nearer than its owner when `nudged` (ExtraVars[0]);
-- wearing the owner's palette when `owner_palette` (ExtraVars[1] 0xFF).
export type FormOverlaySpec = {
    sprite: SpriteAsset,
    -- "normal" by default.
    stepping: FormOverlayStepping?,
    anim_offset: number?,
    nudged: boolean?,
    owner_palette: boolean?,
}

-- A navi's barrier (Collision.barrier).
export type BarrierState = "none" | "up" | "popped"
-- How a barrier behaves once raised (`sub_801A802`): worn down and timed
-- out; back with 1 HP 240 ticks after it is worn down, and broken by elec
-- (BblWrap's bubble); or regenerating up to 200 HP.
export type BarrierBehavior = "plain" | "bubble" | "regenerating"
-- `sub_801A7CC`'s row (`me:raise_barrier`): its HP, the damage a hit must
-- reach to wear it, its timer (0xFFFF: none) and the element that breaks
-- it (an element byte; 0: none).
export type BarrierSpec = {
    behavior: BarrierBehavior,
    hp: number,
    threshold: number,
    timer: number,
    weak_element: number,
}

-- What a dimming chip's controller is spawned with: the user's attack.
export type DimmingChipSpec = {
    element: number,
    -- The damage word (damage | hit parameter << 16).
    damage: number,
    -- The chip (nil: an attack without one), and the bonus the telop shows
    -- with it.
    chip: Chip?,
    bonus: number,
}

-- What a navi chip's navi is spawned with.
export type NaviChipSpec = {
    panel_x: number,
    panel_y: number,
    element: number,
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
    -- `sub_800EB6C`: the local player sees `side`'s objects (unless they
    -- are the other side's and the local navi is blind). Presentation, as
    -- the original keeps it in the objects' visibility.
    viewer_sees: (side: number) -> boolean,
    -- `sub_800ED90` and its record's flags: the chip `user` would use next
    -- deals damage (a player's at its hand's cursor; another object's own).
    next_chip_damages: (user: Object) -> boolean,
    link: () -> boolean,
    -- A battle against a ranked boss (battle effect 1).
    boss_rank: () -> boolean,
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
    -- sound asset (asset.sound).
    play_sound: (sound: Sound) -> (),
    play_sound_for: (side: number, sound: Sound) -> (),
    -- `sub_800AE90`: a warning marker on the HUD this tick (a blinking
    -- arrow, drawn only), over the custom gauge or over the place `at` on
    -- the field, with `sound` on every 16th frame of a console's own frame
    -- counter; on `side`'s console only, or on both. Call it every tick the
    -- warning shows.
    warn: (sound: Sound, at: Vec3?, side: number?) -> (),
    -- `sub_801DC7C`: every console's HUD numbers `object`'s HP under it
    -- (drawn only; one of four places, none when they are taken), rolling
    -- to it as it changes, until its HP is 0 or `hide_hp`.
    show_hp: (object: Object, spec: HpNumberSpec) -> (),
    -- `sub_801DD34`: the object's HP number goes.
    hide_hp: (object: Object) -> (),
    -- `camera_initShakeEffect_80302a8`: the camera shakes for `ticks` ticks
    -- at `magnitude` (0-3). Drawn only, but each shaking tick draws from
    -- the consoles' own RNGs, which ChpShufl's re-deal reads.
    shake_camera: (magnitude: number, ticks: number) -> (),
    navi: (side: number) -> Navi,
    emotion: (side: number) -> Emotion,
    -- Unless the navi's emotion is held.
    set_mood: (side: number, mood: number) -> (),
    -- A side's bug frags, and spending some (its callers check there are
    -- enough: the count wraps below 0).
    bug_frags: (side: number) -> number,
    spend_bug_frags: (side: number, count: number) -> (),
    -- `sub_802E4B8`: the side's SELECT or Cross special in progress.
    side_special: (side: number) -> SideSpecial,
    player: (side: number) -> Object?,
    -- A side's combatants still in, in slot order.
    alive_actors: (side: number) -> { Object },
    -- The objects of a kind in the update list, in update order, whatever
    -- their lifecycle state (the game's walks of the list).
    objects_of: (kind: Kind) -> { Object },
    -- `GetRNG2` (one draw), `GetPositiveSignedRNG2` (bit 31 cleared).
    rng: () -> number,
    rng_positive: () -> number,
    -- `AddRandomVarianceToTwoCoords`: x and z by up to mask/2 pixels.
    jitter: (mask: number, pos: Vec3) -> Vec3,
    -- A side's hand: its cursor.
    hand_cursor: (side: number) -> number,
    advance_hand: (side: number) -> (),
    linked: (side: number) -> LinkedChip,
    set_linked: (side: number, record: LinkedChip) -> (),
    clear_linked: (side: number) -> (),
    -- `sub_80E49C4` (BugFix): a side's NaviCust bugs are fixed (the stats
    -- processing, the panel trail's level, the buster's blanks, the on-hit
    -- status, the custom damage, the emotion, the drains, the battle-start
    -- bug and the hand-shrink turn zeroed).
    clear_navicust_bugs: (side: number) -> (),
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
    -- if the pool is full.
    spawn: (kind: Kind, pos: Vec3?) -> Object?,
    -- `sub_80033E4`: the same at the head of the update list (it first
    -- runs next tick, before everything else).
    spawn_first: (kind: Kind, pos: Vec3?) -> Object?,
    -- `sub_8003374` / `sub_800333C`: the same, at the end of the update
    -- list (it runs after everything else this tick).
    spawn_at_end: (kind: Kind, pos: Vec3?) -> Object?,
    -- The generic one-shot effect (effect object #0) an effect definition
    -- describes.
    effect: (pos: Vec3, effect: Effect, flip: number?, palette_add: number?, priority: number?) -> Object?,
    -- `sub_801BD3C`: the one-shot effect `effect` on each field panel of
    -- hit region `region` around (x, y), turned the way side `side` faces,
    -- at height z (default 0); a whole-field region's from the bottom
    -- right, on the ground.
    region_effects: (x: number, y: number, region: Region, side: number, effect: Effect, z: number?) -> (),
    -- The one-tick hit region (attack object #3).
    hitbox: (owner: Object, spec: HitboxSpec) -> Object?,
    -- `sub_80E11E0`: a screen palette flash (effect object #0x0A) of a
    -- variant (0 white or red, 1 white over two layers) for `ticks`,
    -- optionally going on while dimmed or paused.
    palette_flash: (variant: number, ticks: number, while_dimmed: boolean?, while_paused: boolean?) -> Object?,
    -- A hit spark (effect object #4) a spark definition describes.
    spark: (owner: Object, pos: Vec3, spark: Spark) -> Object?,
    -- `sub_80C468C`: a form overlay (actor object #0x57) on `owner`,
    -- following it every tick.
    form_overlay: (owner: Object, spec: FormOverlaySpec) -> Object?,
    -- Subtype 18 (Otenko): the turn (from 0) a hand's chip at i was picked
    -- in, and adding to its Atk+ bonus.
    hand_turn: (side: number, i: number) -> number,
    add_hand_attack_bonus: (side: number, i: number, n: number) -> (),
    -- A side's hand has a chip at i and it does damage (its record's
    -- "has_damage" flag).
    hand_chip_damages: (side: number, i: number) -> boolean,
    -- Subtype 8 (Wind and Fan): a side's wind (`sub_80E543C`) and who
    -- placed it; `sub_80E541A` makes `me` a side's wind (the one there
    -- before is destroyed at once); `sub_80E544C` makes it no side's.
    wind: (side: number) -> (Object?, WindSource),
    set_wind: (me: Object, side: number, source: WindSource) -> (),
    clear_wind: (me: Object) -> (),
    -- `sub_80E33FA`: a plain afterimage (effect object #0x28) of `owner`'s
    -- side at `pos`: a copy of a sprite that stays behind.
    afterimage: (owner: Object, pos: Vec3, spec: AfterimageSpec) -> Object?,
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

-- Where an object's HP number shows (`battle.show_hp`): `dx`, `dy` pixels
-- from where the HUD projects its position (0 if nil), the digits centred
-- there; with `damage`, the damage it took (its max HP less its HP), its
-- digits right-aligned in four places from there (what the original does
-- for LilBoiler).
export type HpNumberSpec = { dx: number?, dy: number?, damage: boolean? }

-- What the telop of a dimming content starts itself names (`dimming.start`):
-- a chip (nil: a zeroed chip field's, the roles' `zeroed` chip), and the
-- Atk+ / cross bonus shown after its damage (0 if nil).
export type DimmingTelop = { chip: Chip?, bonus: number? }

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
    -- A navi chip's: `sub_800BDB2` (AntiNavi) and `sub_800BA8A` (its
    -- telop), for the chip the controller shows (nil: a zeroed chip
    -- field's, the roles' `zeroed` chip).
    check_anti_navi: (controller: Object, chip: Chip?) -> (),
    show_navi_telop: (controller: Object, chip: Chip?) -> (),
    -- `object_undimScreen`.
    undim_screen: (controller: Object) -> (),
    -- `sub_800BF16`: `side` starts a dimming with `controller` (nil if its
    -- spawn failed), used by `user`; `no_cut_in`: the other side can't cut
    -- in. For controllers no chip action registers (a trap springing).
    -- `telop`: what the controller's telop names (the chip and bonus its
    -- spawn stores); without one the telop names no chip.
    start: (side: number, no_cut_in: boolean, controller: Object?, user: Object, telop: DimmingTelop?) -> (),
    -- `object_timefreezeEnd` (the controller's destroy state).
    finish: (controller: Object) -> (),
    -- `sub_80E1352` / `sub_80E13DC`: a navi chip's user vanishes and comes
    -- back.
    hide_user: (user: Object) -> (),
    show_user: (user: Object) -> (),
    -- `sub_80E1352(user, 0xF)`: the user vanishes, but its barrier visual,
    -- its confusion and blindness visuals and the HUD stay.
    hide_user_sparing: (user: Object) -> (),
}

declare navi_chip: {
    -- The navi is done: its controller moves on.
    navi_left: (controller: Object) -> (),
    -- `sub_80E1332`: the user warps out (`out`) or back in.
    warp: (user: Object, out: boolean) -> (),
}

-- What a stage places a field object with (the original's spawner for
-- the actor list's entry, `off_80073A0[type]`): an object kind's
-- `place(spec)`.
export type PlaceSpec = {
    panel_x: number,
    panel_y: number,
    -- The entry's side (field objects take their panel's instead).
    side: number,
    -- Which of the kind's variants the stage names (a record of the
    -- kind's: one of objects/rock's `variants`).
    variant: any?,
    -- The entry's raw argument (0 unless the stage gives one), for a
    -- spawner that only leaves it in a register.
    argument: number,
}

-- How an absorbed obstacle looks (objects/absorbed-obstacle's `look`, an
-- "absorbed-look" record; the original's obstacle kinds, `byte_80E98C0`):
-- its sprite; `keeps_parts`, it flies with the obstacle's hidden sprite
-- parts (the original's kind 2); `own_facing`, thrown, its sprite's parts
-- keep their own facing and it isn't flipped (sprite 0x23 in the
-- original's test).
export type AbsorbedLook = { sprite: SpriteAsset, keeps_parts: boolean?, own_facing: boolean? }

export type ObstacleCrush = "breaks" | "destroys" | "spares_bodies" | "ignores"
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
    -- ("spares_bodies": bodies don't break it) or ("ignores": `sub_801B878`
    -- while LilBoiler erupts, such a touch is as any hit): damage, removal
    -- and status;
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
    -- `sub_800F90E`: absorbed, it flies to the absorbing navi as an
    -- absorbed obstacle of look `look` (objects/absorbed-obstacle's
    -- `look { ... }`; the original's obstacle kind).
    fly_to_absorber: (me: Object, look: AbsorbedLook) -> (),
    -- `sub_802EF5C`: the per-side target tracking some chips keep.
    release_tracking: (me: Object) -> (),
    -- The field's two stage objects (the registry's stage slots, which the
    -- actor lists' entry types 3 and 0xA fill; `sub_8007450`): whether one
    -- is free; `me` takes the first free one; `me` leaves the one it holds
    -- (the original clears the slot the object remembers).
    stage_slot_free: () -> boolean,
    enter_stage: (me: Object) -> (),
    leave_stage: (me: Object) -> (),
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
    -- thrown back as junk: its identity, which its taker keeps in an
    -- "identity" state field (`me:wear_junk_look`); nil for the objects
    -- DustMan leaves (`sub_800F486`: an identity with `scrap = false`).
    junk_look: (o: Object) -> Identity?,
    -- `sub_80DC3B2`'s test: a field object by its identity, but those
    -- `sub_800F486` excludes (`scrap = false`): what BlzrdBal's ball
    -- swallows.
    swallowable: (o: Object) -> boolean,
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
    -- What places it when a stage names it among its `actors`: it returns
    -- what it placed, if anything.
    place: ((spec: PlaceSpec) -> Object?)?,
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
    -- Ticks to a full charge, by Charge stat 0 to 7 (past 4, what the
    -- original reads on into: the next routine's row).
    charge_ticks: { number },
    -- `off_80117D4[n]`: prepare the attack, name its action (an `Action`).
    -- A weapon with an `instant` effect names none: the instant chips'
    -- action runs it. None only for an A-charge that is its chip
    -- (`charged_chip`), which nothing starts as a weapon.
    -- (Typed loosely: Luau's checker doesn't check a function against an
    -- optional function type.)
    setup: ((navi: Object) -> any)?,
    -- Its instant effect, which the instant chips' action runs once: a
    -- chip's (the arm chips' weapons run Boomer's and FireHit's), or one no
    -- chip has (TenguCross's wind), after which the navi waits 8 ticks
    -- (`instant_waits`).
    instant: ((user: Object, spec: InstantChipSpec) -> ())?,
    instant_waits: boolean?,
    -- As a charged shot: it stays when a form's own would replace it (a
    -- chip's weapon, `sub_800FFAA`), and its attack runs through a dimming.
    sticky: boolean?,
    -- As a buster: it fires while B is held.
    held: boolean?,
    -- As a buster: the buster it gives way to while the charged shot is a
    -- sticky one.
    plain: Weapon?,
    -- As an A-charge without an attack of its own: the charged chip is the
    -- chip itself with the charge's bonus ("bonus"), or that after
    -- GroundCross's falling rocks ("rock_barrage").
    charged_chip: ("bonus" | "rock_barrage")?,
}
export type Weapon = WeaponSpec

-- A chip: its record, and exactly one use.
-- A chip's damage by formula: the opponent's HP; an SP navi chip's, by its
-- navi's deletion time (its slot of the rules' `sp-chips`, and the damage
-- for each of that section's time steps); the custom gauge's; the HP the
-- user has lost; the last two digits of the user's HP; half the opponent's
-- max HP; a link navi's own chip's, by its level.
export type DamageFormula =
    { formula: "opponent_hp" }
    | { formula: "sp_navi", slot: string, by_time: { number } }
    | { formula: "gauge" }
    | { formula: "hp_lost" }
    | { formula: "hp_last_digits" }
    | { formula: "half_opponent_max_hp" }
    | { formula: "navi_level", base: number, per_level: number }

export type ChipSpec = {
    id: string,
    name: string,
    -- What R shows on the custom screen, its lines apart by "\n": the box
    -- takes keys a tick later for each line. None counts as three lines.
    description: string?,
    codes: { string }?,
    element: string?,
    family: string?,
    class: string?,
    rarity: number?,
    mb: number?,
    -- A number below 1000, or a formula the ruleset computes when the chip
    -- enters a hand.
    damage: (number | DamageFormula)?,
    hit_param: number?,
    lockout: number?,
    flags: { string }?,
    -- Named bits, and the value of a bit the engine doesn't name yet.
    extra_flags: { string | number }?,
    -- Its lock-on mode, and whether it goes through the Beast rush (`rush
    -- = false`: a mode without the rush, as some records have).
    beast: { lockon: Lockon?, rush: boolean? }?,
    library: { number: number, index: number, sort: number }?,
    slot_in_limit: number?,
    -- What the ruleset asks of the chip: "no_chain" (the Beast rush's
    -- chain leaves it out), "aura_bonus" (it hits harder while the user's
    -- barrier holds), "no_cut_in" (the other side can't cut in on its
    -- dimming), "element_sword" (a link navi's own chip's family bonus
    -- counts it), "navi_slot" (AntiNavi turns it back though it lacks the
    -- `navi` flag), "heals" (AntiRecv springs on it), "user_stays" (a navi
    -- chip whose user isn't warped off the field), "navi_returns_user" (a
    -- navi chip whose navi brings the user back itself).
    traits: { string }?,
    -- The trap it is while a side's defensive-chip record holds it:
    -- "anti_damage", "anti_sword", "body_guard", "anti_navi",
    -- "anti_recovery".
    trap: string?,
    -- A dark chip: the chip its user gets without bug frags, and what its
    -- use adds to the user's HP bug.
    dark_substitute: Chip?,
    hp_bug: number?,
    modifier: any?,
    program_advances: { any }?,
    -- Its use: exactly one of these.
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
-- Something a stage places when the round starts: a navi of `side`, or a
-- field object by its kind (which needs a `place`), with one of the kind's
-- variants or a raw argument where the kind takes one.
export type StageActor = {
    place: "navi" | Kind,
    side: number?,
    x: number,
    y: number,
    variant: any?,
    argument: number?,
}
-- A stage (§3.7): its panels, three rows of six panel types (`"normal
-- normal cracked ..."`), which columns are whose, its music outside link
-- battles (none: no music starts), and what it places, in order.
export type StageDef = {
    id: string,
    layout: { string },
    panel_pattern: number,
    music: Sound?,
    background: Background,
    mode: number,
    battle_number: number,
    effects: number?,
    actors: { StageActor },
}
-- A status (rules/status): the requests it raises on its target
-- (`ObjectFlags2` bits), for how long, and the timer it runs on.
-- `cancels_flinch`: the hit that lands it doesn't flinch or flash its
-- target (the freezing statuses). `survives_counter`: a counter hit that
-- lands it keeps it rather than paralyzing (the bubbling statuses).
export type StatusDef = {
    id: string,
    requests: number,
    duration: number,
    timer: string | { other: number },
    cancels_flinch: boolean?,
    survives_counter: boolean?,
}
export type Status = StatusDef
-- A Beast Out lock-on mode (rules/lockon): how the rush picks the panel to
-- attack a target from. `stay`: the navi's own panel. `row`: along the
-- target's row counting from the navi's own column (`same_row_offsets`
-- when they share a row), then the rows `row_shifts` away. `near`: the
-- first of `offsets` (dx toward the user's front) next to the target the
-- navi can stand on (`far_column_offsets` for a target in the column
-- farthest ahead), then with the rules' column shifts, with a clear path
-- to the target if asked; afterwards the middle row of that column, if
-- preferred and free.
export type LockonDef = {
    id: string,
    rule: "stay" | "row" | "near",
    offsets: { { number } }?,
    same_row_offsets: { { number } }?,
    row_shifts: { number }?,
    far_column_offsets: { { number } }?,
    column_shifts: boolean?,
    clear_path: boolean?,
    prefers_middle_row: boolean?,
}
export type Lockon = LockonDef

-- What an object is taken for (§3.2): the original's NameID record. A
-- navi's and a form's is nested in its definition (`identity =
-- define.identity { ... }`); a field object's is its kind's.
--   megaman, link_navi, cross, beast, cross_beast, beast_over: a player's
--     navi (MegaMan in his base form, a link navi, MegaMan's forms);
--   field_object: a rock, a statue, a mine;
--   virus, navi, gregar, falzar: what no netbattle object is (an object
--     with no identity is a virus).
export type IdentityClass =
    "megaman"
    | "link_navi"
    | "cross"
    | "beast"
    | "cross_beast"
    | "beast_over"
    | "field_object"
    | "virus"
    | "navi"
    | "gregar"
    | "falzar"
export type IdentitySpec = {
    id: string?,
    class: IdentityClass,
    -- The actor record (`byte_80182C4`): its version, its actor type (by
    -- default its class's), and the AI index that picks its hooks.
    version: number?,
    actor_type: ActorType?,
    ai_index: number?,
    -- A navi's sprite attach points, in pixels, x toward the facing side.
    attach_points: { { number } }?,
    -- Where a held thing with no attach point sits ({ x, z }; none: 13
    -- pixels up).
    held_offset: { number }?,
    -- A field object's look (`byte_8021220`): what something that takes
    -- its place shows (no sprite: nothing). `keeps_flip`: what wears it
    -- keeps its own flip.
    look: { sprite: SpriteAsset?, anim: number?, palette: number?, shadow: boolean?, keeps_flip: boolean? }?,
    -- A field object the obstacle-absorbing action and ColArmy leave
    -- (`absorbable = false`), and one nothing swallows or leaves as junk
    -- (`scrap = false`).
    absorbable: boolean?,
    scrap: boolean?,
}
export type Identity = IdentitySpec

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

-- What the ruleset needs from content by role (rules/roles.luau, once):
-- each role names the definition that fills it.
export type RolesSpec = {
    actions: {
        anti_damage_counter: Action?,
        anti_sword_counter: Action?,
        body_guard_counter: Action?,
        forced_charged_shot: Action?,
        stun_strike: Action?,
        cross_protect: Action?,
        turn: Action?,
        cross_death: Action?,
        volley: Action?,
        charged_sword: Action?,
        beast_claw: Action?,
        dust_beast_scatter: Action?,
        charge_tackle: Action?,
    }?,
    kinds: {
        absorbed_obstacle: Kind?,
        falling_rock: Kind?,
        support: Kind?,
        anti_recovery: Kind?,
        -- What the player whose AI index is 10 spawns in battle mode 9
        -- (the original's attack object #0xD2 and actor object #0x28).
        mode9_attack: Kind?,
        mode9_actor: Kind?,
    }?,
    -- The chips the ruleset names: what a zeroed chip field reads, the
    -- custom screen's Beast Out chip and its invalid selection's, and the
    -- chips the supports' telops name.
    chips: {
        zeroed: Chip?,
        beast_out: Chip?,
        invalid: Chip?,
        rush: Chip?,
        beat: Chip?,
        tango: Chip?,
    }?,
    -- The statuses the ruleset inflicts itself: what a damage word's
    -- paralysis bit carries, what a counter hit lands, what an aqua hit
    -- gives a body on ice, and the NaviCust on-hit bug's.
    statuses: {
        damage_word_paralysis: Status?,
        counter_paralysis: Status?,
        ice_freeze: Status?,
        hit_bug_blind: Status?,
        hit_bug_confuse: Status?,
    }?,
    -- The Beast Out lock-on modes the ruleset names itself: the Beast
    -- forms' charged claw's.
    lockon: {
        beast_claw: Lockon?,
    }?,
    -- The one-shot effects the ruleset shows itself: a navi deleted (and
    -- a cross merged, a navi arriving, a form put on or taken off), HP
    -- recovered, a side cutting in, a trap springing, an obstacle encased,
    -- a form change, Beast Over's beast, blast and bursts.
    effects: {
        deletion: Effect?,
        recovery: Effect?,
        cut_in_flash: Effect?,
        trap_mark: Effect?,
        encased: Effect?,
        form_change: Effect?,
        beast_over_gregar: Effect?,
        beast_over_falzar: Effect?,
        beast_over_blast: Effect?,
        beast_over_burst: Effect?,
    }?,
    -- The hit sparks the ruleset shows itself: a new registration's until
    -- its owner names another, a blocked hit's, an eruption's, a thrown
    -- obstacle's landing's, an uninstall's.
    sparks: {
        plain: Spark?,
        guard: Spark?,
        eruption: Spark?,
        thrown_obstacle: Spark?,
        uninstall: Spark?,
    }?,
    -- The hit regions the ruleset registers itself: a registration's own
    -- panel, which every setup gives it.
    regions: {
        anchor: Region?,
    }?,
    -- The collision types the ruleset registers itself: a navi's body
    -- (floating or not) and what it reacts to, an eruption and a thrown
    -- obstacle's landing and what they reach.
    collision: {
        navi: CollisionType?,
        floating_navi: CollisionType?,
        navi_target: CollisionType?,
        eruption: CollisionType?,
        eruption_target: CollisionType?,
        thrown_obstacle: CollisionType?,
        thrown_obstacle_target: CollisionType?,
    }?,
    -- The sounds the ruleset plays itself (rules/roles.luau says what each
    -- is for).
    sounds: {
        panel_crack: Sound?,
        panel_poison: Sound?,
        own_hit: Sound?,
        hit: Sound?,
        damage: Sound?,
        guard: Sound?,
        counter_hit: Sound?,
        deleted: Sound?,
        recovery: Sound?,
        damage_bonus: Sound?,
        pause: Sound?,
        gauge_full: Sound?,
        low_hp: Sound?,
        cut_in: Sound?,
        telop: Sound?,
        buster_charge: Sound?,
        buster_charged: Sound?,
        freeze: Sound?,
        bubble: Sound?,
        bubble_pop: Sound?,
        confusion: Sound?,
        invisible: Sound?,
        appear: Sound?,
        arrive: Sound?,
        fade: Sound?,
        obstacle_lift: Sound?,
        obstacle_throw: Sound?,
        cross_merge: Sound?,
        form_change: Sound?,
        cross_change: Sound?,
        cross_change_chime: Sound?,
        beast_out: Sound?,
        gregar_roar: Sound?,
        falzar_roar: Sound?,
        beast_over_rumble: Sound?,
        beast_over_burst: Sound?,
        cross_special: Sound?,
        refused: Sound?,
    }?,
    -- The music the ruleset starts itself: a link battle's, the winner's
    -- (in special battles, and otherwise) and the loser's. A stage's music
    -- is its own.
    music: {
        link_battle: Sound?,
        winner_special: Sound?,
        winner: Sound?,
        loser: Sound?,
    }?,
    -- The sprites of the engine's own kinds: the charge glows, the Full
    -- Synchro aura, the status visuals, the ice block and the bubble, the
    -- hit marker, the eruption, the lock-on marker, the Beast head, the
    -- idle overlay.
    sprites: {
        charge_glow: SpriteAsset?,
        charge_glow_a: SpriteAsset?,
        full_synchro_aura: SpriteAsset?,
        confusion: SpriteAsset?,
        blindness: SpriteAsset?,
        immobilized: SpriteAsset?,
        ice: SpriteAsset?,
        bubble: SpriteAsset?,
        hit_marker: SpriteAsset?,
        eruption: SpriteAsset?,
        lockon_marker: SpriteAsset?,
        idle_overlay: SpriteAsset?,
        beast_head: SpriteAsset?,
    }?,
    -- The banners the ruleset shows itself: a round's start, a turn's, the
    -- final turns', a draw, the damage judge's, a telop's on either side,
    -- a Program Advance's and an empty selection's. The win and lose
    -- banners are the navi's.
    banners: {
        round_start: Banner?,
        turn_start: Banner?,
        final_turn: Banner?,
        draw: Banner?,
        judge: Banner?,
        telop: Banner?,
        telop_remote: Banner?,
        program_advance: Banner?,
        program_advance_empty: Banner?,
    }?,
    hooks: {
        -- The NaviCust FirstBarrier: raise its barrier and the barrier's
        -- visual on the navi as it enters (`sub_8013892`).
        first_barrier: ((navi: Object) -> ())?,
        -- `sub_801813A`'s end: what an obstacle encased in ice (`ice`) or a
        -- bubble becomes on its panel; its field-object registry class (nil:
        -- it wasn't registered).
        encased: ((obstacle: Object, ice: boolean, class: number?) -> ())?,
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
    identity: (spec: IdentitySpec) -> Identity,
    -- A rule section (§3.8): one table of rules no entity owns, by its
    -- section name (`define.rules("elements", { ... })`).
    rules: <T>(section: string, spec: T) -> T,
    roles: (spec: RolesSpec) -> RolesSpec,
    -- Luau-only data with a handle and a type name, which a reference field
    -- ("record:<type>") can hold.
    record: <T>(record_type: string, spec: T) -> T,
}

-- Deprecated. `legacy { ... }`: data the ruleset still reads by number (a
-- navi's or a form's number and NameID; the original's numbering of a
-- table), until its last users convert (docs/design/content-model-v2.md
-- §12). As data it is the table itself; the checker counts each use.
declare function legacy(t: any): any
