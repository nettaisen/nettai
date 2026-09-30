-- The battle engine's content API, as Luau content sees it. A definitions
-- file for editors and the type checker (luau-lsp:
-- `--definitions=content/bn6/core.d.luau`); the engine implements it in
-- crates/bn6-luau/src/bind.rs over the Rust `CoreApi` trait.
--
-- Numbers: every engine value is an integer. Scripts compute with plain
-- numbers (exact up to 2^53); when a number goes into the engine it must be
-- an integer (a fraction is an error) and a field store wraps it to the
-- field's width, as the game's byte and halfword stores do. So a u16 timer
-- set to -1 reads back 0xFFFF. Positions are `Vec3` values: three i32 in
-- 16.16 fixed point, wrapping on + and -. Use `//` (floor division) or the
-- `int` library, never `/`.

export type Pool = "actor" | "attack" | "effect"
export type Lifecycle = "init" | "update" | "destroy"
export type Shadow = "hidden" | "ground" | "with_sprite"
export type StatusFlag =
    "guard"
    | "invisible"
    | "invulnerable"
    | "moving"
    | "flashing"
    | "flinching"
    | "paralyzed"
    | "sliding"
    | "frozen"
    | "super_armor"
    | "move_complete"
    | "using_action"
    | "bubbled"

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
    -- Load a sprite (resets the animation and the look).
    function load(self, category: number, index: number): ()
    -- Start an animation from its first frame.
    function set_animation(self, anim: number): ()
    -- Advance the animation one tick.
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
    -- What the last resolution hit (read-only).
    hit_flags: number
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
    lifecycle: Lifecycle
    action: number
    phase: number
    phase_init: number
    panel_x: number
    panel_y: number
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
    damage: number
    stamina: number
    name_id: number
    pos: Vec3
    vel: Vec3
    related1: Object?
    related2: Object?
    visible: boolean
    run_while_paused: boolean
    run_in_time_stop: boolean
    no_sprite_update: boolean
    sprite: Sprite
    collision: Collision
    -- The kind's declared state (cast it to the kind's State type).
    state: any

    -- Navis only: the overlay slot and the attack in progress.
    overlay: Object?
    -- The running action's phase and whether its entry ran.
    step: number
    step_init: number
    -- The attack's variant (a chip's level), and its chip (read-only).
    variant: number
    chip: number

    -- Spawn parameter n (1 to 4: the game's Param1..Param4).
    function param(self, n: number): number
    function set_lifecycle(self, state: Lifecycle): ()
    -- Free the slot now.
    function free(self): ()
    -- Release panel reservations and collision, then free.
    function destroy(self): ()
    -- Request an animation, restarting it even if it is current.
    function set_animation(self, anim: number): ()
    -- Load a requested animation and step the sprite (unless paused/stopped).
    function update_sprite(self): ()
    -- The sprite's attach point n, in pixels facing the object's way.
    function attach_point(self, n: number): (number, number)
    function status(self, flag: StatusFlag): boolean
    function set_status(self, flag: StatusFlag, on: boolean): ()
    function open_counter_window(self): ()
    function check_reactive_abort(self): ()
    -- End the attack: back to the idle action with animation 0.
    function exit_attack(self): ()
    function create_collision(self): boolean
    function setup_collision(self, self_type: number, target_type: number, hit_mod: number): ()
    function hit_spark(self): ()
end

-- A side's navi stats (read-only).
declare class Navi
    -- Fighting in the sun.
    sun: boolean
    form: number
    navi: number
end

declare battle: {
    time_stop: () -> boolean,
    paused: () -> boolean,
    play_sound: (id: number) -> (),
    navi: (side: number) -> Navi,
    -- Spawn into the pool's lowest free slot; it runs later this tick,
    -- right after the spawner. nil if the pool is full.
    spawn: (pool: Pool, index: number, pos: Vec3?, params: { number }?) -> Object?,
    panel_valid: (x: number, y: number) -> boolean,
    -- A panel's center, 16.16.
    panel_center: (x: number, y: number) -> (number, number),
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
