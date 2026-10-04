-- The battle engine's content API, as Luau content sees it. A definitions
-- file for editors and the type checker (luau-lsp:
-- `--definitions=content/nettai/core.d.luau`, with a root's own, as
-- `--definitions=content/bn6/types.d.luau`); every root checks against it
-- (docs/design/rules-in-luau.md §7.2: content/nettai declares the engine's
-- API and defines nothing; BN6's own parts move to content/bn6 in slice
-- S8). The engine implements it in
-- crates/nettai-luau/src/bind.rs over the Rust `CoreApi` trait
-- (crates/nettai-content-api), and docs/design/scripting.md describes it.
--
-- How content runs (content model v2, docs/design/content-model-v2.md).
-- Every module of a pack loads once, and while it loads it makes
-- definitions with `define.<registry>(spec)` (a chip, a kind, an action, an
-- effect...) and names assets with `asset.<kind>(name)`; a definition is its
-- spec table, frozen, and scripts pass it where the API takes one
-- (`battle.spawn(bomb.kind, pos)`, `me:set_attack(action, 2)`). The
-- definitions and their types are at the end of this file.
--
-- Nothing is named by one of the original's numbers: content names
-- content by its definitions and assets by name. The original's numbers
-- are compat's (content/bn6/compat), which only the tools outside the
-- engine read (the trace harness, save import).
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
export type Emotion = "normal" | "tired" | "full_synchro" | "angry" | "worn_out" | "worried"
-- A side's SELECT or Cross special in progress (battle flag 0x40 mode).
-- A side's special in progress: the SELECT special, or a system's takeover
-- of the side's navi (BN6's Cross special).
export type SideSpecial = "none" | "select" | "takeover"
-- A HUD part a chip's effect hides and shows: the custom gauge (draw task
-- 4), the emotion window (14), the battle flag 0x40 mode's gauge drawn by
-- its levels (17).
export type HudPart = "gauge" | "emotion_window" | "level_gauge"
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
    -- BN5's (docs/design/bn5-map.md §15.2).
    | "metal"
    | "lava"
    | "sea"
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
    -- No hit reaches it and it hits nothing (Falzar Beast Over's form,
    -- BN5's DarkInvs drive).
    | "untouchable"
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
    | "navi_switch"
    | "switch_knockout"
    | "mode9_a"
    -- A system's takeover of the side's navi (BN6's Cross special).
    | "takeover"
    | "volley"
    | "weakness_hit"
    -- The slide request (the collision's flag2 0x10).
    | "slide"
    -- The anger request (the collision's flag2 0x200): angry at the next
    -- status update, unless the mood is held.
    | "anger"
-- A navi's state bits.
export type NaviStateFlag =
    "controllable"
    | "chip_in_progress"
    | "form_change"
    | "reverting_form"
    | "no_charge"
    | "can_turn"
    | "trap_armed"
    | "switching_navi"
    | "switch_knockout"
    | "switched"
    | "volley"
    | "uninterruptible"
    | "cross_breaking"
    | "form_change_sprite_held"
    | "heat_trap"
    -- Gone from the field while its navi chip's navi acts.
    | "vanished"
    -- It dives: a panel that submerges (BN5's sea) submerges it and doesn't
    -- hold it at a move's end (BN5's ToadSoul). The status reset ends it.
    | "dives"
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
    -- The bug code and argument the last resolution's hits brought (CollisionData+0x9C, +0x9D), which the navi takes.
    inflicted_bugs: number
    hit_mod_base: number
    -- The damage it deals.
    self_damage: number
    -- The counter byte (bits 0-6 counter strength, bit 7 can't counter),
    -- which setup takes from the damage word's high half.
    counter_byte: number
    -- The ticks left of its counter window (+0x0D; read-only): a hit with a
    -- counter byte counters it while they run.
    counter_timer: number
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
    -- BattleObject +0x1A (read-only): a player's chips left in its hand as
    -- the HUD shows them; another actor's own count (BN5's Jealousy).
    chips_held: number
    pos: Vec3
    vel: Vec3
    related1: Object?
    related2: Object?
    -- The slot is in use (read-only).
    active: boolean
    -- Drawn (for every viewer, when set; read: the local side's console's).
    -- What a console's rule hides per viewer is `hide_from_blind`, and an
    -- attachment follows its owner with `copy_visibility`.
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
    -- The attack's variant (AIAttackVars+3), as the anti-damage counters
    -- read it: what a stance's weapon sets, 0 from a trap's catch.
    attack_variant: number
    charged: number
    -- Lockout the attack applies when it ends.
    attack_lockout: number
    -- The Atk+ / cross bonus.
    extra: number
    special_source: number
    -- Which set_attack kind started it (read-only).
    attack_kind: number
    -- 1 while the action runs inside the side's wrapper (the role
    -- `actions.wrapper`: BN6's Beast Out rush).
    wrapped: number
    -- The wrapper's state starts over (`sub_801011A` sets it with the
    -- attack's links); the wrapper clears it once it has.
    wrapper_fresh: boolean
    -- The controller's state starts over (a form's `berserk` effect); the
    -- controller clears it once it has.
    controller_fresh: boolean
    -- Its mood is held (held tired or exhausted): what another side's rules
    -- read (BN6's counter).
    mood_held: boolean
    -- The side's systems' `navi_tick` runs for it each tick.
    ticked: boolean
    -- Exhausted for the rest of the battle (BN6's after Beast Over): worn
    -- out, the mood can't change, 1 HP lost per tick (never the last), no
    -- Full Synchro or anger.
    exhausted: boolean
    -- What a step or the wrapper turns to face in the panel patterns 0x23,
    -- 0x31 and 0x33 (read-only; only the unused `sub_80116F6` sets one).
    face_target: Object?
    -- The lock-on mode the attack's own action asks the Beast Out rush for
    -- (the charged sword's, by its slash); nil: the chip's.
    rush_lockon: Lockon?
    -- A per-action word some actions keep.
    marker: number
    -- The absorbed obstacle a throw carries (the marker word, as
    -- DustCross's throws keep it): its look and animation.
    thrown_look: AbsorbedLook?
    thrown_anim: number
    -- The attack's count (AIAttackVars+0x12): what an action counts (its
    -- shots, swings, slashes, a hold's ticks, the recovery after a shot).
    -- Every action shares the word and none clears it, so an action that
    -- reads it before writing it reads what an earlier one left (a thrown
    -- obstacle waits the last shot's recovery). An action keeps its count
    -- here, not in its state, as the original's do.
    attack_count: number
    -- The actor data (read-only unless noted).
    actor_type: ActorType
    ai_index: number
    -- The sprite overlay attached for the current attack (writable).
    overlay: Object?
    -- Writable links: the target marker (BN6's Beast Out lock-on marker),
    -- the charge glow, the
    -- Full Synchro aura.
    target_marker: Object?
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
    -- Writable: held tired (AIData+0x32; BN6's "the Beast Out counter is
    -- spent"): emotion 1, the mood held, no anger.
    tired: boolean
    -- Writable: the barrier's visual (AIData+0x60), which hides and shows
    -- with the navi (its state's `shown`).
    barrier_visual: Object?
    -- Writable: BN5's DarkPlus tint (AIData+0x3C; 0 none), which picks the
    -- navi's status shader.
    plus_tint: number
    -- Writable: BN5's Chaos Unison charge, armed (AIData+0x12; the chaos
    -- change arms it, a weapons' load and the failure's end disarm it), and
    -- its level (AIData+0x6C: releases that succeeded, at most 4).
    chaos_armed: boolean
    chaos_level: number
    -- Writable: BN5's no-charge drive (DarkInvs): its ticks left
    -- (AIData+0x36; counted down in the intake while the navi has the
    -- no-charge state, asking for the stun strike at 0; 0xFFFF holds), and
    -- the computer-navi AI driving it (AIData+0xF0, which VarSwrd's pick
    -- reads).
    no_charge_timer: number
    computer_driven: boolean

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
    -- `sub_8010DF6`: put on what an identity wears (its `parts`: a body
    -- overlay, SpoutMan's idle overlay, a beast head), kept in related2
    -- (CircusMan's second one kept by the engine); `arg` 1: their sprites
    -- step even while paused.
    function add_parts(self, identity: Identity, arg: number): ()
    -- `sub_8011044`: take them off (at their next update).
    function remove_parts(self, identity: Identity): ()
    -- `sub_8010DF6` with `owner`'s identity: put on the parts it wears (a
    -- navi's body overlay), kept in related2; with
    -- `keep_stepping`, a part that came steps even while paused and dimmed.
    -- With `paused_stepping`, `sub_8010DF6`'s r2 is 1 rather than the
    -- record's first byte (the dimming chips' stand-ins): the parts step
    -- even while paused.
    function add_parts_of(self, owner: Object, keep_stepping: boolean?, paused_stepping: boolean?): ()
    -- `sub_8011044` with `owner`'s identity: take them off.
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
    -- `sub_80169BE`: shown unless the battle is dimmed; hidden from a
    -- viewer whose navi is blind if it is the other side's.
    function update_visibility(self): ()
    -- `sub_800EB6C` on each console: hidden from a viewer whose navi is
    -- blind if it is the other side's, on top of what each viewer sees of
    -- it now. (Presentation, decided per viewer: `visible` is for every
    -- viewer.)
    function hide_from_blind(self): ()
    -- Seen by whoever sees `owner` (each viewer's view of it): an
    -- attachment following its owner's visibility.
    function copy_visibility(self, owner: Object): ()
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
    -- `sub_800A772`: its side's chips are enabled and its chip lockout over.
    function chips_enabled(self): boolean
    -- `sub_8010332`: the ticks a step of its ends with.
    function move_lag(self): number
    -- As a navi leaves (BN5's 0x081042E6): its status visuals forgotten and
    -- its chips off the HUD.
    function drop_links(self): ()
    -- A navi no player controls leaves (BN5's 0x08104306): no HP, the
    -- damage-carry record forgets it, its reservation goes, it leaves its
    -- side's lists and is destroyed.
    function leave(self): ()
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

    -- What a game's rules do to a navi (docs/design/rules-in-luau.md §4.5).
    -- The invulnerability ends (its timer and flag).
    function clear_invulnerable(self): ()
    -- `sub_800F46C` + `sub_800F2C6`: face the default way under the
    -- standard column patterns, and the sprite with it.
    function face_default(self): ()
    -- The charge drops: its counters and the hold requests.
    function reset_charge(self): ()
    -- `sub_80C4C3A`: the Full Synchro aura goes.
    function end_full_synchro_aura(self): ()
    -- `sub_80158FA`: movement, reactions and the slower statuses end.
    function drop_statuses(self): ()
    -- `sub_801A264`: a navi's statuses end: their flags, requests and
    -- timers (not the whole status word, as `clear_statuses`).
    function end_statuses(self): ()
    -- The navi's overlay (related2) keeps stepping through pauses and
    -- dimming ("keep": its Param3 1, flags 0x14) or steps like any object
    -- ("normal"). Nothing without an overlay.
    function overlay_stepping(self, mode: "keep" | "normal"): ()
    -- `sub_8011384`, `sub_8011268`: take off what a form wore, put on what
    -- it wears.
    function take_off_form_overlay(self, form: FormDef): ()
    function put_on_form_overlay(self, form: FormDef): ()
    -- `sub_800FC9E` and a form change's load: the navi's sprite in `form`,
    -- its animation 0 from the start, shadow on the ground, facing, white.
    function load_form_sprite(self, form: FormDef): ()
    -- `sub_80144C0`: the full status reset.
    function reset_status(self): ()
    -- Anger ends; the mood is back to 0x80 (BN5's: a mood of 0 stays).
    function end_anger(self): ()
    -- The form the navi's side asked to change into at this turn's start
    -- (nil: none, or the base form).
    function form_change_target(self): FormDef?
    -- BN5's Soul Unison: the turns the soul asked for lasts, and whether it
    -- is Chaos Unison.
    function form_change_soul(self): (number, boolean)
    -- BN5's soul change's first step: no longer moving, flinching,
    -- paralyzed or sliding (nor asked to slide).
    function stop_moving(self): ()
    -- `sub_80C4526(overlay, 1)`: an overlay on an image sits in front (an
    -- idle overlay keeps its owner's height).
    function pin_overlay(self): ()
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
    -- `loc_80126EA`: `chip` as the navi's attack (its record's damage, hit
    -- parameter, lockout and element; no bonus; not charged): a weapon
    -- that fires a chip, whose setup then returns the chip.
    function load_chip_attack(self, chip: Chip): ()
    -- `sub_8010740`: in a link battle the opponent's support Rush takes
    -- `chip` (one it cancels) from this navi: it comes, and is spent.
    -- Whether it did.
    function rush_cancels(self, chip: Chip): boolean
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
    -- `sub_800EDD0`'s chip: the hand's chip at the cursor (nil: the hand is
    -- empty).
    function next_chip(self): Chip?
    -- `sub_800FB54`: use the chip the navi's requests ask for (as idle does
    -- on A): whether its use started.
    function use_chip(self): boolean
    -- The attack `chip` starts (`sub_80127C0`'s dispatch: its action, or the
    -- dimming, navi or instant chip's), in `set_attack` slot `kind`.
    function start_chip_attack(self, chip: Chip, kind: number): ()
    -- `sub_80116AE(5, end_lag, 2)`: a step straight to (x, y) (column 0: no
    -- step), then `end_lag` ticks; with `face`, BN5's `sub_80116F6`'s (what
    -- the step turns to face in the panel patterns 0x23, 0x31 and 0x33).
    function start_move_to(self, x: number, y: number, end_lag: number, face: Object?): ()
    -- `sub_80117BA`: `weapon`'s setup, and the action it names started in
    -- `set_attack` slot `kind`.
    function start_weapon(self, weapon: Weapon, kind: number): ()
    -- The wrapper's (the role `actions.wrapper`): run the action the
    -- navi runs (`sub_801B9E6`'s attack).
    function run_wrapped(self): ()
    -- `sub_800FC30`: start the next chip in the hand inside the wrapper
    -- (prepared, its action started, `wrapped` 1), unless it has the
    -- `no_chain` trait, is a dimming chip, or the hand is empty. Whether it
    -- did.
    function chain_next_chip(self): boolean
    -- `sub_8013CC4`: the NaviCust panel-trail bugs on the panel (x, y) the
    -- navi leaves.
    function panel_trail(self, x: number, y: number): ()
    -- On a target marker (a navi's `target_marker`): `sub_80E1654` (true:
    -- it holds where it is, looking locked on) and `sub_80E1662` (false: it
    -- follows its target again).
    function freeze_target_marker(self, on: boolean): ()
    -- `sub_800F2FC`: turn to face `target`'s column.
    function face_toward(self, target: Object): ()
    -- `sub_80DBB64`: put a junk look (`obstacle.junk_look`) on the sprite:
    -- the sprite, a shadow if the look has one, its animation and palette,
    -- flipped by the object's side unless the look keeps its own; false
    -- when the look is the table's none.
    function wear_junk_look(self, look: Identity): boolean
    -- CrosOver's MegaMan (`sub_80BDBC8`): take the user's identity when the
    -- user is MegaMan (`megaman`) or one of his forms (true), else
    -- MegaMan's own: its sprite (a ground shadow, animation 0 loaded by the
    -- next sprite update) and the side's form's palette.
    function wear_navi_image(self, user: Object, megaman: NaviDef): boolean
    -- MstrCros's Crosses (`sub_80BE7BC`), Darkness's Dark MegaMan
    -- (`sub_80BF710`): take the identity of `navi` (MegaMan) in `form` (the
    -- form's; in his base form his own): its sprite (a ground shadow,
    -- animation 0 loaded by the next sprite update) and palette.
    function wear_form_image(self, navi: NaviDef, form: FormDef): ()
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
-- level, the hand-shrink bug's turn, the start weapons and the shoes; and
-- what the patch cards change, which they write at the round's start).
declare class Navi
    -- Fighting in the sun.
    sun: boolean
    -- The form and the navi, as their definitions.
    form: FormDef
    navi: NaviDef
    navi_variant: number
    -- The base form's element.
    element: number
    -- The buster's attack level (writable: `SetBattleNaviStatsByte`).
    attack: number
    rapid: number
    charge: number
    mood: number
    -- Writable: the Beast Out turns left.
    beast_out_counter: number
    -- (`form` is writable too: a game's form change puts the navi in it.)
    -- The form the navi started the battle in.
    starting_form: FormDef
    -- The navi's game: 0 Gregar, 1 Falzar.
    version: number
    -- The Regular chip's MB at most.
    regular_memory: number
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
    custom_level: number
    -- NaviCust bug: the hand shrinks from this turn on (0 none).
    hand_shrink_turn: number
    -- The weapons rounds start with (nil: none).
    charge_shot_weapon: Weapon?
    back_special_weapon: Weapon?
    float_shoes: boolean
    air_shoes: boolean
    undershirt: boolean
    -- BN5's Hub Style (NaviStats +0x4C), which its patch card 111 sets: 1 Team ProtoMan's, 2 Team Colonel's; 0 none.
    hub_style: number
    -- HP when the round starts, and its maximum.
    hp: number
    max_hp: number
    -- The folder's Mega and Giga chip limits.
    mega_level: number
    giga_level: number
    super_armor: boolean
    status_guard: boolean
    -- The B button's weapon (nil: none).
    buster_weapon: Weapon?
    -- The barrier the navi enters with (nil: none).
    first_barrier: Barrier?
    gauge: "normal" | "fast" | "slow"
    -- The supports; none with the NaviCust's support bug (setting one
    -- then leaves it so).
    rush: boolean
    beat: boolean
    tango: boolean
    -- Writable: a game's NaviCust rules bring the support bug (none, and
    -- none can be set) or clear it.
    support_bug: boolean
    -- NaviCust bugs: steps go astray (1), the panel trail's level, the
    -- status a hit gives, the custom screen's damage, the emotion swings,
    -- the battle start's.
    step_bug: number
    panel_trail_level: number
    hit_status: number
    custom_damage: number
    emotion_bug: number
    battle_start_bug: number
    -- NaviCust ChpShufl and NumbrOpn.
    chip_shuffle: boolean
    number_open: boolean
    -- What the NaviCust does to chip drops (1 its collector bug) and random
    -- encounters (1 its encounter bug): no netbattle reads them. Writable:
    -- a game's NaviCust rules set them.
    chip_drops: number
    encounters: number
    -- `sub_800FE52`: how many kinds of NaviCust bug the navi has.
    bug_kinds: number
    -- BN5's NaviStats +0x32: the turns a soul lasts longer (SoulT+1), which the custom screen adds to a soul's three.
    soul_turn_bonus: number
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
-- with its `tether`: when its side leaves the forms whose afterimages stay
-- (the form trait `afterimages_stay`: BN6's Beast forms), or when its owner
-- stops attacking.
export type AfterimageSpec = {
    sprite: SpriteAsset?,
    anim: number?,
    flip: number?,
    lifetime: number,
    color_shader: number?,
    palette: number?,
    shadow: Shadow?,
    steady: boolean?,
    tether: ("form" | "attack")?,
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
    -- Battle flag 4 alone, with none of a dimming's machinery (BN5's chaos
    -- failure).
    set_dimmed: (on: boolean) -> (),
    -- BattleState+0x40, `sub_800A704`: the fight's ticks so far (capped).
    battle_time: () -> number,
    -- Player `side`'s tactics (BN5's computer-navi data, which a computer
    -- navi on the other side plays): how many entries they count; the entry
    -- in place `i` (from 1; past the count, an empty place): a chip, a
    -- pattern's number (from 1), false (nothing: chip 0, a block no save
    -- filled) or nil (an empty place); and
    -- pattern `i` (from 1): its place from the target (`dx` columns toward
    -- the navi's enemies, `dy` rows) and its chips.
    tactic_count: (side: number) -> number,
    -- Slot `i` (1 to 4) of a side's list of alive actors, filled or not.
    alive_actor_slot: (side: number, i: number) -> Object?,
    tactic: (side: number, i: number) -> (Chip | number | false)?,
    tactic_pattern: (side: number, i: number) -> (number, number, { Chip }),
    -- The first entry and the entry in place `i` (from 1) change places
    -- (0x0802C0DC); the first entry goes last of the count, the rest move
    -- up (0x0802BF1C).
    swap_tactics: (side: number, i: number) -> (),
    turn_tactics: (side: number) -> (),
    -- BN5's 0x08006AAE: the navi `identity` (actor type navi, with a
    -- `body`) comes onto (x, y) for `side`, brought by `summoner`, driven by
    -- `system` (its `controller`, with the navi's state its `navi_state`;
    -- of the summoner's side's ruleset): on the side's list of alive
    -- actors, not counted. Nil when a pool or the side's list is full.
    spawn_navi: (identity: Identity, x: number, y: number, side: number, summoner: Object?, system: System) -> Object?,
    paused: () -> boolean,
    -- `battle_isBattleOver`: a side has no navi left, or time is up.
    over: () -> boolean,
    -- The same as the routines that read its Z flag see it: true only once
    -- time is up (a KO reads false).
    time_up: () -> boolean,
    -- `sub_800ED90` and its record's flags: the chip `user` would use next
    -- deals damage (a player's at its hand's cursor; another object's own).
    next_chip_damages: (user: Object) -> boolean,
    link: () -> boolean,
    -- A battle against a ranked boss (battle effect 1).
    boss_rank: () -> boolean,
    -- A battle whose dark chips fizzle (battle effect 0x100000: BN5's dark chip rule).
    no_dark_chips: () -> boolean,
    -- A battle that holds BN5's light/dark value at 500 (battle effect 0x20000, 0x08010EDC).
    light_dark_held: () -> boolean,
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
    -- Battle flag 2: the custom gauge is full.
    gauge_full: () -> boolean,
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
    -- `battle_setFlags(0x20)` / `battle_clearFlags(0x20)`: the cameras
    -- shake even while the battle is paused (BN5's TomahawkSoul's change).
    set_shake_through_pause: (on: boolean) -> (),
    -- `sub_80302B6`: the cameras' second shake.
    shake_camera_secondary: (magnitude: number, ticks: number) -> (),
    -- `sub_80EA438`: a burst around the navi's panel (effect #0x90, which
    -- runs while paused).
    burst: (navi: Object) -> Object?,
    -- `sub_801DA48` (`shown`) or `sub_801DACC` with the parts' HUD draw
    -- tasks: every console shows or hides them (drawn only).
    show_hud: (parts: { HudPart }, shown: boolean) -> (),
    navi: (side: number) -> Navi,
    emotion: (side: number) -> Emotion,
    -- Unless the navi's emotion is held.
    set_mood: (side: number, mood: number) -> (),
    -- The mood rises by `n` to 254 at most; 0 and 0xFF stay (BN5's
    -- 0x08012802: its recovery chips).
    gain_mood: (side: number, n: number) -> (),
    -- The mood falls by `n` to 1 at least; 0 stays (`sub_8015C12`, BN5's
    -- 0x08012820: its navi chips leaving).
    lose_mood: (side: number, n: number) -> (),
    -- Whether the side's console starts its emotion window glitching (the
    -- save's NaviCust bug flag): a system decides it in `round_setup` (BN6's
    -- patch cards do).
    set_emotion_window_glitch: (side: number, on: boolean) -> (),
    -- A side's installed patch cards in their list's order, each switched
    -- on or not (the player's setup's; a game's rules apply them).
    patch_cards: (side: number) -> { { card: PatchCard, enabled: boolean } },
    -- A side's NaviCust (the player's setup's; a game's rules compile it into
    -- the navi's stats in `round_setup`): its board's expansions and its
    -- programs in its list's order, each with its color, its center on the
    -- 7x7 grid, its quarter turns clockwise and whether it is compressed;
    -- nil when the setup gives none (the stats are already the NaviCust's).
    navicust: (side: number) -> NaviCust?,
    -- The folder a tool asks the rules to check (a system's `folder_check`):
    -- whose side, its chips in order, its Regular and tag chips (places in
    -- `chips`), and whether it is all of a folder (else the chips so far:
    -- the rules about a whole folder wait); nil when none is.
    checked_folder: () -> CheckedFolder?,
    -- A rule the checked folder breaks: the rule's name and what to say.
    folder_problem: (rule: string, text: string) -> (),
    -- A side's bug frags, and spending some (its callers check there are
    -- enough: the count wraps below 0).
    bug_frags: (side: number) -> number,
    spend_bug_frags: (side: number, count: number) -> (),
    -- The level of the navi code a side's save received (0 to 14), nil
    -- when none was (BN6's event flag 0x163).
    navi_level: (side: number) -> number?,
    -- `sub_802E4B8`: the side's SELECT or Cross special in progress.
    side_special: (side: number) -> SideSpecial,
    -- A system's takeover of the side's navi starts for `ticks` (counted
    -- down in the navi's stage B, `sub_802E1D8`), or ends; while it runs,
    -- idle asks the side's systems' `takeover`. Its ticks left.
    take_over: (side: number, ticks: number) -> (),
    end_takeover: (side: number) -> (),
    takeover_ticks: (side: number) -> number,
    player: (side: number) -> Object?,
    -- A side's combatants still in, in slot order.
    alive_actors: (side: number) -> { Object },
    -- `sub_802EFEE`: the actor `side` tracks in the battle flag 0x40 mode
    -- (`per_player_gauges`), if any.
    tracked: (side: number) -> Object?,
    -- The objects of a kind in the update list, in update order, whatever
    -- their lifecycle state (the game's walks of the list).
    objects_of: (kind: Kind) -> { Object },
    -- `GetRNG2` (one draw), `GetPositiveSignedRNG2` (bit 31 cleared).
    rng: () -> number,
    rng_positive: () -> number,
    -- `GetPositiveSignedRNG1` on a side's console: a draw of that console's
    -- own RNG (RNG1), which the consoles never share; what a console decides
    -- by it differs between them, so the simulation follows the local
    -- side's console (`local_side`) and draws the other's alike.
    console_rng_positive: (side: number) -> number,
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
    -- `sub_801E658` (BugFix): the save's emotion window glitch is gone from
    -- every console's emotion window, which then flickers (and draws its
    -- console's RNG1) for NaviCust bugs only.
    clear_emotion_window_glitch: () -> (),
    fill_custom_gauge: () -> (),
    -- `sub_801DFD0` (BN5's CusVolt): the custom gauge drops by `n`, to 0 at least.
    drain_custom_gauge: (n: number) -> (),
    -- `sub_801DF8C`: the custom gauge fills `rate` a tick (full at 0x4000).
    set_gauge_rate: (rate: number) -> (),
    -- A side's slow and fast gauge timers (`sub_802E070`+0x3C, +0x3A).
    set_gauge_speed_ticks: (side: number, slow: number, fast: number) -> (),
    -- `sub_8010B78`: the damage a side's custom gauge gives (its own gauge
    -- in the battle flag 0x40 mode, else the shared one).
    gauge_damage: (side: number) -> number,
    -- A side's sword pick (`sub_802E070`+0x12): the swing a variable sword
    -- makes for a navi no buttons drive (BN5's computer navi draws it).
    sword_pick: (side: number) -> number,
    set_sword_pick: (side: number, pick: number) -> (),
    -- Presentation: whether the side's emotion window shows its base form's second set of faces (`mugshot.variant`:
    -- BN5's Hub Style).
    set_face_variant: (side: number, variant: boolean) -> (),
    -- `sub_802E032`: add to a side's own custom gauge, up to full.
    add_side_gauge: (side: number, n: number) -> (),
    -- `sub_802E04E`: take from a side's own custom gauge, down to empty.
    drain_side_gauge: (side: number, n: number) -> (),
    -- `sub_8010488`'s special-source branch: add to a bonus a side stores
    -- for its special chip (`index` 0: the Atk+ bonus a damaging chip
    -- spends; 1: the Navi+ bonus a navi chip spends).
    add_special_bonus: (side: number, index: number, n: number) -> (),
    bump_side_stat: (side: number, index: number, n: number) -> (),
    -- `sub_800AB2E`: set a side's statistics counter.
    set_side_stat: (side: number, index: number, n: number) -> (),
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
    -- BN5's DarkPlus: the Atk+ bonus of a hand's chip at i becomes n.
    set_hand_attack_bonus: (side: number, i: number, n: number) -> (),
    -- A side's hand has a chip at i and it does damage (its record's
    -- "has_damage" flag).
    hand_chip_damages: (side: number, i: number) -> boolean,
    -- BN5's Jealousy (0x080E4596): the chips left in a side's hand, from
    -- its cursor to the first empty slot.
    hand_left: (side: number) -> number,
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
    -- A side's registry slot `i` (1 to 3), filled or not.
    object_slot: (side: number, i: number) -> Object?,
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
-- from where the HUD projects its position (0 if nil), the digits centered
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
    -- The Gregar and Falzar chips' controllers' first action in place of
    -- the dim (the Japanese ROMs' 0x080EDD0C and 0x080EDED0): the field
    -- darkens to black, then the next action.
    fade_to_black: (controller: Object) -> (),
    -- `sub_800BCF6`, their last in place of the undim: back from black (to
    -- the dim at once while the other side's dimming runs), then end.
    fade_from_black: (controller: Object) -> (),
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
    -- `sub_80E146C` / `sub_80E14AC`: an actor that isn't a player vanishes
    -- (its barrier visual and its confusion and blindness visuals with it)
    -- and comes back (BN5's Django's coffin).
    hide_actor: (o: Object) -> (),
    show_actor: (o: Object) -> (),
}

-- The last navi chip used (`navi_chip.last`): the chip (whose `navi` hook
-- brings its navi), and the element and the damage word, bonus included,
-- its navi came with.
export type LastNaviChip = { chip: Chip, element: number, damage: number }

declare navi_chip: {
    -- The navi is done: its controller moves on (an object of a content
    -- kind that brought the navi itself: its kind's `navi_left`).
    navi_left: (controller: Object) -> (),
    -- The last navi chip used, of either side, this battle (`byte_203C960`,
    -- BN5's 0x0203C430: the navi chip controller keeps the chips of the navi
    -- chips' block, as their navis come), or nil. BN5's DethPhnx brings its
    -- navi again.
    last: () -> LastNaviChip?,
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
    -- kind's: one of chips/rockcube/rock's `variants`).
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
    -- that, and any one side's hit but type 0x1000 pushes it a panel away;
    -- BN5's, 0x08017A78, by the hitters' flips: its game's rules'
    -- `push_reading`), "ignored" (`sub_801AD6A`: never pushed). An
    -- obstacle's push reads its hits as its game's `push_reading` says.
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
    -- BN5's Poltergeist's test (0x080E8CA0): `o`'s identity is `throwable`.
    throwable: (o: Object) -> boolean,
    -- `sub_800F6AC`: ask the field object `o` to be picked up by `side` and
    -- thrown at panel (x, y) after shaking `shake` ticks, hitting with the
    -- damage word `damage`.
    throw: (o: Object, side: number, x: number, y: number, shake: number, damage: number) -> (),
    -- BN5's ColonelSoul army (docs/design/bn5-map.md §15.11): arm side
    -- `side` (BattleState+0x5C bit 0x10 or 0x20, 0x080CAC1E) with its
    -- soldiers' damage words (0x080CABF8): the sword soldier's and the gun
    -- soldier's. While it is armed, an obstacle of a game whose rules have
    -- `effects.obstacle_soldiers`, standing on a panel of the side's enemy
    -- with one of the enemy's bodies on its row, turns into the side's
    -- soldier (the role `kinds.obstacle_soldier`).
    arm_soldiers: (side: number, sword: number, gun: number) -> (),
    -- 0x080CAC30: disarm side `side`; its words stay.
    disarm_soldiers: (side: number) -> (),
    -- Whether side `side` is armed, and its words (0x080CAC06,
    -- 0x080CAC12): what its soldiers strike with as they read them.
    soldiers: (side: number) -> (boolean, number, number),
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

-- Assets, named in full while content loads (`asset.sprite("bn6:bomb")`: the
-- pack's game, then its name, a game's compat/assets.toml's): one value per asset. (`SpriteAsset`, since
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

-- An object kind: one behavior; what differs between uses goes in its
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
    -- An object of it brought a navi chip's navi itself (passing itself as
    -- the navi's controller): the navi is done (`navi_chip.navi_left`).
    navi_left: ((me: Object) -> ())?,
}
export type Kind = KindSpec

-- One self-contained piece of a game's rules (docs/design/rules-in-luau.md
-- §2.2): its state of each side and its player's setup (named fields, as a
-- kind's state), and the hooks the framework calls, each for one side
-- (§4.1). A system sees only its own state, of the side it is called for.
-- A system's custom-screen button (docs/design/rules-in-luau.md §4.4).
export type ButtonSpec = {
    -- Its first slot, and how many it takes (1 or 2).
    slot: number,
    cells: number?,
    -- Its uses on a screen (the machinery it starts counts them).
    uses: number?,
    -- Its first cell's right neighbor and its last cell's left one, over
    -- the layout's.
    right: number?,
    left: number?,
    -- As the screen opens: whether it is on the screen. The first button
    -- to claim a slot keeps it.
    shown: (side: number) -> boolean,
    -- At the open and after each pick, unless it is picked or used up.
    -- None: selectable at the open, then left as it is.
    state: ((side: number) -> ("selectable" | "unavailable"))?,
    -- A on it.
    pressed: (side: number) -> (),
    -- B took its pick back.
    taken_back: ((side: number) -> ())?,
}
-- A system's custom-screen window (§4.4): its update, each tick it is up
-- (`custom.window_tick` counts them from 1): whether it stays up.
export type WindowSpec = {
    update: (side: number) -> boolean,
}
export type SystemSpec = {
    id: string,
    state: StateSchema?,
    setup: StateSchema?,
    -- What a player's setup that says nothing of a field holds (else zero): a
    -- value of each field given (an enum's by name).
    setup_defaults: { [string]: number | boolean | string }?,
    -- Its own actions, which reach its state (`system.state()`) for the
    -- side of the navi running them.
    actions: { Action }?,
    -- Its custom screen's extras (docs/design/rules-in-luau.md §4.4): the
    -- side's screen asks them; the first system that answers decides.
    -- Its custom-screen buttons, by name (what the frontend draws them by):
    -- where each sits and its functions, which run as the system's.
    buttons: { [string]: ButtonSpec }?,
    -- Its custom-screen windows, by name (`custom.open_window`).
    windows: { [string]: WindowSpec }?,
    -- Its fields on its game's definitions (docs/design/rules-in-luau.md
    -- §7.5), by registry (chip, form, navi): each a type name, a list of
    -- variants, or a table of such fields. The engine checks a definition's
    -- and reads none of them.
    extends: { [string]: { [string]: any } }?,
    custom: {
        -- As the screen opens: how many chips it deals (`sub_802A40C`).
        -- None answering: the custom level (8 at most), NumbrOpn's 10, and
        -- the hand-shrink bug.
        hand_size: ((side: number) -> number?)?,
        -- The screen deals, before it opens: the folder as the last screen
        -- left it (a used chip's place still empty: `custom.folder`), with
        -- the framework's hand size (`custom.hand_size`), on the side's
        -- console (`battle.console_rng_positive`).
        deal: ((side: number) -> ())?,
        -- The screen opens, before its hand size and layout.
        open: ((side: number) -> ())?,
        -- OK built the hand.
        confirmed: ((side: number) -> ())?,
        -- A chip of the hand picked, or its pick taken back.
        chip_picked: ((side: number, chip: Chip) -> ())?,
        chip_taken_back: ((side: number, chip: Chip) -> ())?,
        -- Choosing chips, a tick with keys (`custom.pressed`,
        -- `custom.repeated`), before the screen's own: whether the system
        -- took them. The first system that answers true takes them.
        keys: ((side: number) -> boolean)?,
        -- B with nothing picked: whether the system took something back
        -- (none: B is refused).
        take_back: ((side: number) -> boolean)?,
    }?,
    hooks: {
        -- Once per side as the round is set up, before anything reads the
        -- side's navi stats (`battle.navi(side)`), which it may change.
        round_setup: ((side: number) -> ())?,
        -- Once per side, after the navis spawn.
        round_start: ((side: number) -> ())?,
        -- The turn-start sequencer's check of a side that isn't changing
        -- form or asks for a navi switch (`sub_801486C`): a form whose time
        -- ran out asks to revert.
        turn_check: ((side: number) -> ())?,
        -- The turn starts, after the sequencer (`sub_800840C`): a turn in a
        -- form is spent.
        turn_started: ((side: number) -> ())?,
        -- A custom screen was asked for mid-fight (`sub_8008452`).
        custom_requested: ((side: number) -> ())?,
        -- Both results are in and the fight resumes (`sub_8009338`).
        custom_closed: ((side: number) -> ())?,
        -- A tool asks whether a folder keeps the side's game's folder rules
        -- (not the simulation): the folder is `battle.checked_folder()`, and
        -- each rule it breaks is `battle.folder_problem(rule, text)`.
        folder_check: ((side: number) -> ())?,
        -- Each tick of the fight, in the navi's intake after the standing
        -- effects (`sub_801AC6C`; BN5's light and dark MegaMan clears a holy
        -- panel there).
        navi_intake: ((side: number, navi: Object) -> ())?,
        -- A chip's use is prepared (the end of `sub_80127C0`): nil lets it
        -- be used, a chip is what the navi uses instead. The first system
        -- that answers decides.
        chip_check: ((side: number, navi: Object, chip: Chip?) -> Chip?)?,
        -- Earlier in the preparation, before the hand bonus's panel is
        -- spent (BN5's 0x08010030: a dark chip's use and the chip's cost):
        -- as `chip_check`; a chip in the use's place skips the rest,
        -- `chip_check` too.
        chip_cost: ((side: number, navi: Object, chip: Chip?) -> Chip?)?,
        -- A chip's use is prepared, before its record is loaded
        -- (`sub_8010D58`): a chip is what the navi uses in its place, with
        -- its own record, damage and bonus; nil, the chip. The first system
        -- that answers decides.
        chip_substitute: ((side: number, navi: Object, chip: Chip) -> Chip?)?,
        -- A chip's use was prepared (`sub_80127C0`: a use, the rush's chain,
        -- the counter cut-in's), its substitute taken: `chip` the chip it
        -- uses (the zeroed chip for the empty hand).
        chip_prepared: ((side: number, navi: Object, chip: Chip) -> ())?,
        -- A chip's use started (`sub_800FB54`, its action set): `chip` the
        -- chip it reads (the zeroed chip for the empty hand), `weapon` the
        -- form's weapon run instead of it (a charged use), else nil.
        chip_used: ((side: number, navi: Object, chip: Chip, weapon: Weapon?) -> ())?,
        -- Each tick of the idle action of a navi whose form is `controlled`
        -- (BN6's Beast Over, `sub_802D322`), in place of the player's
        -- decisions: what it did ("nothing", "chip": a chip's use started,
        -- "buster": the buster is to fire, "moved": a step started), which
        -- the framework carries out as idle does. The first system that
        -- answers decides.
        controller: ((side: number, navi: Object) -> ("nothing" | "chip" | "buster" | "moved")?)?,
        -- The framework reverts the navi to its base form, its form not yet
        -- changed (`sub_80158CC`).
        form_reverted: ((side: number, navi: Object) -> ())?,
        -- Idle finds the navi's takeover request (`sub_802E4E4`; BN6's
        -- DarkInvs asks for its Cross special): a system starts it
        -- (`battle.take_over`).
        takeover_requested: ((side: number, navi: Object) -> ())?,
        -- The side's navi landed a counter on `victim` (`sub_801A200`).
        countered: ((side: number, victim: Object) -> ())?,
        -- Each unpaused tick, after the navi's input (`sub_8013DA0`'s
        -- place), for a navi a system asked it for (its `ticked`).
        navi_tick: ((side: number, navi: Object) -> ())?,
        -- The mood the side's navi starts the round with (`sub_8015C2C`'s
        -- 0x80, where `sub_8013892` sets it; BN5's by the light/dark value,
        -- 0x0801283A). The first system that answers decides.
        starting_mood: ((side: number) -> number?)?,
        -- Each tick, the sprite palette of the side's navi of a player's
        -- kind (presentation; `sub_801002C`, BN5's 0x0800DD94). The first
        -- system that answers decides; none, the framework's (BN6's).
        navi_palette: ((side: number, navi: Object) -> number?)?,
        -- Before the navi takes its hit's NaviCust bug (`sub_80139F6`, BN5's 0x0801103E): a system may change the bug
        -- (`navi.collision.inflicted_bugs`), or answer true: the bug and the weapons' reload are skipped. The first
        -- system that answers true decides.
        navi_bug: ((side: number, navi: Object) -> boolean?)?,
        -- Each tick of the idle action of a navi whose side's takeover runs
        -- (`sub_802D4C6`), in place of the player's decisions: what it did,
        -- as `controller` answers, or "own_chip" (an attack of its own
        -- started: the attack's chip). The system ends it
        -- (`battle.end_takeover`). The first system that answers decides.
        takeover: ((side: number, navi: Object) -> ("nothing" | "chip" | "buster" | "moved" | "own_chip")?)?,
    }?,
}
export type System = SystemSpec

-- A player's rules: a list of systems, which the framework calls in this
-- order. A game's own is its `stock` ruleset (one per game). A ruleset made
-- from another (`base`, a mix) has its base's systems less `remove`, with
-- `add` after them. Its game, whose data (roles, rule sections) the side
-- reads, is the root `game` names, else its base's, else its own root
-- (docs/design/rules-in-luau.md §2.3).
export type RulesetSpec = {
    id: string,
    stock: boolean?,
    systems: { System }?,
    base: Ruleset?,
    add: { System }?,
    remove: { System }?,
    game: string?,
    -- Its own rule sections (`define.rules(...)`, ids of its folder): a
    -- mix's, over its game's for the sides that play by it; only the
    -- sections about a side (custom-screen, berserk, navicust, status,
    -- lockon, cross-special).
    sections: { any }?,
}
export type Ruleset = RulesetSpec

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

-- (A definition holds no display text: names, descriptions and messages
-- are the content root's locales/<lang>.toml, by the definition's key. The
-- content's own language's words (en) time the chatbox: a description's
-- lines, a message's characters per line.)
export type WeaponSpec = {
    id: string,
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
-- user has lost (at most `cap`: BN6's 500 when none); the last two digits
-- of the user's HP; half the opponent's max HP; a link navi's own chip's,
-- by its level; by a count of the user's side (BN5's DS navi chips, Roll
-- SP's, Django SP's: `by_count[n]`, the last for more). BN5's SP and count
-- formulas give `per_player_gauges` in the battle flag 0x40 mode.
export type DamageFormula =
    { formula: "opponent_hp" }
    | { formula: "sp_navi", slot: string, by_time: { number }, per_player_gauges: number? }
    | { formula: "gauge" }
    | { formula: "gauge_level", base: number }
    | { formula: "hp_lost", cap: number? }
    | { formula: "hp_last_digits" }
    | { formula: "half_opponent_max_hp" }
    | { formula: "navi_level", base: number, per_level: number }
    | { formula: "count", of: DamageCount, by_count: { number }, per_player_gauges: number? }

-- What a count formula counts: a side statistic, the side's panels of a
-- type, the turns before this one.
export type DamageCount = { side_stat: number } | { own_panels: PanelType } | "turns_before"

export type ChipSpec = {
    id: string,
    -- The palette of its picture on the custom screen, 16 BGR555 colors,
    -- for a chip whose palette no ROM holds (the pack's picture is black
    -- without it). Presentation only.
    art_palette: { number }?,
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
    -- `navi` flag), "not_navi_slot" (AntiNavi lets it through and it isn't
    -- kept as the last navi chip, though it has the `navi` flag: past the
    -- navi chips' block), "heals" (AntiRecv springs on it), "user_stays" (a
    -- navi chip whose user isn't warped off the field), "navi_returns_user"
    -- (a navi chip whose navi brings the user back itself), "goes_with_any"
    -- (it goes with any selection on the custom screen, and leaves the
    -- selection's rules alone: BN6's BeastOut chip), "hides_damage_as_a"
    -- (the custom screen shows its damage as "???" on a copy of code A).
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

-- A chip family and an element, by name.
export type ChipFamily =
    "fire"
    | "aqua"
    | "elec"
    | "wood"
    | "plus"
    | "sword"
    | "cursor"
    | "summon"
    | "wind"
    | "break"
    | "null"
    | "program_advance"
    | "special"
    | "recovery"
    | "invisible"
export type ElementName = "null" | "fire" | "aqua" | "elec" | "wood"
-- The chips a navi's or a form's rule is about: an element's, or a
-- family's.
export type ChipMatch = { element: ElementName?, family: ChipFamily? }

-- A form's faces in the emotion window by its navi's emotion: an emotion
-- without one shows `normal`.
export type FaceSet = {
    normal: Mugshot,
    angry: Mugshot?,
    tired: Mugshot?,
    full_synchro: Mugshot?,
    worn_out: Mugshot?,
    -- BN5's mood under 65.
    worried: Mugshot?,
}
export type MugshotFaces = {
    normal: Mugshot,
    angry: Mugshot?,
    tired: Mugshot?,
    full_synchro: Mugshot?,
    worn_out: Mugshot?,
    worried: Mugshot?,
    -- A second set, shown while the navi's Chaos Unison charge is armed (BN5's soul's chaos look), or in the base form
    -- when the side's rules ask (`battle.set_face_variant`: BN5's Hub Style).
    variant: FaceSet?,
}

-- A navi (§3.2): MegaMan or a link navi. (`NaviDef`, since `Navi` is a
-- side's stats.)
export type NaviDef = {
    id: string,
    -- The battle sprite (a navi that changes form draws its form's).
    sprite: SpriteAsset,
    element: ElementName,
    weakness: { string }?,
    buster_bonus: number,
    -- Ticks of lag after a move, by navi variant.
    move_lag: { number },
    banners: { win: Banner, lose: Banner },
    -- A link navi's face in the emotion window (shown in its second palette
    -- in Full Synchro); MegaMan's face is his form's.
    mugshot: Mugshot?,
    -- Extra height of its image as it merges with MegaMan in a Cross.
    merge_height: number?,
    -- The no-running message (L on the custom screen in a netbattle).
    run_message: RunMessage?,
    -- A link navi's own chip, offered once a round.
    own_chip: { chip: Chip, code: string }?,
    -- Its damage bonus on its family's damaging chips, by its level.
    chip_bonus: { family: ChipFamily, dimming_chips: boolean?, by_level: { number } }?,
    -- MegaMan's bonus on a family's damaging chips used standing on a type
    -- of panel, in a form with no `chip_bonus` (BN5's: Aqua chips on sea);
    -- the use turns the panel Normal.
    panel_bonus: { panel: PanelType, family: ChipFamily, damage: number }?,
    -- The chips it charges with A: its family's damaging chips, from a
    -- navi level on (`byte_8021369`).
    charged_chips: { family: ChipFamily, from_level: number }?,
    -- The chips a charge doubles (`sub_8012AFA`).
    charge_doubles: ChipMatch?,
    -- Its A charge builds up the next Fire chip's damage, up to a limit by
    -- its level (`byte_802136D`).
    fire_charge: { number }?,
    -- "status_immune": a hit's status doesn't take.
    traits: { "status_immune" }?,
    -- The forms it changes into (MegaMan's), by game: the Crosses by their
    -- number on the custom screen, Beast Out and Beast Over. Where the
    -- original asks whether a navi is MegaMan, the ruleset asks whether it
    -- has forms. BN5's MegaMan has souls instead (forms with a `soul`,
    -- which the custom screen's soul button offers).
    forms: { gregar: FormSet?, falzar: FormSet?, souls: { FormDef }? }?,
    identity: Identity,
    -- What a navi switch brings it with: its weapons, the rest of its
    -- fresh stats, and its HP after the change, by side.
    weapons: FormWeapons?,
    fresh: {
        hp: number,
        super_armor: boolean?,
        float_shoes: boolean?,
        air_shoes: boolean?,
        undershirt: boolean?,
        -- The barrier it enters with (lib/barriers; nil: none).
        first_barrier: Barrier?,
        mega_level: number,
        giga_level: number,
        back_special_damage: number?,
    }?,
    cross_hp: { number }?,
    -- What the save's reload gives it by its link navi level (BN6's
    -- `reloadCurNaviBaseStats`): its base HP by the story's progress, and
    -- what each level adds over its fresh stats. Tools fill a side's stats
    -- from it; no battle reads it.
    levels: NaviLevels?,
}
-- A navi's levels (docs/engine/link-navis.md).
export type NaviLevels = {
    -- The base and maximum HP, by the story's progress (0 to 6).
    base_hp: { number },
    -- Each level's gains, from level 0.
    by_level: { NaviLevelGain },
}
-- What a level adds: HP to the maximum, the buster's levels (to 4 at
-- most), the custom level (to 8) and the Mega level (to 10); the abilities
-- it gives; the B+Back special it sets.
export type NaviLevelGain = {
    hp: number,
    attack: number?,
    rapid: number?,
    charge: number?,
    custom_level: number?,
    mega_level: number?,
    super_armor: boolean?,
    float_shoes: boolean?,
    air_shoes: boolean?,
    back_special: Weapon?,
}
-- A navi's no-running message. Its words are locales/<lang>.toml's; the
-- content's own language's (en) time it: the characters in each line, and
-- which of them move the speaker's mouth.
export type RunMessage = {
    -- Who says it: the portrait beside the box (presentation).
    portrait: SpriteAsset?,
}
export type FormSet = { crosses: { FormDef }, beast_out: FormDef?, beast_over: FormDef? }
export type FormWeapons = {
    buster: Weapon?,
    charge_shot: Weapon?,
    back_special: Weapon?,
    a_charge: Weapon?,
    alt_a_charge: Weapon?,
    mode9_a: Weapon?,
    -- A BN5 soul's Chaos Unison weapon: what an armed full B charge
    -- released in the chaos cycle's window fires.
    chaos: Weapon?,
}
-- What a form's status reset gives the navi (`sub_8014536`).
export type FormEffect =
    "clear_statuses"
    | "super_armor"
    | "air_shoes"
    | "float_shoes"
    | "floating_body"
    | "untouchable"
    | "target_marker"
    | "invulnerable"
    | "berserk"
-- What the ruleset asks of particular forms:
--   status_immune: a hit's status doesn't take (TomahawkCross);
--   erases: its damaging Null chips delete what has a 4 in its HP
--     (EraseCross);
--   special_holds_buster: its held buster doesn't fire while its B+Back
--     special is asked for (TenguCross and DustCross in Beast Out);
--   stands_on_metal: a metal panel doesn't slide the navi (BN5's soul 5);
--   controlled: the side's systems' `controller` decides its idle (BN6's
--     Beast Over: the berserk), the player's buttons don't reach it, and a
--     full gauge opens the custom screen;
--   shows_target_marker: the navi's target marker shows (BN6's Beast
--     forms);
--   afterimages_stay: an afterimage of the navi lasts while it stays in
--     such a form, not until its attack ends (BN6's Beast forms);
--   alt_charge_time: with a Null chip next, its A charge's time is the
--     alternative A-charge weapon's (BN6's Beast forms);
--   doubles_null: its damaging Null chips deal double, outside battle mode
--     1 (Beast Over);
--   mood_palette: its palette, and its buster arm's, follow the mood
--     (Gregar and Falzar Beast).
export type FormTrait =
    "status_immune"
    | "erases"
    | "special_holds_buster"
    | "stands_on_metal"
    | "controlled"
    | "shows_target_marker"
    | "afterimages_stay"
    | "alt_charge_time"
    | "doubles_null"
    | "mood_palette"
-- One of MegaMan's forms (§3.2).
export type FormDef = {
    id: string,
    sprite: SpriteAsset,
    element: ElementName,
    weakness: { string }?,
    buster_bonus: number,
    -- Its game's base form: what the game's navis are in before they change
    -- form (one a game).
    base: boolean?,
    -- The action that changes a navi into it (a game's form change: BN6's
    -- forms', rules/forms), which the pause handler runs at a turn's start.
    -- Every form but the base form names one.
    change: Action?,
    -- BN6's forms system's extension (rules/forms/system.luau; until S8
    -- moves BN6's declarations to bn6.d.luau): its kind (rules/forms/kind),
    -- whose game's form it is (the Beast's roar), the navi a Cross (and one
    -- in Beast Out) is made with, and the animation a change into a Cross
    -- lets the navi go of.
    kind: ("cross" | "beast" | "cross_beast" | "beast_over")?,
    game: ("gregar" | "falzar")?,
    cross_of: NaviDef?,
    cross_release_anim: number?,
    -- BN6's beast system's: a Cross's form in Beast Out, the Cross special's
    -- buster volley in it (the original stores the form's number where it
    -- meant 6), and whether its charged sword runs inside the Beast Out
    -- rush (SlashCross in Beast Out).
    beast: FormDef?,
    special_volley: number?,
    charged_sword_rush: boolean?,
    -- BN6's cross system's: the custom screen deals one more chip for each
    -- screen spent in it, up to three (ChargeCross), or has the scrap
    -- button (DustCross).
    extra_chips: boolean?,
    scrap_button: boolean?,
    -- What a weakness hit drops it to (none: it stays).
    breaks_to: FormDef?,
    -- MegaMan's palette in a Cross.
    palette: number?,
    -- Its face in the emotion window, or its faces by its navi's emotion.
    mugshot: (Mugshot | MugshotFaces)?,
    weapons: FormWeapons?,
    identity: Identity?,
    -- The damage it adds to a family's damaging chips (`uncharged`: only
    -- on a use that isn't charged, with the A charge not full), and to
    -- damaging Null chips when that doesn't apply (Beast Out's, outside
    -- battle mode 1).
    chip_bonus: { family: ChipFamily, damage: number, dimming_chips: boolean?, uncharged: boolean? }?,
    null_bonus: number?,
    -- The chips it charges with A: a family's damaging chips that aren't
    -- dimming chips (`damaging = false`: any of the family), and with
    -- `element_swords` the chips with the `element_sword` trait; `plain`:
    -- only those that are neither dimming nor dark chips (BN5's souls).
    charged_chips: { { family: ChipFamily, damaging: boolean?, element_swords: boolean?, plain: boolean? } }?,
    -- What a charged chip gains in it.
    charged_bonus: { damage: number?, paralyzes: boolean? }?,
    -- The chips a charge doubles, and those whose use heals a twentieth of
    -- the base HP.
    charge_doubles: ChipMatch?,
    chip_heals: ChipMatch?,
    -- Its A charge builds up the next Fire chip's damage, up to this much.
    fire_charge: number?,
    -- What its status reset gives the navi, and what a NaviCust change
    -- gives back (none: the reset's, without the target marker).
    status_reset: { FormEffect }?,
    navicust_refresh: { FormEffect }?,
    -- The height it floats at, in pixels.
    hover: number?,
    -- Its glow: the navi's color shader by the battle time, an entry a
    -- tick, over and over (Beast Over's). A navi in it takes no sprite
    -- palette and no invulnerable glow.
    glow: { number }?,
    -- The lag at the end of a move in it, in place of MegaMan's 4.
    move_lag: number?,
    -- What primes it (a use of a chip it names, not a dimming chip, with
    -- a sound), and the damaging chips a primed use doubles (not dimming
    -- chips), spending it; while it is primed nothing else doubles a chip.
    -- The status reset spends it (BN5's GyroSoul).
    priming: { by: ChipMatch, sound: Sound, doubles: { ChipMatch } }?,
    -- The damaging chips (not dimming chips) that deal double while it
    -- stands on grass, which the use turns normal (BN5's TomahawkSoul).
    grass_doubles: ChipMatch?,
    -- The ticks a damaging chip (not a dimming chip) used with the panel
    -- ahead not its side's keeps it invulnerable (BN5's KnightSoul).
    front_guard: number?,
    -- The action its charged chips start in place of their own (BN5's
    -- NapalmSoul's).
    charged_action: Action?,
    -- What else the status reset does in it, after its `status_reset`
    -- (BN5's souls' own routines: SearchSoul's reveal of invisible
    -- navis, TomahawkSoul's grass).
    reset: ((navi: Object) -> ())?,
    -- What the navi puts on as it takes the form (`sub_8011268`'s routine by
    -- the form), in place of its identity's parts: BN5's base form's shade
    -- in Hub Style (0x0800EE1C), which `add_parts` puts on. A base form's
    -- also runs as a player's init ends, as its navi's init hook
    -- (`sub_8010DD0`: BN5's MegaMan's record names the same routine).
    wears: ((navi: Object) -> ())?,
    -- What else it puts on and takes off with its start and end hooks,
    -- after its identity's parts (BN5's NumberSoul's layer).
    put_on: ((navi: Object) -> ())?,
    take_off: ((navi: Object) -> ())?,
    traits: { FormTrait }?,
    -- The buster arm the navi raises in it (lib/buster): its animation,
    -- and what it adds to the arm's palette.
    buster_arm: { anim: number, palette_add: number? }?,
    -- A soul (BN5's Soul Unison): its number, the chip
    -- family the custom screen's soul button gives up for it, and the
    -- chaos cycle's row its Chaos Unison charge keeps whatever the chaos
    -- level (MeddySoul's; none: the level's, at most 2).
    soul: { number: number, family: ChipFamily, chaos_cycle: number? }?,
}
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
-- A patch card (BN4's, BN5's and BN6's Modification Cards, 改造カード;
-- docs/engine/patch-cards.md): what every game's card is. Its key (compat's
-- patch-cards.toml gives its number), its capacity cost (BN6's MB, which
-- the installed cards' limit counts) and its effects in the card's order:
-- each a table with its `kind` and whether the card shows it as a `bug`,
-- with the kind's own fields. What an effect does is a game's rules' (BN6's
-- patch-cards system, rules/patch-cards: `PatchCardEffect`). Its name is
-- the locales' (locales/<lang>.toml's `patch-cards`).
export type PatchCardSpec = {
    id: string,
    mb: number,
    effects: { PatchCardEffect },
}
export type PatchCard = PatchCardSpec

-- A NaviCust program (BN4's, BN5's and BN6's Navi Customizer parts;
-- docs/design/navicust.md): what every game's program is. Its key (compat's
-- navicust.toml gives its number), the colors it comes in in its variants'
-- order, whether it is a plus part, and its shape: seven rows of seven cells
-- (`#` a cell it covers, `.` one it doesn't), centered on the middle cell, and
-- compressed (nil: it doesn't compress). What it does is a game's rules'
-- (BN6's: `bug`, `exclusive`, `anywhere`, `effects`, read by rules/navicust). Its name is
-- the locales' (locales/<lang>.toml's `navicust-programs`).
export type NaviCustProgramSpec = {
    id: string,
    colors: { string },
    plus: boolean?,
    shape: { string },
    compressed: { string }?,
    bug: string?,
    exclusive: string?,
    anywhere: boolean?,
    effects: { NaviCustEffect }?,
}
export type NaviCustProgram = NaviCustProgramSpec

-- A program on a side's NaviCust (`battle.navicust`).
export type PlacedProgram = {
    program: NaviCustProgram,
    color: string,
    x: number,
    y: number,
    rotation: number,
    compressed: boolean,
}

export type NaviCust = {
    expansions: number,
    parts: { PlacedProgram },
}

-- A folder being checked (`battle.checked_folder`).
export type CheckedFolder = {
    side: number,
    chips: { { chip: Chip, code: string } },
    regular: number?,
    tags: { number }?,
    complete: boolean,
}

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
    -- BN5's Poltergeist takes it (`throwable = false`: BN5's NameIDs 0xDA,
    -- 0xD3, 0xD2, 0xE5, 0xE4 and 0xE7).
    throwable: boolean?,
    -- What its object wears (its actor record's init hook, `off_8010E0C`):
    -- a body overlay (and a second), an overlay worn only while standing,
    -- or a beast's head (its palette; none: it follows the side's mood).
    parts: Parts?,
    -- Which of its object's hooks touch what the object wears: its death
    -- takes it off, an animation change, a flinch or a drag restarts it.
    -- By default the death hook takes its own parts off and an animation
    -- change restarts them (not an overlay worn only while standing).
    overlay_hooks: { "death" | "refresh" | "flinch" | "drag" }?,
    -- The Full Synchro aura's animation around it (a player's has one).
    aura_anim: number?,
    -- The ice block that fits it (none: small).
    ice: ("small" | "medium" | "large")?,
    -- What it is as a navi no player controls (actor type navi, which a
    -- battle brings: `battle.spawn_navi`), from the original's enemy structs
    -- by NameID and its AI index's post-init hook.
    body: NaviBody?,
}
-- A navi no player controls: its sprite (with a shadow on the ground or
-- drawn with the sprite), the multiplier of its palette by version and its
-- palette once in (none: by version); its HP, element and what its body
-- deals; its body's collision (what it is, what it reacts to, its hit
-- modifier); the status flags it starts with; and whether its HP number
-- never shows.
export type NaviBody = {
    sprite: SpriteAsset,
    shadow: boolean?,
    palette_scale: number,
    palette: number?,
    hp: number,
    element: ElementName,
    body_damage: number,
    collision: { body: CollisionType, target: CollisionType, hit_modifier: number },
    flags: { "superarmor" | "airshoe" | "floatshoe" | "affected_by_ice" | "untouchable" }?,
    hides_hp: boolean?,
}
-- A body overlay (actor object #0x56): a second sprite over a navi's body,
-- drawn in front of it but in the animations of `behind` (ranges `{ first,
-- last }` of the wearer's animations; `anims`: how many the original's
-- table covers), where it sits one pixel further back. `own_palette`: it
-- keeps its palette 0 rather than its wearer's. `anim_offset`: added to
-- the wearer's animation.
export type BodyPart = {
    sprite: SpriteAsset,
    anims: number,
    behind: { { number } }?,
    own_palette: boolean?,
    anim_offset: number?,
}
export type Parts = {
    body: BodyPart?,
    second: BodyPart?,
    idle: { sprite: SpriteAsset }?,
    beast_head: { sprite: SpriteAsset, palette: number? }?,
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

-- What the ruleset needs from content by role (rules/roles.luau, once per
-- game): each role names the definition that fills it. `id` names the game
-- whose roles they are (`bn6:roles`).
export type RolesSpec = {
    id: string,
    actions: {
        anti_damage_counter: Action?,
        anti_sword_counter: Action?,
        body_guard_counter: Action?,
        forced_charged_shot: Action?,
        stun_strike: Action?,
        switch_protect: Action?,
        turn: Action?,
        switch_knockout: Action?,
        volley: Action?,
        dust_beast_scatter: Action?,
        charge_tackle: Action?,
        wrapper: Action?,
        chaos_failure: Action?,
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
        burst: Effect?,
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
        burst: Sound?,
        refused: Sound?,
        custom_open: Sound?,
        custom_cursor: Sound?,
        custom_hide: Sound?,
        custom_pick: Sound?,
        custom_ok: Sound?,
        custom_back: Sound?,
        custom_cross_open: Sound?,
        custom_cross_close: Sound?,
        custom_cross_chosen: Sound?,
        custom_run_message: Sound?,
        custom_description: Sound?,
        custom_description_close: Sound?,
        custom_beast_out_falzar: Sound?,
        custom_beast_out_gregar: Sound?,
        custom_beast_out_flash: Sound?,
        custom_cancel: Sound?,
        custom_redeal: Sound?,
        custom_redeal_shuffle: Sound?,
        custom_scrap: Sound?,
        custom_scrap_done: Sound?,
        program_advance_part: Sound?,
        program_advance: Sound?,
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
    -- hit marker, the eruption, the target marker. (The Beast head and the
    -- idle overlay are the parts of the identities that wear them.)
    sprites: {
        charge_glow: SpriteAsset?,
        charge_glow_a: SpriteAsset?,
        charge_glow_chaos: SpriteAsset?,
        full_synchro_aura: SpriteAsset?,
        confusion: SpriteAsset?,
        blindness: SpriteAsset?,
        immobilized: SpriteAsset?,
        ice: SpriteAsset?,
        bubble: SpriteAsset?,
        hit_marker: SpriteAsset?,
        eruption: SpriteAsset?,
        target_marker: SpriteAsset?,
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
        -- A navi no player controls is being deleted (BN5's 0x08013DA0):
        -- what sparkles over it while it flashes, which its deletion's end
        -- ends.
        navi_deleted: ((navi: Object) -> Object?)?,
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
    patch_card: (spec: PatchCardSpec) -> PatchCard,
    navicust_program: (spec: NaviCustProgramSpec) -> NaviCustProgram,
    -- A rule section (§3.8): one table of rules no entity owns, by its
    -- section name (`define.rules("elements", { ... })`).
    rules: <T>(section: string, spec: T) -> T,
    roles: (spec: RolesSpec) -> RolesSpec,
    -- A system of a game's rules, and a player's ruleset
    -- (docs/design/rules-in-luau.md §2.2).
    system: (spec: SystemSpec) -> System,
    ruleset: (spec: RulesetSpec) -> Ruleset,
    -- Luau-only data with a handle and a type name, which a reference field
    -- ("record:<type>") can hold.
    record: <T>(record_type: string, spec: T) -> T,
}

-- What a system's own calls reach (docs/design/rules-in-luau.md §4.5,
-- §5.3): only in a hook the framework calls (from modules under rules/,
-- which the content check holds to).
-- A side's custom screen, in its systems' custom hooks (§4.4): the
-- refusal sound; the shared machinery for the button under the cursor (the
-- picked chips scrapped, the chips not picked dealt again); whether the
-- last pick is a chip; the state of the button under the cursor.
declare custom: {
    -- The side's battle folder as its screen has it: its 30 places in order,
    -- false where a used chip left one empty until the screen deals; two of
    -- its places swapped (1 to 30); how many chips the screen deals.
    folder: (side: number) -> { Chip | false },
    swap_folder: (side: number, a: number, b: number) -> (),
    hand_size: (side: number) -> number,
    refuse: (side: number) -> (),
    sacrifice: (side: number) -> (),
    redeal: (side: number) -> (),
    last_pick_is_chip: (side: number) -> boolean,
    cursor_state: (side: number) -> "selectable" | "unavailable" | "selected",
    -- For a system's buttons and windows: the slot under the cursor picked;
    -- a screen sound ("pick", "refused", "back", "cancel", "cursor",
    -- "description", "beast_out_gregar", "beast_out_falzar",
    -- "beast_out_flash", "cross_window_open", "cross_window_close",
    -- "cross_chosen"); the last pick's column cell; one of the system's
    -- windows opened (`ticks`: the ticks it has had already), and its ticks
    -- so far; this console's camera shake; the window's frame counter; the
    -- emblem's spin; a screen fade ("beast_out", "beast_out_back",
    -- "end_to_white", "intro_from_white"); the emotion window's face; the
    -- last pick moved first (its column icon `icon`, else its chip's); one
    -- of the system's buttons' state; the availability redone; the emblem
    -- drawn; the form the system's pick holds for the turn's start, and
    -- whether another system's holds it; whether the picks are full;
    -- whether one of the system's buttons is picked; what the screen reads
    -- of its player that isn't a system's setup (the emotion it reads, a
    -- random battle).
    pick: (side: number) -> (),
    play: (side: number, sound: string) -> (),
    set_column_icon: (side: number, chip: Chip?) -> (),
    open_window: (side: number, window: string, ticks: number?) -> (),
    window_tick: (side: number) -> number,
    shake: (side: number, magnitude: number, ticks: number) -> (),
    frame: (side: number) -> number,
    set_frame: (side: number, frame: number) -> (),
    spin: (side: number) -> (),
    fade: (side: number, mode: string, speed: number) -> (),
    set_face: (side: number, form: FormDef?) -> (),
    pick_first: (side: number, icon: Chip?) -> (),
    set_button_state: (side: number, button: string, state: "selectable" | "unavailable" | "selected") -> (),
    update_availability: (side: number) -> (),
    draw_emblem: (side: number, x: number) -> (),
    set_form: (side: number, form: FormDef?) -> (),
    form_taken: (side: number) -> boolean,
    full: (side: number) -> boolean,
    button_picked: (side: number, button: string) -> boolean,
    -- The screen's cursor (its slot) and moving it; whether a key is
    -- pressed this tick, or repeats.
    cursor: (side: number) -> number,
    set_cursor: (side: number, slot: number) -> (),
    pressed: (side: number, key: Key) -> boolean,
    repeated: (side: number, key: Key) -> boolean,
    -- The screen's drawing, for a window: the window's sprites (the
    -- emblem, the Regular chip's frame, the last turns' block), the Regular
    -- chip's frame alone, the Cross window's cursor, the chip window
    -- showing the slot under the cursor, and the window's Cross tab.
    draw_window: (side: number) -> (),
    draw_regular: (side: number) -> (),
    draw_cross_cursor: (side: number) -> (),
    show_chip_window: (side: number) -> (),
    set_cross_tab: (side: number, on: boolean) -> (),
    -- R in the window up: the form's description (three lines without
    -- one), back to the window when it closes.
    describe: (side: number, form: FormDef?) -> (),
    -- The systems' buttons' states asked again.
    refresh_buttons: (side: number) -> (),
    player: (side: number) -> {
        emotion: Emotion,
        random_battle: boolean,
    },
}

declare system: {
    -- The system's state of the side it was called for (cast it to its
    -- declared type: `system.state() :: State`).
    state: () -> any,
    -- What the player of that side brought for the system: read-only.
    setup: () -> any,
    -- That side (0 or 1).
    side: () -> number,
}

