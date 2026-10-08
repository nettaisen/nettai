-- The battle engine's content API, as Luau content sees it. A definitions
-- file for editors and the type checker (for luau-lsp, generate the combined
-- declarations with `.zed/generate-luau-definitions.py`; see README.md).
-- Every root checks against it
-- (docs/design/rules-in-luau.md §7.2: content/nettai declares the engine's
-- API and defines nothing; EXE6's own parts move to content/exe6 in slice
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
-- are compat's (content/exe6/compat), which only the tools outside the
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
-- How the first part of each of a sprite's frames is drawn, by the
-- original's three routines: "ground" (`sprite_hasShadow`: it is a shadow,
-- drawn on the ground under the object), "hidden" (`sprite_removeShadow`:
-- it is a shadow, not drawn), "with_sprite" (`sprite_noShadow`: the sprite
-- has no shadow part, and the first part is drawn as the rest are).
-- "hidden" on a sprite without a shadow part hides its first part.
export type Shadow = "hidden" | "ground" | "with_sprite"
export type ActorType = "virus" | "navi" | "player"
-- A side's emotion: a name of its game's (its rules' status section's `emotion.order`: EXE6's "normal", "tired",
-- "full_synchro", "angry", "worn_out"; EXE5's "worried" in place of "tired").
export type Emotion = string
-- A side's SELECT or Cross special in progress (each side its own gauge:
-- `battle.own_gauges`).
-- A side's special in progress: the SELECT special, or the rules' takeover
-- of the side's navi (EXE6's Cross special).
export type SideSpecial = "none" | "select" | "takeover"
-- A HUD part a chip's effect hides and shows: the custom gauge (draw task
-- 4), the emotion window (14), a side's own gauge drawn by its levels (17,
-- `battle.own_gauges`), the HP box (7: its drawing only, the low-HP alarm
-- sounds on), the chips' icons over each console's own navi (1).
export type HudPart = "gauge" | "emotion_window" | "level_gauge" | "hp_box" | "chip_icons"
-- A screen fade content starts (`SetScreenFade`'s modes): the
-- transformation sequencer's fade out (0x44, to full) and back in (0x40).
export type ScreenFade = "transform_out" | "transform_in"
-- A panel type, by name: one of its game's (its rules' `panels.numbers`).
export type PanelType = string
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
    -- EXE5's DarkInvs drive).
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
    -- The rules' takeover of the side's navi (EXE6's Cross special).
    | "takeover"
    | "volley"
    | "weakness_hit"
    -- A navi's own requests, which its own `tick` sets and its own `idle`
    -- reads; nothing else clears them (EXE5's GyroMan's: 0x10000000 and
    -- 0x20000000 of his request word).
    | "take_off"
    | "land"
    -- The slide request (the collision's flag2 0x10).
    | "slide"
    -- The anger request (the collision's flag2 0x200): angry at the next
    -- status update, unless the mood is held.
    | "anger"
    -- The drag request (the collision's flag2 0x100: a knockback's).
    | "drag"
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
    | "form_breaking"
    | "form_change_sprite_held"
    | "heat_trap"
    -- Gone from the field while its navi chip's navi acts.
    | "vanished"
    -- It dives: a panel that submerges (EXE5's sea) submerges it and doesn't
    -- hold it at a move's end (EXE5's ToadSoul). The status reset ends it.
    | "dives"
    -- In the air (EXE5's GyroMan over a panel that isn't solid): a step it
    -- takes sets no animation and leaves the one it has.
    | "hovering"
export type Key = "a" | "b" | "select" | "start" | "right" | "left" | "up" | "down" | "r" | "l"
-- A navi's drag reaction step.
export type DragStep = "start" | "slide" | "recover"
-- A navi's slide type: none, a push's, by the arena's rows and the move's
-- direction (the rules' `reactions.slide_rows`: ice's), the panel's own carry
-- (a type's `carries` or `carries_by_move`: a road's, EXE5's magnet's).
export type SlideType = "none" | "push" | "rows" | "panel"
-- What reaches a panel in a panel type's `slide`: a navi's slide, its drag,
-- an obstacle's slide.
export type SlideHow = "slide" | "drag" | "obstacle"
-- A panel type's `slide`'s answer: one panel further, carried the type's way
-- (a navi's slide), stopped.
export type SlideAnswer = "on" | "carry" | "stop"
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
declare extern type Vec3 with
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
declare extern type Sprite with
    palette: number
    hflip: boolean
    vflip: boolean
    shadow: Shadow
    -- The original's sprite flag 0x20 (`sprite_setField0x3Bit5`): with a
    -- "ground" shadow, the whole sprite is drawn as the shadow is, under
    -- every object's sprite (a mark on the floor).
    under_objects: boolean
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
declare extern type Collision with
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
    -- The status it takes this window (its hits', a counter's, or one the
    -- rules put in their place: the NaviCust's hit bug), if any.
    status_final: Status?
    -- Bug code (low byte) and argument (high byte).
    bugs: number
    -- The bug code and argument the last resolution's hits brought (CollisionData+0x9C, +0x9D), which the navi takes.
    inflicted_bugs: number
    hit_mod_base: number
    -- The damage it deals.
    self_damage: number
    -- What a panel type counts while the body stands on it (poison's drain,
    -- `sub_801A186`); the standing effects zero it on a type with no `stand`.
    standing_count: number
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
    -- Add to the damage taken this window in an element (0 to 4, 5 the
    -- sixth slot), and to the raw damage barriers and traps see where `raw`.
    function add_damage(self, element: number, amount: number, raw: boolean): ()
    -- Add to the mood damage taken this window.
    function add_mood_damage(self, amount: number): ()
    -- Set hit modifier bits this window, both sides' too.
    function add_hit_mod(self, bits: number): ()
    -- Set hit modifier bits this window, the final modifier's alone.
    function add_final_hit_mod(self, bits: number): ()
end

-- A handle to an object slot. Like the game's pointers, it can outlive the
-- object (and then names whatever takes the slot next).
declare extern type Object with
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
    -- the HUD shows them; another actor's own count (EXE5's Jealousy).
    chips_held: number
    pos: Vec3
    vel: Vec3
    related1: Object?
    related2: Object?
    -- The slot is in use (read-only).
    active: boolean
    -- It has a collision record (read-only): `collision`'s fields and methods need one.
    has_collision: boolean
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
    -- A navi's slide type, which its slide request starts.
    slide_type: SlideType
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
    -- `actions.wrapper`: EXE6's Beast Out rush).
    wrapped: number
    -- The wrapper's state starts over (`sub_801011A` sets it with the
    -- attack's links); the wrapper clears it once it has.
    wrapper_fresh: boolean
    -- The controller's state starts over (a form's `controller_restart` effect); the
    -- controller clears it once it has.
    controller_fresh: boolean
    -- Its mood is held (held tired or exhausted): what another side's rules
    -- read (EXE6's counter).
    mood_held: boolean
    -- The side's rules' `navi_tick` runs for it each tick.
    ticked: boolean
    -- Exhausted for the rest of the battle (EXE6's after Beast Over): worn
    -- out, the mood can't change, 1 HP lost per tick (never the last), no
    -- Full Synchro or anger.
    exhausted: boolean
    -- What a step or the wrapper turns to face in the panel patterns 0x23,
    -- 0x31 and 0x33 (read-only; only the unused `sub_80116F6` sets one).
    face_target: Object?
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
    -- The ticks before a panel may carry the navi again (read-only).
    slide_cooldown: number
    -- The sprite overlay attached for the current attack (writable).
    overlay: Object?
    -- Writable links: the target marker (EXE6's Beast Out lock-on marker),
    -- the charge glow, the
    -- Full Synchro aura.
    target_marker: Object?
    charge_glow: Object?
    full_synchro_aura: Object?
    -- The charge: level 0 none, 1 charging, 2 full; source 0 none, 1 A, 2 B.
    charge_level: number
    charge_source: number
    charge_counter: number
    -- The B charge's level (0 none, 1 charging, 2 full) as the navi last asked for an attack, kept until its next
    -- ask (EXE4's AIData +0x15); 0 in a game whose decode keeps none (read-only).
    b_charge_at_ask: number
    -- Writable: EXE4's rapid presses (AIData +0x12, +0x11), the B presses
    -- counted toward a form's `rapid_presses` and the ticks left for the
    -- next (the forced charged shot clears both as it ends).
    rapid_presses: number
    rapid_window: number
    -- Writable: an object tied to the navi that the full status reset ends
    -- (AIData+0x50, EXE4's +0x5C: WindSoul's wind).
    reset_linked_object: Object?
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
    -- Writable: held tired (AIData+0x32; EXE6's "the Beast Out counter is
    -- spent"): emotion 1, the mood held, no anger.
    tired: boolean
    -- Writable: the barrier's visual (AIData+0x60), which hides and shows
    -- with the navi (its state's `shown`).
    barrier_visual: Object?
    -- Writable: EXE5's DarkPlus tint (AIData+0x3C; 0 none), which picks the
    -- navi's status shader.
    plus_tint: number
    -- A B charge the side's rules arm in place of the charged shot's (EXE5's Chaos Unison charge,
    -- AIData+0x12): the ticks to a full charge (in place of the charged shot's by the Charge stat), the
    -- glow it shows (seen and heard by its side alone) and the glow's animation once full. Nil: no such
    -- charge (a weapons' load drops them all); armed while the time is set. A full charge's release is
    -- the rules' (`charge_released`, then `release_taken` from idle).
    b_charge_time: number?
    b_charge_glow: SpriteAsset?
    b_charge_anim: number?
    -- Writable: EXE5's priming (AIData+0x0D): a form's `priming` chip used,
    -- its next doubling waiting.
    primed: boolean
    -- Writable: the chip a weapon of the navi's loads (EXE5's AIData+0x32:
    -- ColonelSoul's arm chip, which its charged shot's routine 0x13 fires;
    -- nil: none).
    weapon_chip: Chip?
    -- Writable: EXE5's no-charge drive (DarkInvs): its ticks left
    -- (AIData+0x36; counted down in the intake while the navi has the
    -- no-charge state, asking for the stun strike at 0; 0xFFFF holds), and
    -- the auto battle AI driving it (AIData+0xF0, which VarSwrd's pick
    -- reads).
    no_charge_timer: number
    in_auto_battle: boolean

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
    -- As a navi leaves (EXE5's 0x081042E6): its status visuals forgotten and
    -- its chips off the HUD.
    function drop_links(self): ()
    -- `sub_801A5E2`: its status visuals forgotten (confusion's, blindness').
    function drop_status_visuals(self): ()
    -- Its chip (BattleObject +0x2A) none, its chip count kept.
    function drop_chip(self): ()
    -- `sub_800A142` (EXE4's 0x080079C6): one fewer alive navi on its side,
    -- the alive lists kept.
    function drop_alive_count(self): ()
    -- A panel that burns (its type's `burn`) burns it, an object not a
    -- player's (held while the battle is dimmed): EXE5's 0x08016D80, EXE4's
    -- 0x0801309E.
    function burn_on_panel(self): ()
    -- Its collision's barrier byte cleared (`sub_801A7F4`'s store).
    function drop_barrier(self): ()
    -- A navi no player controls leaves (EXE5's 0x08104306): no HP, the
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
    -- Whether its side's form has the trait (a form definition's `traits`).
    function form_trait(self, name: FormTrait): boolean
    -- The buttons, and the record kept while the battle is dimmed.
    function held(self, key: Key): boolean
    function pressed(self, key: Key): boolean
    function released(self, key: Key): boolean
    function dimmed_held(self, key: Key): boolean
    function dimmed_pressed(self, key: Key): boolean
    function dimmed_released(self, key: Key): boolean
    -- `object_setDefaultCounterTime`: a counter window of 16 ticks, or of
    -- `ticks` (`object_setCounterTime`).
    function open_counter_window(self, ticks: number?): ()
    -- `sub_801056A`, after each phase of an attack.
    function check_reactive_abort(self): ()
    -- `sub_80F22F8`'s spawns: the two objects a navi keeps in battle mode 9
    -- (the roles' `mode9_attack` and `mode9_actor`): EXE6's DustMan's
    -- `post_init`.
    function spawn_mode9_objects(self): ()
    -- `sub_80105F2`: the stance's own trap (the AntiDmg program's) caught a
    -- hit: its counter (the roles' `anti_damage_counter`, or
    -- `anti_sword_counter` for a sword hit) starts at the next tick with
    -- this attack's damage word and lockout.
    function start_stance_counter(self): ()
    -- `sub_8011450`: restart the form overlay with the navi after an
    -- animation change.
    function refresh_form_overlay(self): ()
    -- `object_exitAttackState`: back to idle with animation 0, the attack's
    -- lockout handed on by the rules (`reactions.attack_end_lockout`); with
    -- `keeps_lockout`, none handed on (EXE4's 0x0800CA28).
    function exit_attack(self, keeps_lockout: boolean?): ()

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
    -- EXE5's 0x080C451A: the navi's body overlay (related2) adds `offset` to
    -- its owner's animation (its depth read by the sum). Nothing without
    -- one.
    function set_overlay_anim_offset(self, offset: number): ()
    -- `sub_8011384`, `sub_8011268`: take off what a form wore, put on what
    -- it wears (its `take_off`, `put_on`, or the defaults below).
    function take_off_form_overlay(self, form: FormDef): ()
    function put_on_form_overlay(self, form: FormDef): ()
    -- The default take-off and put-on routines of a form, which its own
    -- replace: what its identity's death hook takes down (the base form:
    -- what is there), and its identity's parts. A form's hook that wants
    -- the default and more calls these.
    function take_off_form_parts(self, form: FormDef): ()
    function put_on_form_parts(self, form: FormDef): ()
    -- `sub_800FC9E` and a form change's load: the navi's sprite in `form`,
    -- its animation 0 from the start, shadow on the ground, facing, white.
    function load_form_sprite(self, form: FormDef): ()
    -- `sub_80144C0`: the full status reset.
    function reset_status(self): ()
    -- `sub_8014080` (`sub_80140EE` without `undershirt`): MegaMan's body
    -- programs go (SuperArmor, FloatShoe, Undershirt, AirShoe, the B+Back
    -- special); a navi that doesn't change form keeps its. Whether they went.
    function strip_body_programs(self, undershirt: boolean): boolean
    -- `sub_801469C`: the form's flags back on the navi.
    function refresh_form_flags(self): ()
    -- `sub_801A554`'s work for `status` now: its timer set and its
    -- requests raised (a bug's paralysis and blindness).
    function take_status(self, status: Status): ()
    -- Anger ends; the mood is back to 0x80 (EXE5's: a mood of 0 stays).
    function end_anger(self): ()
    -- The form the navi's side asked to change into at this turn's start
    -- (nil: none, or the base form).
    function form_change_target(self): FormDef?
    -- The terms of the change its side asked for: its turns, and whether it is
    -- the alternate (EXE5's Soul Unison: the soul's turns, and Chaos Unison).
    function form_change_terms(self): (number, boolean)
    -- EXE5's soul change's first step: no longer moving, flinching,
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
    -- What the navi runs: an action definition, or one of the rules' own
    -- ("entry", "take_control", "deletion", "flinch", "paralysis", "drag",
    -- "freeze", "bubble", "idle", "move", "dimming_chip", "hand_off_chip",
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
    -- step), then `end_lag` ticks; with `face`, EXE5's `sub_80116F6`'s (what
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
    -- `sub_80DBB64`: put an absorbed look (`obstacle.absorbed_look`) on the sprite:
    -- the sprite, a shadow if the look has one, its animation and palette,
    -- flipped by the object's side unless the look keeps its own; false
    -- when the look is the table's none.
    function wear_absorbed_look(self, look: Identity): boolean
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
    -- `object_subtractHP`: the HP down by `amount`, to 0, by its side's
    -- rules (EXE5's drain a player's side's gauge too, and may hold a dark
    -- MegaMan at 1 HP: his last stand).
    function subtract_hp(self, amount: number): ()
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
-- level, the hand-shrink bug's turn, the start weapons and the shoes; what
-- the patch cards change, which they write at the round's start; and what a
-- game's rules/save writes as the round is set up: the sun, the Regular
-- memory, the base HP and MegaMan's variant).
declare extern type Navi with
    -- The form and the navi, as their definitions.
    form: FormDef
    navi: NaviDef
    -- MegaMan's variant (+0x2B, his move lag): his base HP in hundreds.
    navi_variant: number
    -- The base form's element.
    element: number
    -- The buster's attack level (writable: `SetBattleNaviStatsByte`).
    attack: number
    rapid: number
    charge: number
    mood: number
    -- (`form` is writable too: a game's form change puts the navi in it.)
    -- The form the navi started the battle in (writable: a bug code puts
    -- the base form there), and its base form.
    starting_form: FormDef
    base_form: FormDef
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
    -- NaviCust bugs: random steps after a move (+0x11), damage at the
    -- battle's start (+0x3D).
    auto_step: number
    starting_damage: number
    -- The folder the navi brings (+0x2D, 0 to 2), the folders' Regular
    -- chips (+0x2E, +0x2F) and tag chips (+0x56 to +0x59), entries as the
    -- save keeps them (0xFF none).
    folder: number
    folder_1_regular: number
    folder_2_regular: number
    folder_1_tag_a: number
    folder_1_tag_b: number
    folder_2_tag_a: number
    folder_2_tag_b: number
    -- The weapons of the A button's charge (+0x39) and of battle mode 9's A
    -- button (+0x44); nil: none.
    a_charge_weapon: Weapon?
    mode9_a_weapon: Weapon?
    -- `sub_800FE52`: how many kinds of NaviCust bug the navi has.
    bug_kinds: number
    -- Its game's own stats (its rules' `stats`: EXE6's `beast_out_counter`,
    -- `sun`, `chip_shuffle`, ...; EXE5's `hub_style`, `soul_turn_bonus`,
    -- ...), read and written by name.
    [string]: any
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
-- one, a copy of the owner's sprite, wearing its form's overlay where the
-- game's afterimages do: the effects rule `afterimages_wear_overlays`), flipped
-- by the game's flip value, for `lifetime` ticks, blinking unless
-- `steady`; its shadow at its height unless said otherwise. It ends early
-- with its `tether`: when its side leaves the forms whose afterimages stay
-- (the form trait `afterimages_stay`: EXE6's Beast forms), or when its owner
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
-- (BblWrap's bubble); regenerating up to 200 HP; or back whole a while after it is worn down, as the rules' barrier
-- tick says (`status.intake.barrier.regrowing`: EXE4's BlakBarr).
export type BarrierBehavior = "plain" | "bubble" | "regenerating" | "regrowing"
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
    -- Battle flag 4 alone, with none of a dimming's machinery (EXE5's chaos
    -- failure, EXE4's soul change).
    set_dimmed: (on: boolean) -> (),
    -- BattleState+0x40, `sub_800A704`: the fight's ticks so far (capped).
    battle_time: () -> number,
    -- Slot `i` (1 to 4) of a side's list of alive actors, filled or not.
    alive_actor_slot: (side: number, i: number) -> Object?,
    -- EXE5's 0x08006AAE: the navi `identity` (actor type navi, with a
    -- `body`) comes onto (x, y) for `side`, brought by `summoner`, driven by
    -- the rules of the summoner's side (their `controller`, with the navi's
    -- state their `navi_state`): on the side's list of alive actors, not
    -- counted. Nil when a pool or the side's list is full.
    spawn_navi: (identity: Identity, x: number, y: number, side: number, summoner: Object?) -> Object?,
    paused: () -> boolean,
    -- `battle_isBattleOver`: a side has no navi left, or time is up.
    over: () -> boolean,
    -- The same as the routines that read its Z flag see it: true only once
    -- time is up (a KO reads false).
    time_up: () -> boolean,
    -- The battle's count of fight ticks modulo `period` (20 or 180), which
    -- grass heals by.
    cycle: (period: number) -> number,
    -- The shift a hit of element `hitter` takes against a body of element
    -- `receiver` (the rules' `elements.weakness`).
    weakness: (receiver: number, hitter: number) -> number,
    -- `sub_80D8DEE` (EXE4's 0x080E0842): `side` loses the round: its actor
    -- count (what the fight's result reads) to 0 and the round's time-up
    -- byte set (and battle flag 8, which nothing in EXE4 reads).
    lose_round: (side: number) -> (),
    -- `sub_800ED90` and its record's flags: the chip `user` would use next
    -- deals damage (a player's at its hand's cursor; another object's own).
    next_chip_damages: (user: Object) -> boolean,
    link: () -> boolean,
    -- A battle against a ranked boss (battle effect 1).
    boss_rank: () -> boolean,
    -- The battle's effects (its settings' word), whose bits a game's rules read by their own names.
    effects: () -> number,
    mode: () -> number,
    panel_pattern: () -> number,
    -- Every navi is in (the intro's bit 2).
    navis_in: () -> boolean,
    -- Presentation only: the side the simulation's perspective is.
    local_side: () -> number,
    -- Custom screens opened so far.
    turn: () -> number,
    -- Battle flag 0x40 (the engine's `OWN_GAUGES`): each side has its own
    -- custom gauge. Each game's mode of its own: EXE5's operation battle
    -- (content/exe5/lib/operation_battle), EXE6's chip gate battle
    -- (content/exe6/lib/chip_gate_battle). Never in a netbattle without
    -- chip gates.
    own_gauges: () -> boolean,
    -- Battle flag 1: the fight is on (collision is live).
    fighting: () -> boolean,
    -- Battle flag 2: the custom gauge is full.
    gauge_full: () -> boolean,
    -- A link battle's last turns, from its 15th custom screen (`sub_800A97A`).
    late_turns: () -> boolean,
    -- A screen fade runs (`IsScreenFadeActive`).
    screen_fading: () -> boolean,
    -- `SetScreenFade`: the screen fades out (to full) or back in (to clear)
    -- from where it is, `speed` a step.
    screen_fade: (fade: ScreenFade, speed: number) -> (),
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
    -- shake even while the battle is paused (EXE5's TomahawkSoul's change).
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
    -- The mood rises by `n` to 254 at most; 0 and 0xFF stay (EXE5's
    -- 0x08012802: its recovery chips).
    gain_mood: (side: number, n: number) -> (),
    -- The mood falls by `n` to 1 at least; 0 stays (`sub_8015C12`, EXE5's
    -- 0x08012820: its navi chips leaving).
    lose_mood: (side: number, n: number) -> (),
    -- Whether the side's console starts its emotion window glitching (the
    -- save's NaviCust bug flag): the rules decide it in `round_setup` (EXE6's
    -- patch cards do).
    set_emotion_window_glitch: (side: number, on: boolean) -> (),
    -- A definition's name in the content's own strings (its locales'
    -- language), else its key: for what the rules say to a tool
    -- (`validate`).
    name: (def: any) -> string,
    -- The folder a tool asks the rules to check (the rules' `folder_check`):
    -- whose side, its chips in order, its Regular and tag chips (places in
    -- `chips`), and whether it is all of a folder (else the chips so far:
    -- the rules about a whole folder wait); nil when none is.
    checked_folder: () -> CheckedFolder?,
    -- A rule the checked folder breaks: the rule's name and what to say.
    folder_problem: (rule: string, text: string) -> (),
    -- `sub_802E4B8`: the side's SELECT or Cross special in progress.
    side_special: (side: number) -> SideSpecial,
    -- The rules' takeover of the side's navi starts for `ticks` (counted
    -- down in the navi's stage B, `sub_802E1D8`), or ends; while it runs,
    -- idle asks the side's rules' `takeover`. Its ticks left.
    take_over: (side: number, ticks: number) -> (),
    end_takeover: (side: number) -> (),
    takeover_ticks: (side: number) -> number,
    player: (side: number) -> Object?,
    -- A side's combatants still in, in slot order.
    alive_actors: (side: number) -> { Object },
    -- `sub_802EFEE`: the actor `side` tracks when each side has its own
    -- gauge (`own_gauges`), if any.
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
    -- `sub_801EB18`: the other player's console names `chip` as used by
    -- `side` again, with its damage and bonus (presentation only: EXE4's
    -- Slasher as it slashes).
    show_used_chip: (side: number, chip: Chip, damage: number, bonus: number) -> (),
    -- `sub_800ABC6`: the trap's mark over `o`'s panel (the role
    -- `effects.trap_mark`, facing the local side's way, where the chip-use
    -- rules' `anti_navi_sparkle` puts it from the panel's center), with its
    -- sound (the role `sounds.cut_in`): its height.
    trap_mark: (o: Object) -> number,
    -- `sub_80E49C4` (BugFix): a side's NaviCust bugs are fixed (the stats
    -- processing, the panel trail's level, the buster's blanks, the on-hit
    -- status, the custom damage, the emotion, the drains, the battle-start
    -- bug and the hand-shrink turn zeroed).
    clear_bugs: (side: number) -> (),
    -- `sub_801E658` (BugFix): the save's emotion window glitch is gone from
    -- every console's emotion window, which then flickers (and draws its
    -- console's RNG1) for NaviCust bugs only.
    clear_emotion_window_glitch: () -> (),
    fill_custom_gauge: () -> (),
    -- `sub_801DFD0` (EXE5's CusVolt): the custom gauge drops by `n`, to 0 at least.
    drain_custom_gauge: (n: number) -> (),
    -- `sub_801DF8C`: the custom gauge fills `rate` a tick (full at 0x4000).
    set_gauge_rate: (rate: number) -> (),
    -- A side's slow and fast gauge timers (`sub_802E070`+0x3C, +0x3A).
    set_gauge_speed_ticks: (side: number, slow: number, fast: number) -> (),
    -- `sub_8010B78`: the damage a side's custom gauge gives (its own gauge
    -- when each side has one, `own_gauges`, else the shared one).
    gauge_damage: (side: number) -> number,
    -- A side's sword pick (`sub_802E070`+0x12): the swing a variable sword
    -- makes for a navi no buttons drive (EXE5's auto battle draws it).
    sword_pick: (side: number) -> number,
    set_sword_pick: (side: number, pick: number) -> (),
    -- Presentation: whether the side's emotion window shows its form's second set of faces (`mugshot.variant`): in the
    -- base form ("base", the default: EXE5's Hub Style), or whatever the form while its navi's rules' B charge is
    -- armed ("charged": EXE5's Chaos Unison look).
    set_face_variant: (side: number, variant: boolean, mode: ("base" | "charged")?) -> (),
    -- Presentation: whether the custom screen's enemy names show the side's navi by its variant name (the locale's
    -- `variant_name`: EXE5's Hub Style in a link battle).
    set_name_variant: (side: number, variant: boolean) -> (),
    -- Presentation: whether the side's emotion window shows its count (its navi's stat of the role `window_count`:
    -- EXE6's Beast Out turns), else its empty box.
    set_window_count: (side: number, shown: boolean) -> (),
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
    -- The same spawner called again straight after a spawn, at height z:
    -- its X and Y what the object allocator left in their registers (list
    -- node addresses), which the engine doesn't know.
    effect_after_spawn: (z: number, effect: Effect, flip: number?, palette_add: number?, priority: number?) -> Object?,
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
    -- EXE5's DarkPlus: the Atk+ bonus of a hand's chip at i becomes n.
    set_hand_attack_bonus: (side: number, i: number, n: number) -> (),
    -- Adding to the charge bonus of a hand's chip at i (what a form's A
    -- charge builds up, and EXE4's souls' status resets give: the status
    -- reset clears it).
    add_hand_charge_bonus: (side: number, i: number, n: number) -> (),
    -- A side's hand has a chip at i and it does damage (its record's
    -- "has_damage" flag).
    hand_chip_damages: (side: number, i: number) -> boolean,
    -- The chip at i of a side's hand (from its first chip, used ones too),
    -- if any.
    hand_chip: (side: number, i: number) -> Chip?,
    -- EXE5's Jealousy (0x080E4596): the chips left in a side's hand, from
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
    -- something stands on it; with `sound` in place of the panel crack's
    -- (`object_breakPanelLoud`'s).
    break_panel: (x: number, y: number, sound: Sound?) -> boolean,
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
    -- What the panel's type counts (an `expires` type's ticks left, EXE4's
    -- pitfall's crumble), and setting it.
    timer: (x: number, y: number) -> number,
    set_timer: (x: number, y: number, ticks: number) -> (),
    -- The field's cycle (the panel rules' `cycle`: EXE6's volcanos erupt by
    -- it), counted down each panel update; 0 in a game without one.
    cycle: () -> number,
    -- A grounded body stands on the panel: a body, none floating.
    grounded_body: (x: number, y: number) -> boolean,
    -- Drawn only.
    highlight: (x: number, y: number) -> (),
    -- `object_panel_setPoison` and its kind: a solid panel becomes the type
    -- in place (its flags keeping what the type mask doesn't own, taking the
    -- type's own), no type change's hooks, with `sound`; whether it was solid.
    overwrite: (x: number, y: number, kind: PanelType, sound: Sound?) -> boolean,
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
    -- A telop for a controller whose effect may already run
    -- (`sub_800BA8A`'s): its first tick starts it, naming `chip` unless the
    -- controller's telop names one (nil: a zeroed chip field's, the roles'
    -- `zeroed` chip), once the other side isn't mid-dimming (a record
    -- already running stays so); once the banner is done and no cut-in
    -- holds the side, the side's dimming runs: true. What follows (the next
    -- action, or another) is the caller's.
    telop_running: (controller: Object, chip: Chip?) -> boolean,
    -- The navi that used `side`'s dimming chip (the record's +0xC; nil once
    -- the dimming changed sides).
    user: (side: number) -> Object?,
    -- `side`'s dimming runs its effect from now (the record's state 4).
    run: (side: number) -> (),
    -- A telop for `side` naming `chip` (nil: the zeroed chip's) without
    -- damage, and its sound; a used chip's name makes way.
    chip_telop: (side: number, chip: Chip?) -> (),
    -- The telop's banner is done.
    telop_done: () -> boolean,
    -- The dimming changes sides (`sub_800BE2C`'s): `controller`'s side's
    -- dimming is over (its user stays recorded), and `controller` takes the
    -- other side, which now owns, runs and started the dimming (its own
    -- controller, if any, is told to end).
    turn: (controller: Object) -> (),
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
    -- and comes back (EXE5's Django's coffin).
    hide_actor: (o: Object) -> (),
    show_actor: (o: Object) -> (),
}

declare navi_chip: {
    -- What a controller brought is done: the controller's kind's
    -- `navi_left`.
    navi_left: (controller: Object) -> (),
    -- `sub_80E1332`: the user warps out (`out`) or back in (the navi warp,
    -- the engine's actor #0x2D).
    warp: (user: Object, out: boolean) -> (),
    -- A chip's use springs the other side's armed AntiRecv (EXE6's
    -- `loc_80E1968`, Roll's chips; EXE4's 0x080E5554): the trap's mark over
    -- `user`, the other side's defensive-chip record spent, and AntiRecv's
    -- counterattack (the role `kinds.anti_recovery`) against `user` with the
    -- damage word `damage`; `names_trap`: its telop names the record's chip
    -- (AntiRecv; else none, the caller's telop then). The counterattack (nil:
    -- the pool is full); its dimming is the caller's to register.
    spring_anti_recovery: (user: Object, damage: number, names_trap: boolean?) -> Object?,
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
    -- The HP the entry gives it, if it states one.
    hp: number?,
}

-- How an absorbed obstacle looks (objects/absorbed_obstacle's `look`, an
-- "absorbed-look" record; the original's obstacle kinds, `byte_80E98C0`):
-- its sprite; `keeps_parts`, it flies with the obstacle's hidden sprite
-- parts (the original's kind 2); `own_facing`, thrown, its sprite's parts
-- keep their own facing and it isn't flipped (sprite 0x23 in the
-- original's test).
export type AbsorbedLook = { sprite: SpriteAsset, keeps_parts: boolean?, own_facing: boolean? }

export type ObstacleCrush = "breaks" | "destroys" | "spares_bodies" | "ignores" | "destroys_sparing_bodies"
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
-- (then the kind's own action table, in the framework's numbering: its
-- entries for actions 0 appear, 2 destroyed and 8 idle, and the shared
-- ones below for 1, 3 to 7), the
-- sprite, and presenting its collision.
declare obstacle: {
    -- `setFieldBattleObject_800F614`: one of `side`'s field objects of
    -- `class` (0: two a side, 1: one); a third evicts the oldest (its HP
    -- drops to 0).
    register: (me: Object, side: number, class: number) -> (),
    -- `sub_800F656`: forget it in the field-object registry.
    unregister: (me: Object) -> (),
    -- `sub_801AD9E` and its variants: resolve this tick's hits. What a
    -- pushing hit (a push with its game's drag bit) does: "forgets_damage" (the default,
    -- `sub_801AD9E`: pushed, its damage forgotten), "keeps_damage"
    -- (`sub_801AD12`: pushed, the damage lands), "any_hit" (`sub_801ADFA`:
    -- that, and any one side's hit but type 0x1000 pushes it a panel away;
    -- EXE5's, 0x08017A78, by the hitters' flips: its game's rules'
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
    -- while LilBoiler erupts, such a touch is as any hit) or
    -- ("destroys_sparing_bodies": EXE4's 0x08014058, "destroys" but no
    -- body's touch and no removal request): damage, removal
    -- and status;
    -- the action the kind's table runs now, nil when a status routine ran.
    -- While dimmed it holds the obstacle still once it has appeared
    -- ("after_appearing", the default) or always ("always", `sub_801B750`).
    react: (me: Object, crush: ObstacleCrush?, hold: ObstacleHold?) -> number?,
    -- The framework's action numbers are EXE6's (the kind's own from 8: its
    -- idle, then the rest); the game may store others (EXE5's obstacles
    -- have no frozen or bubbled entries: their own from 6). `react` returns
    -- the framework's number; `action_byte` gives the byte to store for
    -- one (as `me.action`, or to `me:set_action`), `current` the object's
    -- action in the framework's numbering.
    action_byte: (me: Object, action: number) -> number,
    current: (me: Object) -> number,
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
    -- absorbed obstacle of look `look` (objects/absorbed_obstacle's
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
    -- "identity" state field (`me:wear_absorbed_look`); nil for the objects
    -- DustMan leaves (`sub_800F486`: an identity with `scrap = false`).
    absorbed_look: (o: Object) -> Identity?,
    -- `sub_80DC3B2`'s test: a field object by its identity, but those
    -- `sub_800F486` excludes (`scrap = false`): what BlzrdBal's ball
    -- swallows.
    swallowable: (o: Object) -> boolean,
    -- EXE5's Poltergeist's test (0x080E8CA0): `o`'s identity is `throwable`.
    throwable: (o: Object) -> boolean,
    -- `sub_800F6AC`: ask the field object `o` to be picked up by `side` and
    -- thrown at panel (x, y) after shaking `shake` ticks, hitting with the
    -- damage word `damage`.
    throw: (o: Object, side: number, x: number, y: number, shake: number, damage: number) -> (),
    -- EXE5's ColonelSoul army (docs/design/exe5-map.md §15.11): arm side
    -- `side` (BattleState+0x5C bit 0x10 or 0x20, 0x080CAC1E) with its
    -- soldiers' damage words (0x080CABF8): the sword soldier's and the gun
    -- soldier's. While a side is armed, an obstacle's reaction asks the
    -- rules (`obstacle_reaction`: EXE5's turns one standing on a panel of
    -- the side's enemy, with one of the enemy's bodies on its row, into the
    -- side's soldier).
    arm_conversion: (side: number, melee: number, ranged: number) -> (),
    -- 0x080CAC30: disarm side `side`; its words stay.
    disarm_conversion: (side: number) -> (),
    -- 0x080CABF8 alone: side `side`'s soldiers' damage words become these,
    -- armed or not (EXE5's BusterUp in ColonelSoul, 0x08011C50).
    set_conversion_words: (side: number, melee: number, ranged: number) -> (),
    -- Whether side `side` is armed, and its words (0x080CAC06,
    -- 0x080CAC12): what its soldiers strike with as they read them.
    conversion: (side: number) -> (boolean, number, number),
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

-- Assets, named while content loads by the game's asset pack's names (`asset.sprite("bomb")`, a game's
-- compat/assets.toml's): one value per asset. Only a game's modules have `asset`; a support pack's take
-- theirs from their callers. (`SpriteAsset`, since `Sprite` is an object's sprite.)
declare extern type SpriteAsset with
    name: string
end
declare extern type Sound with
    name: string
end
declare extern type Banner with
    name: string
end
declare extern type Background with
    name: string
end
declare extern type Mugshot with
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
-- "i16", "i32", "object", "vec3", "code" (a chip code, "A" to "Z" or "*"),
-- an array ("u8[18]"), a list of variant names, a reference ("kind",
-- "action", "effect", "spark", "region", "collision", "chip", "weapon",
-- "record", "record:<type>" (records of one type), "sprite", "sound",
-- "banner", "background", "mugshot": a definition or an asset, or nil), a
-- record (a table of fields of its own: `{ chip = "chip", code = "code" }`)
-- or a list (`schema.list(T, n)`). A record's fields and a list's
-- elements are read and written each by its place (`s.drive.mode`,
-- `s.folder[2].code`, `#s.folder`); a list grows by its next element
-- (`s.folder[#s.folder + 1] = ...`).
export type SchemaField = string | { string } | SchemaList | SchemaRole | StateSchema
export type StateSchema = { [string]: SchemaField }
-- A list's declaration (`schema.list(T, n)`): up to `n` elements of `T`.
export type SchemaList = { __list: boolean, of: SchemaField, max: number }
-- A role's declaration (`schema.role(name, T)`): a field of type `T` the
-- engine knows by the role `name` (a player's setup field it reads:
-- "form_list", the forms a form list window offers; a state field a view
-- shows: "form_list.cursor").
export type SchemaRole = { __role: boolean, role: string, of: SchemaField }

-- What a state's or a setup's declaration takes beside type names, lists of
-- variants and records: `schema.list(T, n)`, a list of up to `n` (1 to
-- 65535) elements of type `T` (a type name, a list of variants, a record
-- or another list), and how many it holds; `schema.role(name, T)`, a field
-- of type `T` that the engine knows by the role `name`.
declare schema: {
    list: (of: SchemaField, max: number) -> SchemaList,
    role: (role: string, of: SchemaField) -> SchemaRole,
}

-- An object kind: one behavior; what differs between uses goes in its
-- state when spawned. `id` is qualified by its owner's folder when it lives
-- in one ("minibomb/held").
export type KindSpec = {
    id: string?,
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
-- (§4.1). The rules see only their own state, of the side they are called
-- for.
-- What a frontend draws of a button besides its look, which is its pack's
-- by the button's name (docs/design/rules-in-luau.md §4.8): "form_offer",
-- it offers a form (the picked column shows the offered form's icon, and
-- its picture is in its second palette for the form's alternate offer; the
-- rules' state keeps the fields of the roles `form_offer.form`, a form,
-- `form_offer.alternate`, a bool, and `form_offer.turns`, the turns left in
-- the form it gave, which the emotion window counts: `schema.role`);
-- "chip_picture", its picture in the chip window is also the picture of the
-- chip the rules' role `button_chip` names.
export type ButtonView = "form_offer" | "chip_picture"
-- What a frontend draws while a window is up (§4.8). A list of forms to
-- choose from, in its four windows: "form_list_opening", "form_list",
-- "form_list_closing" and "form_chosen" (the rules' state keeps the fields
-- of the roles `form_list.offered`, a u8 array of the places offered,
-- `form_list.count`, `form_list.marked`, a bool array, `form_list.cursor`,
-- `form_list.chosen`, a bool, and `form_list.chosen_place`).
-- "offer_flight": the form a "form_offer" button offers flies to the picked
-- column as its icon (`offer_flight.step` and `offer_flight.count`).
-- "chip_flight": the chip a button shows flies there (`chip_flight.button`,
-- which of the screen's buttons that show a chip, from 1,
-- `chip_flight.step` and `chip_flight.count`). The engine finds each by its
-- role (`schema.role(role, T)`), never by a field's name.
export type WindowView = "form_list_opening" | "form_list" | "form_list_closing" | "form_chosen" | "offer_flight" | "chip_flight"
-- A custom-screen button of the rules (docs/design/rules-in-luau.md §4.4).
export type ButtonSpec = {
    -- Its first slot, and how many it takes (1 or 2).
    slot: number,
    cells: number?,
    -- What a frontend draws of it besides its look, if anything.
    view: ButtonView?,
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
    -- As the screen lays it out: the chip it shows, if any (EXE5's capsules:
    -- the chip window shows the chip for it, and R describes it).
    chip: ((side: number) -> Chip?)?,
    -- At OK while it is picked: the chip it puts in the hand in its place, in
    -- code A, if any (EXE4's soul button: the soul's chip, which changes the
    -- navi as the turn starts).
    hand_chip: ((side: number) -> Chip?)?,
}
-- A custom-screen window of the rules (§4.4): its update, each tick it is up
-- (`custom.window_tick` counts them from 1): whether it stays up.
export type WindowSpec = {
    -- What a frontend draws while it is up, if anything of its own.
    view: WindowView?,
    update: (side: number) -> boolean,
}
-- A game's rules, their one definition (its rules/init.luau; a second
-- `define.rules` is a load error; docs/design/rules-in-luau.md §2.5): what a
-- side keeps through a round, what a player brings, the hooks the framework
-- calls (each a plain function, which says in its code which of the game's
-- modules it runs), the custom screen's buttons and windows, the game's
-- fields on its definitions. It has no `id` and no variants: every match of
-- the game plays by it.
--
-- It also holds the game's rule sections (docs/design/content-model-v2.md
-- §3.8: tables of rules no entity owns, each a plain table its module
-- returns, `panels = require("@self/panels")`), which the engine reads
-- against its schemas when the rules are defined, and the game's roles
-- (§7.4, `roles = require("@self/roles")`). The engine has no game's rules
-- of its own: the rules state `chip_use`, `effects`, `flow`,
-- `fresh_stats`, `panels`, `pools`, `reactions` and `status` (required
-- here, so rules without one don't type-check), and every field of
-- them that has no neutral value (one left out is a load error that names
-- it). A section it may leave out is a feature the game
-- hasn't (`lockon`, `navicust`, `banners`) or a
-- table that is empty without it.
--
-- A section is a table of its rules by name, which the engine reads by its
-- schema. Named records need no string indexer. A table, rather than `any`,
-- keeps required sections required; the loader validates each section's fields.
export type RuleSection = {}
export type RulesSpec = {
    -- What a side keeps through a round (its state, which only the rules' own
    -- calls reach: `rules.state()`).
    state: StateSchema?,
    -- What a player brings (a side's facts, by these names: the match
    -- format's), read-only in battle (`rules.setup()`).
    setup: StateSchema?,
    -- The state of each navi no player controls that its `controller`
    -- drives (the navi object's own: an object's state, 64 bytes at most).
    navi_state: StateSchema?,
    -- A navi's stats of the game's own, beside the engine's (16 bytes at
    -- most: the stats' every copy carries them): `battle.navi(side)` reads
    -- and writes them by name, a compat crate maps them to its block's
    -- bytes, `fresh_stats` gives their fresh values, and a frontend reads
    -- one by its role (`schema.role("window_count", "u8")`: the count the
    -- emotion window's box shows).
    stats: StateSchema?,
    -- What a player's setup that says nothing of a field holds (else zero): a
    -- value of each field given (an enum's by name, a definition's by its
    -- id; an array's or a list's, a list of them from its first element; a
    -- record's, a table of its fields). An enum of the setup has no default
    -- but one given here: a player's setup states it, or the round doesn't
    -- start (the engine picks none of its names). (The load checks each
    -- against its field: a number, a flag, a name, a list, a table of
    -- fields, as deep as the field's type nests them.)
    setup_defaults: { [string]: any }?,
    -- Its own actions, which reach the rules' state (`rules.state()`) for
    -- the side of the navi running them.
    actions: { Action }?,
    -- Its custom-screen buttons, by name (the key a pack has each one's look
    -- under): where each sits and its functions, which run as the rules'.
    buttons: { [string]: ButtonSpec }?,
    -- Its custom-screen windows, by name (`custom.open_window`).
    windows: { [string]: WindowSpec }?,
    -- Its fields on its game's definitions (docs/design/rules-in-luau.md
    -- §7.5), by registry (chip, form, navi): each a type name, a list of
    -- variants, or a table of such fields. The engine checks a definition's
    -- and reads none of them.
    extends: { [string]: { [string]: any } }?,
    -- Its custom screen's extras (docs/design/rules-in-luau.md §4.4), which
    -- the side's screen asks.
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
        -- `custom.repeated`), before the screen's own: whether the rules
        -- took them.
        keys: ((side: number) -> boolean)?,
        -- B with nothing picked: whether the rules took something back
        -- (none: B is refused).
        take_back: ((side: number) -> boolean)?,
    }?,
    hooks: {
        -- Once per side as the round is set up, before anything reads the
        -- side's navi stats (`battle.navi(side)`), which it may change.
        round_setup: ((side: number) -> ())?,
        -- Once per side, after the navis spawn.
        round_start: ((side: number) -> ())?,
        -- The turn-start sequencer's check begins (`sub_801486C`'s entry;
        -- EXE5's 0x08011DDC), each side in turn, before either side's
        -- `turn_check` or change of form: the side's request for the turn is
        -- read (EXE5's ColonelSoul takes its arm chip).
        turn_opened: ((side: number) -> ())?,
        -- The turn-start sequencer's check of a side that isn't changing
        -- form or asks for a navi switch (`sub_801486C`): a form whose time
        -- ran out asks to revert.
        turn_check: ((side: number) -> ())?,
        -- The turn starts, after the sequencer (`sub_800840C`): a turn in a
        -- form is spent.
        turn_started: ((side: number) -> ())?,
        -- A custom screen was asked for mid-fight (`sub_8008452`).
        custom_requested: ((side: number) -> ())?,
        -- Both results are in and each hand with chips is installed
        -- (`sub_800B3D8`), the tick before the fight resumes: EXE4's dark
        -- chips' costs (0x0801EA1E).
        custom_result: ((side: number) -> ())?,
        -- Both results are in and the fight resumes (`sub_8009338`).
        custom_closed: ((side: number) -> ())?,
        -- A tool asks whether a folder keeps the side's game's folder rules
        -- (not the simulation): the folder is `battle.checked_folder()`, and
        -- each rule it breaks is `battle.folder_problem(rule, text)`.
        folder_check: ((side: number) -> ())?,
        -- A tool asks what is wrong with side `side`'s setup (not the
        -- simulation), once the round is set up: each problem a sentence, or
        -- with the setup field it is of and the entry of a list field (from
        -- 1). Nothing: none.
        validate: ((side: number) -> { SetupProblem }?)?,
        -- Each tick of the fight, in the navi's intake after the standing
        -- effects (`sub_801AC6C`; EXE5's light and dark MegaMan clears a holy
        -- panel there).
        navi_intake: ((side: number, navi: Object) -> ())?,
        -- A chip's use is prepared (the end of `sub_80127C0`): nil lets it
        -- be used, a chip is what the navi uses instead.
        chip_check: ((side: number, navi: Object, chip: Chip?) -> Chip?)?,
        -- Earlier in the preparation, before the hand bonus's panel is
        -- spent (EXE5's 0x08010030: a dark chip's use and the chip's cost):
        -- as `chip_check`; a chip in the use's place skips the rest,
        -- `chip_check` too.
        chip_cost: ((side: number, navi: Object, chip: Chip?) -> Chip?)?,
        -- A chip's use is prepared, before its record is loaded
        -- (`sub_8010D58`): a chip is what the navi uses in its place, with
        -- its own record, damage and bonus; nil, the chip.
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
        -- (EXE6's Beast Over, `sub_802D322`), in place of the player's
        -- decisions: what it did ("nothing", "chip": a chip's use started,
        -- "buster": the buster is to fire, "moved": a step started), which
        -- the framework carries out as idle does.
        controller: ((side: number, navi: Object) -> ("nothing" | "chip" | "buster" | "moved")?)?,
        -- The framework reverts the navi to its base form, its form not yet
        -- changed (`sub_80158CC`).
        form_reverted: ((side: number, navi: Object) -> ())?,
        -- Idle finds the navi's takeover request (`sub_802E4E4`; EXE6's
        -- DarkInvs asks for its Cross special): the rules start it
        -- (`battle.take_over`).
        takeover_requested: ((side: number, navi: Object) -> ())?,
        -- The side's navi landed a counter on `victim` (`sub_801A200`).
        countered: ((side: number, victim: Object) -> ())?,
        -- Each unpaused tick, after the navi's input (`sub_8013DA0`'s
        -- place), for a navi the rules asked it for (its `ticked`).
        navi_tick: ((side: number, navi: Object) -> ())?,
        -- The mood the side's navi starts the round with (`sub_8015C2C`'s
        -- 0x80, where `sub_8013892` sets it; EXE5's by the light/dark value,
        -- 0x0801283A); none, 0x80.
        starting_mood: ((side: number) -> number?)?,
        -- Each tick, the sprite palette of the side's navi of a player's
        -- kind (presentation; `sub_801002C`, EXE5's 0x0800DD94); none, the
        -- framework's (EXE6's).
        navi_palette: ((side: number, navi: Object) -> number?)?,
        -- The navi takes its hit's NaviCust bug (`sub_80139F6`, EXE5's 0x0801103E: `navi.collision.inflicted_bugs`,
        -- its code and argument): the game's table writes the stat a code names (@exelib/navicust/bugs), and
        -- answers "edited" (the abilities and form flags come back), nil (nothing of the stats changed) or "spared"
        -- (the weapons' reload skipped too). Asked on a tick something hit the navi.
        navi_bug: ((side: number, navi: Object) -> ("edited" | "spared")?)?,
        -- Any navi (a player's or not) takes the bugs its hit's code brings every navi (`sub_801A6B4`,
        -- `sub_801A720`: the HP bug's level, paralysis and blindness), before its status and damage; asked when a hit
        -- brought a code.
        hit_bug: ((side: number, navi: Object) -> ())?,
        -- A full B charge is released while the rules' own B charge is armed (`navi.b_charge_time`; EXE5's Chaos
        -- Unison, 0x0801086E): they note what it does, which idle then has them start (`release_taken`, the navi out
        -- of idle: EXE5's 0x080F034E and 0x080F0382).
        charge_released: ((side: number, navi: Object) -> ())?,
        release_taken: ((side: number, navi: Object) -> ())?,
        -- Whether the navi's hit's bug shows the HP bug's marker (`sub_801A4A6`), as its hits' effects show; asked
        -- when a hit brought a code.
        bug_mark: ((side: number, navi: Object) -> boolean?)?,
        -- A player's navi's hits ask it to flinch or be dragged (its requests 0x104: `sub_8013F1E`, EXE5's
        -- 0x0801156E), before the hit's status takes: the NaviCust's hit bug (`hit_status`), behind the game's gate,
        -- may put its own in its place (`navi.collision.status_final`).
        navi_flinched: ((side: number, navi: Object) -> ())?,
        -- A loss of HP brought the side's navi to 0, by a game whose HP loss asks (`status.hp_loss = "gauge"`: EXE5's
        -- object_subtractHP, 0x0800C6E0, calling 0x0802C16C): the rules may hold it (EXE5's last stand), and answer
        -- true: the hit shows (white, its sounds: EXE5's applyDamageToPlayer, 0x080185A2); none, it doesn't.
        hp_emptied: ((side: number, navi: Object) -> boolean?)?,
        -- An obstacle's reaction (after the damage and the crushing hits) leaves it standing, outside the dimming and
        -- past its first action, while a side is armed (`obstacle.arm_conversion`): the rules may turn it into
        -- something else and break it (its HP and max HP to 0: EXE5's ColonelSoul army, 0x080CAB02). `side` is the
        -- obstacle's.
        obstacle_reaction: ((side: number, obstacle: Object) -> ())?,
        -- Each tick of the idle action of a navi whose side's takeover runs
        -- (`sub_802D4C6`), in place of the player's decisions: what it did,
        -- as `controller` answers, or "own_chip" (an attack of its own
        -- started: the attack's chip). The rules end it
        -- (`battle.end_takeover`).
        takeover: ((side: number, navi: Object) -> ("nothing" | "chip" | "buster" | "moved" | "own_chip")?)?,
    }?,
    -- The sections every game states.
    chip_use: RuleSection,
    effects: RuleSection,
    flow: RuleSection,
    -- What a link battle picks at random with its settings, for whoever
    -- makes a random match (the engine picks none): `stages`, the stage for
    -- each index of the pick (a stage as often as an index reaches it;
    -- none: no random pick), and `backgrounds`, the background for each
    -- (none: the stage's own).
    link_pick: RuleSection,
    fresh_stats: RuleSection,
    panels: RuleSection,
    pools: RuleSection,
    reactions: RuleSection,
    status: RuleSection,
    -- The sections a game may leave out.
    banners: any?,
    buster: any?,
    custom_screen: any?,
    elements: any?,
    math: any?,
    navicust: any?,
    roles: RolesSpec?,
}
export type Rules = RulesSpec

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
    id: string?,
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
-- A chip's damage by formula: the opponent's HP; the custom gauge's (a full
-- gauge's `full`: EXE4's 0, none EXE6's and EXE5's 10); the HP
-- the user has lost (at most `cap`: EXE6's 500 when none); the last two
-- digits of the user's HP; half the opponent's max HP; a link navi's own
-- chip's, by its buster's attack level; by a count of the user's side
-- (EXE5's DS navi chips, Roll SP's, Django SP's: `by_count[n]`, the last
-- for more). EXE5's count formula gives `operation_battle` in its operation
-- battle. (A chip whose damage goes by its side gives a function of the
-- side and the chip instead, `ChipSpec`'s `damage`: EXE5's team navis' own
-- chips by their level, both games' SP navi chips by the side's deletion
-- time, rules/sp_chips.)
export type DamageFormula =
    { formula: "opponent_hp" }
    | { formula: "gauge", full: number? }
    | { formula: "gauge_level", base: number }
    | { formula: "hp_lost", cap: number? }
    | { formula: "hp_last_digits" }
    | { formula: "half_opponent_max_hp" }
    | { formula: "navi_level", base: number, per_level: number }
    | { formula: "count", of: DamageCount, by_count: { number }, operation_battle: number? }

-- What a count formula counts: a side statistic, the side's panels of a
-- type, the field's panels of a type (either side's), the turns before this
-- one.
export type DamageCount = { side_stat: number } | { own_panels: PanelType } | { panels: PanelType } | "turns_before"

export type ChipSpec = {
    -- Its id, its key in the game's root (`chips.cannon`): the loader's to
    -- read, never written in the table.
    id: string?,
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
    -- A number below 1000, or a formula the rules compute when the chip
    -- enters a hand.
    -- A number, a formula, or a function of the side and the chip, which the
    -- round's setup asks once for each side (EXE5's team navis' own chips:
    -- their row at the side's level, lib/navi_level; the SP navi chips: by
    -- the side's deletion time for the chip, rules/sp_chips).
    damage: (number | DamageFormula | (side: number, chip: Chip) -> number)?,
    hit_param: number?,
    lockout: number?,
    flags: { string }?,
    -- Named bits, and the value of a bit the engine doesn't name yet.
    extra_flags: { string | number }?,
    slot_in_limit: number?,
    -- What the rules ask of the chip: "no_chain" (the Beast rush's
    -- chain leaves it out), "aura_bonus" (it hits harder while the user's
    -- barrier holds), "no_cut_in" (the other side can't cut in on its
    -- dimming), "element_sword" (a link navi's own chip's family bonus
    -- counts it), "goes_with_any" (it goes with any selection on the custom
    -- screen, and leaves the selection's rules alone: EXE6's BeastOut chip),
    -- "hides_damage_as_a" (the custom screen shows its damage as "???" on a
    -- copy of code A), "no_icon_when_next" (next in the hand it shows no icon
    -- over its user: EXE4's souls' chips).
    traits: { string }?,
    -- The trap it is while a side's defensive-chip record holds it:
    -- "anti_damage", "anti_sword", "body_guard", "anti_navi",
    -- "anti_recovery".
    trap: string?,
    modifier: any?,
    program_advances: { any }?,
    -- Its use: exactly one of these.
    action: Action?,
    dimming: ((user: Object, spec: DimmingChipSpec) -> Object?)?,
    -- A chip handed off to its controller (the user goes back to idle at
    -- once): the controller's spawner, as a cut-in chip's `dimming`, or a
    -- table with it as its `hook` (and whatever else the content keeps of
    -- the chip's controller: EXE6's and EXE5's navi chips', @exelib/navi_chips).
    navi: (((user: Object, spec: DimmingChipSpec) -> Object?) | { hook: (user: Object, spec: DimmingChipSpec) -> Object? })?,
    instant: ((user: Object, spec: InstantChipSpec) -> ())?,
    -- What the chip itself does to the attack as a navi's use of it is
    -- prepared (the attack's variables are loaded: the navi's `charged`,
    -- its action's state): EXE5's team navis' own chips'.
    setup: ((navi: Object) -> ())?,
    -- Its game's rules' fields (their `extends.chip`: EXE6's `beast`,
    -- `dark_substitute`, `hp_bug`; content/exe6/types.d.luau types them).
    [string]: any,
}
export type Chip = ChipSpec

-- A chip family, by name: one of its game's (its rules' `elements.families`).
export type ChipFamily = string
-- An element, by name.
export type ElementName = "null" | "fire" | "aqua" | "elec" | "wood"
-- The chips a navi's or a form's rule is about: an element's, or a
-- family's.
export type ChipMatch = { element: ElementName?, family: ChipFamily? }

-- A form's faces in the emotion window by its navi's emotion, keyed by its game's emotion names (its rules' `emotion`
-- order's; the engine holds a set to them as the content loads): an emotion without one shows the game's default
-- emotion's (the order's last case's: EXE6's and EXE5's `normal`), which a set has.
export type FaceSet = { [Emotion]: Mugshot }
export type MugshotFaces = {
    -- A second set, shown where the side's rules ask (`battle.set_face_variant`): while the navi's rules' B charge is
    -- armed (EXE5's soul's Chaos Unison look), or in the base form (EXE5's Hub Style).
    variant: FaceSet?,
    [Emotion]: Mugshot,
}

-- A navi (§3.2): MegaMan or a link navi. (`NaviDef`, since `Navi` is a
-- side's stats.)
export type NaviDef = {
    -- Its id, its key in the game's root (`chips.cannon`): the loader's to
    -- read, never written in the table.
    id: string?,
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
    -- The version of its game a link navi belongs to (EXE6's "gregar" or
    -- "falzar"), by the name the game's pack keeps a version's pictures
    -- under; none: every version's.
    version: string?,
    -- Extra height of its image as it merges with MegaMan in a Cross.
    merge_height: number?,
    -- The no-running message (L on the custom screen in a netbattle).
    run_message: RunMessage?,
    -- A link navi's own chip, offered once a round.
    own_chip: { chip: Chip, code: string }?,
    -- The offer draws once from the console's RNG (EXE5's, which picks
    -- between two entries of its table, the same chip).
    own_chip_draws: boolean?,
    -- Its damage bonus on its family's damaging chips: how much, a function
    -- of its side, which the round's setup asks once (EXE6's by the navi's
    -- level: rules/by_level).
    chip_bonus: { family: ChipFamily, dimming_chips: boolean?, damage: (side: number) -> number? }?,
    -- MegaMan's bonus on a family's damaging chips used standing on a type
    -- of panel, in a form with no `chip_bonus` (EXE5's: Aqua chips on sea);
    -- the use turns the panel Normal.
    panel_bonus: { panel: PanelType, family: ChipFamily, damage: number }?,
    -- The chips it charges with A: a family's, never its own chip.
    -- `damaging` is always stated: only the damaging chips that aren't
    -- dimming chips (false: any). `when`: only where this function of its
    -- side, which the round's setup asks once, says so (none: always).
    -- `plain`: only the chips that are neither dimming nor dark chips.
    -- (EXE6's link navis: their family's damaging chips from a level,
    -- `byte_8021369`; EXE5's team navis: their family's plain chips.)
    charged_chips: { family: ChipFamily, damaging: boolean, when: ((side: number) -> boolean)?, plain: boolean? }?,
    -- What a chip it charged gains (damage 0xFF: its damage again), where
    -- its form states no `charged_bonus`: a form's comes first.
    charged_bonus: { damage: number?, paralyzes: boolean? }?,
    -- The chips a charge doubles (`sub_8012AFA`).
    charge_doubles: ChipMatch?,
    -- Its A charge builds up the next Fire chip's damage, up to a limit: a
    -- function of its side, which the round's setup asks once (nil: no
    -- charge; EXE6's ChargeMan's by his level, `byte_802136D`).
    fire_charge: ((side: number) -> number?)?,
    -- "status_immune": a hit's status doesn't take.
    traits: { "status_immune" }?,
    -- What lifts its charge glow: while its animation is one of `anims` the
    -- glow shows `pixels` above its attach point 0 (EXE5's GyroMan's in the
    -- air).
    charge_glow_lift: { anims: { number }, pixels: number }?,
    -- What its sprite's palettes go by, for a navi that doesn't change form
    -- (1 when none is given): its Full Synchro palette is 4 of them, its
    -- can't-charge palette 1. Presentation only.
    palette_step: number?,
    -- Its own hooks, the original's tables by AI index. `tick`: its part
    -- of the per-tick pipeline, after its action, for a navi that doesn't
    -- change form (one that does runs its form's `tick`): EXE5's
    -- GyroMan's watches the panel under him. `idle`: its own idle, before
    -- the common one, which doesn't run on a tick it starts an action
    -- (`Navi:set_attack`): GyroMan's take-off and landing. `post_init`: as
    -- a player's init ends and after a navi switch: EXE6's DustMan's
    -- objects of battle mode 9, EXE5's ToadMan's dive.
    tick: ((navi: Object) -> ())?,
    idle: ((navi: Object) -> ())?,
    post_init: ((navi: Object) -> ())?,
    -- Its forms (MegaMan's): where the original asks whether a navi is
    -- MegaMan, the rules ask whether it has them. The table is its game's,
    -- sets by name, whose `form_list` the engine reads (a frontend numbers
    -- a set's pictures by it): EXE6's by game (`gregar`, `falzar`: the
    -- Crosses by their number on the custom screen, and its Beast Out and
    -- Beast Over), EXE5's `souls` (Soul Unison's, which the custom screen's
    -- soul button offers).
    forms: { [string]: any }?,
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
    switch_hp: { number }?,
    -- What the save's reload gives it by its link navi level (EXE6's
    -- `reloadCurNaviBaseStats`): its base HP by the story's progress, and
    -- what each level adds over its fresh stats. Tools fill a side's stats
    -- from it; no battle reads it.
    levels: NaviLevels?,
    -- What the story gives it by its level (EXE5's team navis): its HP by
    -- the story's progress, from 0, and its last level; a level below the
    -- last is the progress, and at the last the story is taken as done (the
    -- last HP). Tools fill a side's stats from it; no battle reads it.
    story: { hp: { number }, max_level: number }?,
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
export type FormWeapons = {
    buster: Weapon?,
    charge_shot: Weapon?,
    back_special: Weapon?,
    a_charge: Weapon?,
    alt_a_charge: Weapon?,
    mode9_a: Weapon?,
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
    | "controller_restart"
-- What the rules ask of particular forms:
--   status_immune: a hit's status doesn't take (TomahawkCross);
--   erases: its damaging Null chips delete what has a 4 in its HP
--     (EraseCross);
--   special_holds_buster: its held buster doesn't fire while its B+Back
--     special is asked for (TenguCross and DustCross in Beast Out);
--   stands_on_magnet: a magnet panel doesn't slide the navi (EXE5's MagnetSoul);
--   controlled: the side's rules' `controller` decides its idle (EXE6's
--     Beast Over: the berserk), the player's buttons don't reach it, and a
--     full gauge opens the custom screen;
--   shows_target_marker: the navi's target marker shows (EXE6's Beast
--     forms);
--   afterimages_stay: an afterimage of the navi lasts while it stays in
--     such a form, not until its attack ends (EXE6's Beast forms);
--   alt_charge_time: with a Null chip next, its A charge's time is the
--     alternative A-charge weapon's (EXE6's Beast forms);
--   doubles_null: its damaging Null chips deal double, outside battle mode
--     1 (Beast Over);
--   mood_palette: its palette, and its buster arm's, follow the mood
--     (Gregar and Falzar Beast).
export type FormTrait =
    "status_immune"
    | "erases"
    | "special_holds_buster"
    | "stands_on_magnet"
    | "controlled"
    | "shows_target_marker"
    | "afterimages_stay"
    | "alt_charge_time"
    | "doubles_null"
    | "mood_palette"
-- One of MegaMan's forms (§3.2).
export type FormDef = {
    -- Its id, its key in the game's root (`chips.cannon`): the loader's to
    -- read, never written in the table.
    id: string?,
    sprite: SpriteAsset,
    element: ElementName,
    weakness: { string }?,
    buster_bonus: number,
    -- Its game's base form: what the game's navis are in before they change
    -- form (one a game).
    base: boolean?,
    -- The action that changes a navi into it (a game's form change: EXE6's
    -- forms', rules/forms), which the pause handler runs at a turn's start.
    -- Every form but the base form names one.
    change: Action?,
    -- What a weakness hit drops it to (none: it stays).
    breaks_to: FormDef?,
    -- MegaMan's palette in a Cross.
    palette: number?,
    -- Its face in the emotion window, or its faces by its navi's emotion.
    mugshot: (Mugshot | MugshotFaces)?,
    -- The version of its game it belongs to (EXE6's "gregar" or "falzar":
    -- a version's Crosses and Beast), by the name the game's pack keeps a
    -- version's pictures under.
    version: string?,
    weapons: FormWeapons?,
    identity: Identity?,
    -- The damage it adds to a family's damaging chips (`uncharged`: only
    -- on a use that isn't charged, with the A charge not full), and to
    -- damaging Null chips when that doesn't apply (Beast Out's, outside
    -- battle mode 1).
    chip_bonus: { family: ChipFamily, damage: number, dimming_chips: boolean?, uncharged: boolean? }?,
    null_bonus: number?,
    -- The chips it charges with A, a family's by each rule. `damaging` is
    -- always stated: only the damaging chips that aren't dimming chips
    -- (false: any of the family). With `element_swords` the chips with the
    -- `element_sword` trait too; `plain`: only those that are neither
    -- dimming nor dark chips (EXE5's souls).
    charged_chips: { { family: ChipFamily, damaging: boolean, element_swords: boolean?, plain: boolean? } }?,
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
    ability_refresh: { FormEffect }?,
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
    -- The status reset spends it (EXE5's GyroSoul).
    priming: { by: ChipMatch, sound: Sound, doubles: { ChipMatch } }?,
    -- The panel type it doubles on, and the damaging chips (not dimming
    -- chips) that deal double while it stands on one, which the use turns
    -- normal (EXE5's TomahawkSoul's Wood chips on grass).
    panel_doubles: { panel: PanelType, chips: ChipMatch }?,
    -- The ticks a damaging chip (not a dimming chip) used with the panel
    -- ahead not its side's keeps it invulnerable (EXE5's KnightSoul).
    front_guard: number?,
    -- The B presses that ask for the forced charged shot (the role
    -- `actions.forced_charged_shot`), each within `window` ticks of the
    -- last (EXE4's GutsSoul: six within 10).
    rapid_presses: { presses: number, window: number }?,
    -- The action its charged chips start in place of their own (EXE5's
    -- NapalmSoul's).
    charged_action: Action?,
    -- What else the status reset does in it, after its `status_reset`
    -- (EXE5's souls' own routines: SearchSoul's reveal of invisible
    -- navis, TomahawkSoul's grass).
    reset: ((navi: Object) -> ())?,
    -- Its put-on and take-off routines (`sub_8011268`, `sub_8011384` by the
    -- form; EXE5's tables by soul), in place of the defaults
    -- (`put_on_form_parts`, `take_off_form_parts`): EXE5's base form's
    -- shade in Hub Style (0x0800EE1C), NumberSoul's layer. A base form's
    -- `put_on` also runs as a player's init ends, in place of its navi's
    -- init hook (`sub_8010DD0`: EXE5's MegaMan's record names the same
    -- routine).
    put_on: ((navi: Object) -> ())?,
    take_off: ((navi: Object) -> ())?,
    -- Its own part of the per-form tick (EXE5's MegaMan's, 0x080F04CE:
    -- GyroSoul's propeller by the priming), each tick.
    tick: ((navi: Object) -> ())?,
    -- What a chip's use from idle does besides in it, as the hand moves on past the chip (`navi.attack_chip`): EXE4's
    -- RollSoul's heal (0x080EED04).
    chip_used: ((navi: Object) -> ())?,
    traits: { FormTrait }?,
    -- The buster arm the navi raises in it (lib/buster): its animation,
    -- and what it adds to the arm's palette.
    buster_arm: { anim: number, palette_add: number? }?,
    -- Its game's rules' fields (their `extends.form`: EXE6's `kind`,
    -- `game`, `cross_of`, `beast`, ...; content/exe6/types.d.luau types them).
    [string]: any,
}
-- Something a stage places when the round starts: a navi of `side`, or a
-- field object by its kind (which needs a `place`), with one of the kind's
-- variants, its side or its HP where the kind takes one.
export type StageActor = {
    place: "navi" | Kind,
    side: number?,
    x: number,
    y: number,
    variant: any?,
    hp: number?,
}
-- A stage (§3.7): its panels, three rows of six panel types (`"normal
-- normal cracked ..."`), which columns are whose, its music outside link
-- battles (none: no music starts), and what it places, in order.
export type StageDef = {
    -- Its id, its key in the game's root (`chips.cannon`): the loader's to
    -- read, never written in the table.
    id: string?,
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
    id: string?,
    requests: number,
    duration: number,
    timer: string | { other: number },
    cancels_flinch: boolean?,
    survives_counter: boolean?,
}
export type Status = StatusDef
-- A problem of a side's setup (the rules' `validate`): a sentence, or with the setup field it is of and the entry
-- of a list field (from 1), which a tool shows it at.
export type SetupProblem = string | { text: string, field: string?, entry: number? }

-- A folder being checked (`battle.checked_folder`).
export type CheckedFolder = {
    side: number,
    chips: { { chip: Chip, code: string } },
    regular: number?,
    tags: { number }?,
    complete: boolean,
}

-- What an object is taken for (§3.2): the original's NameID record. A
-- navi's and a form's is nested in its definition (`identity =
-- define.identity { ... }`); a field object's is its kind's.
--   player: a player's navi (the navi that changes form in any form, a
--     link navi);
--   field_object: a rock, a statue, a mine;
--   virus, navi: what no netbattle object is (an object with no identity
--     is a virus).
export type IdentityClass = "player" | "field_object" | "virus" | "navi"
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
    -- EXE5's Poltergeist takes it (`throwable = false`: EXE5's NameIDs 0xDA,
    -- 0xD3, 0xD2, 0xE5, 0xE4 and 0xE7).
    throwable: boolean?,
    -- What its object wears (its actor record's init hook, `off_8010E0C`):
    -- a body overlay (and a second), an overlay worn only while standing,
    -- a beast's head (its palette; none: it follows the side's mood), or
    -- what a routine of its own puts on.
    parts: Parts?,
    -- Which of its object's hooks touch what the object wears: its death
    -- takes it off, an animation change, a flinch or a drag restarts it.
    -- By default the death hook takes its own parts off and an animation
    -- change restarts them (not an overlay worn only while standing).
    -- "flinch_checked" is a flinch hook that tests for what the navi wears
    -- first (EXE5's team navis'). "lets_go": as a flinch, a drag or a
    -- paralysis starts, what the navi wears is let go where an attack held
    -- it shown or hidden (one routine in the three tables: EXE5's
    -- KnightMan's ball and chain).
    overlay_hooks: { "death" | "refresh" | "flinch" | "flinch_checked" | "drag" | "lets_go" }?,
    -- The Full Synchro aura's animation around it (a player's has one).
    aura_anim: number?,
    -- The anger tick passes over it (EXE5's actor records of AI index 23:
    -- ShadowMan's SplitUp shadow's, which no navi a player operates has).
    never_angers: boolean?,
    -- One of the navi that changes form's own (its base form's or a form's):
    -- a navi image wears its own look.
    changes_form: boolean?,
    -- A weakness hit breaks the form (where the rules' `form_break` breaks
    -- marked forms: EXE6's Crosses, Beast Out and the Crosses in it).
    breaks_on_weakness: boolean?,
    -- A win over it is quiet: no music or banner, and the end fades to
    -- white (the Cybeasts).
    quiet_win: boolean?,
    -- The NaviCust bug 0xF6 doesn't blind it (the Cybeasts; the rules read
    -- it: @exelib/navicust/bugs).
    bug_blind_immune: boolean?,
    -- The animation in which the target marker over it isn't raised to its
    -- attach point (Gregar's 0x4F).
    marker_flat_anim: number?,
    -- Where the HUD stacks the chip icons over it ({ x, y } from its place on
    -- the screen, y down), where the rules' `effects.chip_icons` is
    -- "navi_offset" (EXE4's table by navi number).
    chip_icons_at: { number }?,
    -- Where its status visual sits from its position (x toward the enemy
    -- side, z up), where the rules' `effects.status_visual.place` is
    -- `status_mark` (EXE4's actor record's +6 and +7).
    status_mark: { number }?,
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
    -- A routine of its own, in place of the others: its record's init hook
    -- where what it wears is an object of the content's own kind, which
    -- the routine keeps in the wearer's second related slot (`related2`)
    -- itself. It runs wherever the record's init hook does: for a player,
    -- for its image, and for an afterimage of it where the game's
    -- afterimages wear what their owner does (the effects rule
    -- `afterimages_wear_overlays`). The death hook takes down what is in
    -- that slot, as it does the others (EXE5's NumberMan's face,
    -- NumberSoul's layer).
    own: ((wearer: Object) -> ())?,
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
export type CollisionTypeSpec = { id: string?, side0: number, side1: number, row_offset: number? }
export type CollisionType = CollisionTypeSpec

-- What the rules need from content by role (rules/roles.luau, once per
-- game: a plain table, the rules' `roles`): each role names the
-- definition that fills it.
export type RolesSpec = {
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
        ungrounded: Action?,
        glowless: Action?,
        wrapper: Action?,
        -- A player's deletion, where the rules have their own (EXE4's):
        -- the navi's deletion action runs it (its phase, timer and
        -- animation the object's); unfilled, the engine's.
        deletion: Action?,
    }?,
    kinds: {
        absorbed_obstacle: Kind?,
        falling_rock: Kind?,
        support: Kind?,
        anti_recovery: Kind?,
        -- What `Navi:spawn_mode9_objects` spawns (EXE6's DustMan's
        -- post-init hook in battle mode 9: the original's attack object
        -- #0xD2 and actor object #0x28).
        mode9_attack: Kind?,
        mode9_actor: Kind?,
        -- The charge glow a navi's charge brings, where the rules' effects
        -- say `charge_glow = "with_charge"` (EXE4's effect object 5): the
        -- engine spawns it as the navi's charge gets a level and its link
        -- (`charge_glow`) is empty, the navi its `related1`, links it and
        -- sets its state's `shown`.
        charge_glow: Kind?,
    }?,
    -- The chips the rules name: what a zeroed chip field reads, the chip a
    -- custom-screen button's pick stands as in the hand (EXE6's BeastOut)
    -- and its invalid selection's, and the chips the supports' telops name.
    chips: {
        zeroed: Chip?,
        button_chip: Chip?,
        invalid: Chip?,
        rush: Chip?,
        beat: Chip?,
        tango: Chip?,
    }?,
    -- The statuses the rules inflict themselves: what a damage word's
    -- paralysis bit carries (and, with EXE5's rule `effects.damage_word`,
    -- its confusion and blindness bits), what a counter hit lands, what an
    -- aqua hit gives a body on ice.
    statuses: {
        damage_word_paralysis: Status?,
        damage_word_confusion: Status?,
        damage_word_blindness: Status?,
        counter_paralysis: Status?,
    }?,
    -- The one-shot effects the rules show themselves: a navi deleted (and
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
    -- The hit sparks the rules show themselves: a new registration's until
    -- its owner names another, a blocked hit's, a thrown obstacle's
    -- landing's.
    sparks: {
        plain: Spark?,
        guard: Spark?,
        thrown_obstacle: Spark?,
    }?,
    -- The hit regions the rules register themselves: a registration's own
    -- panel, which every setup gives it.
    regions: {
        anchor: Region?,
    }?,
    -- The collision types the rules register themselves: a navi's body
    -- (floating or not) and what it reacts to, a thrown obstacle's landing
    -- and what it reaches.
    collision: {
        navi: CollisionType?,
        floating_navi: CollisionType?,
        navi_target: CollisionType?,
        thrown_obstacle: CollisionType?,
        thrown_obstacle_target: CollisionType?,
    }?,
    -- The sounds the rules play themselves (rules/roles.luau says what each
    -- is for).
    sounds: {
        panel_crack: Sound?,
        own_hit: Sound?,
        hit: Sound?,
        auto_battle_hit: Sound?,
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
        custom_shade: Sound?,
        custom_pick: Sound?,
        custom_ok: Sound?,
        custom_back: Sound?,
        custom_run_message: Sound?,
        custom_description: Sound?,
        custom_description_close: Sound?,
        custom_cancel: Sound?,
        custom_redeal: Sound?,
        custom_redeal_shuffle: Sound?,
        custom_scrap: Sound?,
        custom_scrap_done: Sound?,
        program_advance_part: Sound?,
        program_advance: Sound?,
    }?,
    -- The music the rules start themselves: a link battle's, the winner's
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
    -- hit marker, the target marker. (The Beast head and the
    -- idle overlay are the parts of the identities that wear them.)
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
        target_marker: SpriteAsset?,
    }?,
    -- The banners the rules show themselves: a round's start, a turn's, the
    -- final turns', a draw, the damage judge's, a telop's on either side,
    -- a Program Advance's and an empty selection's; a win's where it
    -- isn't the winner's navi's (the flow's `navi_win_banner`), and a
    -- win's and a loss's on the judge's ruling. A loss's otherwise is the
    -- navi's.
    banners: {
        round_start: Banner?,
        turn_start: Banner?,
        final_turn: Banner?,
        draw: Banner?,
        win: Banner?,
        win_judged: Banner?,
        lose_judged: Banner?,
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
        -- A navi no player controls is being deleted (EXE5's 0x08013DA0):
        -- what sparkles over it while it flashes, which its deletion's end
        -- ends.
        navi_deleted: ((navi: Object) -> Object?)?,
        -- `sub_801390C` has taken the navi's guard and untouchability down
        -- (its init, a form change): what its stats raise again (EXE4's
        -- All Guard, NaviStats +0x28: the guard, 0x0800D8E4).
        abilities_reset: ((navi: Object) -> ())?,
    }?,
}

-- The tag constructors (docs/design/content-model-v2.md §2.2): a table of
-- what only code reaches says what it is, and is returned; no id, no
-- registry, no effect. What a match names is a plain table in the game's
-- root, keyed by id (its init.luau). Only while content loads.
declare new: {
    weapon: (spec: WeaponSpec) -> Weapon,
    kind: (spec: KindSpec) -> Kind,
    action: (spec: ActionSpec) -> Action,
    effect: (spec: EffectSpec) -> Effect,
    spark: (spec: EffectSpec) -> Spark,
    region: (spec: RegionSpec) -> Region,
    collision: (spec: CollisionTypeSpec) -> CollisionType,
    status: (spec: StatusDef) -> StatusDef,
    identity: (spec: IdentitySpec) -> Identity,
    -- Luau-only data with a type name, which a reference field
    -- ("record:<type>") can hold.
    record: <T>(record_type: string, spec: T) -> T,
}

-- What the rules' own calls reach (docs/design/rules-in-luau.md §4.5,
-- §5.3): only in a hook the framework calls (from modules under rules/,
-- which the content check holds to).
-- A side's custom screen, in the rules' custom hooks (§4.4): the
-- refusal sound; the shared machinery for the button under the cursor (the
-- picked chips scrapped, the chips not picked dealt again); whether the
-- last pick is a chip; the state of the button under the cursor.
declare custom: {
    -- The side's battle folder as its screen has it: its 30 places in order,
    -- false where a used chip left one empty until the screen deals; two of
    -- its places swapped (1 to 30); how many chips the screen deals.
    folder: (side: number) -> { Chip | false },
    swap_folder: (side: number, a: number, b: number) -> (),
    -- As the screen deals (the rules' `deal`): a chip from outside the folder
    -- the screen shows in a slot (0 to 11) once laid out, in a code (a letter,
    -- or nil: the code outside the alphabet, EXE4's dark chips').
    offer: (side: number, slot: number, chip: Chip, code: string?) -> (),
    hand_size: (side: number) -> number,
    refuse: (side: number) -> (),
    sacrifice: (side: number) -> (),
    redeal: (side: number) -> (),
    last_pick_is_chip: (side: number) -> boolean,
    cursor_state: (side: number) -> "selectable" | "unavailable" | "selected",
    -- For the rules' buttons and windows: the slot under the cursor picked;
    -- a screen sound ("pick", "refused", "back", "cancel", "cursor",
    -- "description", "program_advance_part", "program_advance"), and a sound
    -- of the rules' own in the screen's order (`play_sound`); the last
    -- pick's column cell; one of the rules'
    -- windows opened (`ticks`: the ticks it has had already), and its ticks
    -- so far; this console's camera shake; the window's frame counter; the
    -- emblem's spin; a screen fade ("beast_out", "beast_out_back",
    -- "end_to_white", "intro_from_white", "soul_flash", "soul_flash_back");
    -- the emotion window's face; the
    -- last pick moved first (its column icon `icon`, else its chip's); one
    -- of the rules' buttons' state; the availability redone; the emblem
    -- drawn; the form the rules' pick holds for the turn's start, and
    -- whether another button's or window's holds it; whether the picks are full;
    -- whether one of the rules' buttons is picked; what the screen reads
    -- of its player that isn't the rules' setup (the emotion it reads, a
    -- random battle).
    -- The form takes, beside it, the transform record's turns and Chaos flag
    -- (EXE5's Soul Unison). The last pick, if it is a chip (dealt, or a link
    -- navi's own), as the screen checked it; the rules' button in the last
    -- pick's place, first in the selection (B puts the chip back; at OK the
    -- chip leaves the folder where the button stands): false when the last
    -- pick isn't a chip; whether the screen's fade is still running.
    -- A button attached to the last pick (EXE5's capsules): the last pick, a
    -- chip with no button attached yet (`last_pick`'s `attached`), carries
    -- `modifiers` into the hand (its modifier bits: 0x02, 0x04, 0x08 and
    -- 0x20 the damage word's 0x4000, 0x2000, 0x1000 and 0x0800, 0x10 a tenth
    -- of its user's HP healed at the use; the last three where the game's
    -- chip-use rules count them, `mixed_modifiers`), and the button is
    -- picked until B takes the chip back. A button holding the last pick
    -- (EXE5's Arm Change): the last pick, a chip dealt from the folder,
    -- leaves the picks for the button, which is picked; B, with the picks as
    -- they were then, puts it back; at OK the chip leaves the folder. The
    -- chip a button holds, and its icon in the column cell it left, shown or
    -- not.
    pick: (side: number) -> (),
    play: (side: number, sound: string) -> (),
    play_sound: (side: number, sound: Sound) -> (),
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
    -- `by`: the rules' button or window whose pick holds the form.
    set_form: (side: number, by: string, form: FormDef?, turns: number?, alternate: boolean?) -> (),
    last_pick: (side: number) -> { slot: number, chip: Chip, regular: boolean, navi_chip: boolean, attached: boolean }?,
    trade_last_pick: (side: number, button: string) -> boolean,
    attach_to_last_pick: (side: number, button: string, modifiers: number) -> boolean,
    hold_last_pick: (side: number, button: string) -> boolean,
    held_pick: (side: number, button: string) -> Chip?,
    set_held_icon: (side: number, button: string, shown: boolean) -> (),
    fading: (side: number) -> boolean,
    -- Whether a pick of another button or window than `by` holds the form.
    form_taken: (side: number, by: string) -> boolean,
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
    -- chip's frame alone, the icon of the chip a button holds over that
    -- button (`hold_last_pick`), the Cross window's cursor, the chip window
    -- showing the slot under the cursor, and the window's Cross tab.
    draw_window: (side: number) -> (),
    draw_regular: (side: number) -> (),
    draw_held: (side: number) -> (),
    draw_form_list_cursor: (side: number) -> (),
    show_chip_window: (side: number) -> (),
    set_form_list_tab: (side: number, on: boolean) -> (),
    -- R in the window up: the form's description (three lines without
    -- one), back to the window when it closes.
    describe: (side: number, form: FormDef?) -> (),
    -- The rules' buttons' states asked again.
    refresh_buttons: (side: number) -> (),
    player: (side: number) -> {
        emotion: Emotion,
        random_battle: boolean,
    },
}

-- The game's rules' own calls' reach: only a game's modules have it (a
-- support pack has no game context, docs/design/content-model-v2.md §4.0).
declare rules: {
    -- The rules' state of the side the call is for (cast it to its declared
    -- type: `rules.state() :: State`).
    state: () -> any,
    -- What the player of that side brought (their setup of the rules):
    -- read-only.
    setup: () -> any,
    -- That side (0 or 1).
    side: () -> number,
    -- The rules' state of side `side`, for a game's rules alone: its API
    -- module (EXE6's rules/api.luau) reaches what the rules keep for its
    -- chips, which never call this themselves (the content check holds to
    -- it, and the call refuses a module outside rules/).
    state_of: (side: number) -> any,
    -- Side `side`'s player's setup of the rules, read-only, likewise a
    -- game's rules' alone: its API module reads the save's `level` there.
    setup_of: (side: number) -> any,
}
