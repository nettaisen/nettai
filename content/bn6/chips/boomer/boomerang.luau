--!strict
-- The boomerang (the original's attack object #0x32, `sub_80CA2A8`):
-- Boomer, HiBoomer and M-Boomer's (./chips, instant chip effect 1,
-- `sub_80CA4F6`), which TomahawkCross Beast's tomahawks (action 0x4E,
-- navis/megaman/forms/tomahawkcross-beast/throw) use too, as does
-- BoomrArm's charged shot (chips/boomrarm/charge, through Boomer's
-- effect). It starts on the edge of a row and flies
-- along it at its variant's speed; past the column it turns at it flies
-- along that column to the other edge row, and back along that row until
-- it leaves the field. It hits what it passes, once per panel, and the
-- variants that do turn the other side's panels it crosses to grass. What
-- differs between its users is its variant, a record (`boomerang.variant
-- { ... }`, a row of `byte_80CA26C`).

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

local boomerang = {}

-- A boomerang's variant: 16.16 pixels a tick along a row and along the
-- column, whether it turns the other side's panels it crosses to grass,
-- and whether it is the tomahawk (its own sprite and shadow, and its
-- second animation off row 1); its sprite where a game's isn't BN6's
-- (BN5's Boomer).
export type Variant = { speed: number, turn_speed: number, grass: boolean, tomahawk: boolean?, sprite: SpriteAsset? }

-- How a boomerang is thrown: its variant; the column (0 to 2) where it
-- turns, or `turns = false` (the original's Param2 3: it flies straight
-- along its row); a stronger hit (hit modifier 3, else 1).
export type Throw = { variant: Variant, turns: boolean?, column: number?, strong: boolean? }

export type State = {
    variant: Variant,
    turns: boolean,
    column: number,
    strong: boolean,
    -- Its leg (Param3 in the original): 0 out along the row, 1 along the
    -- column, 2 back along the other row.
    leg: number,
    -- The panel (x | y << 8) where it last hit something: its region
    -- comes back once it is on another (ExtraVars).
    hit_panel: number,
}

local SPRITE, TOMAHAWK_SPRITE = asset.sprite("boomerang"), asset.sprite("boomerang-tomahawk")
local SOUND = asset.sound("boomerang")

-- Where a leg along a row ends, by the column it turns at (`byte_80CA4B4`
-- flying left, `byte_80CA4C4` flying right), 16.16.
local TURN_LEFT = { [0] = -0x64_0000, -0x3C_0000, -0x14_0000 }
local TURN_RIGHT = { [0] = 0x64_0000, 0x3C_0000, 0x14_0000 }
-- Where the leg along the column ends: row 1's and row 3's centers.
local TOP, BOTTOM = 0x4_0000, 0x34_0000

-- Its collision: an attack that hits navis; hit modifier 1, or 3 for a
-- strong one.
local IS, HITS = collision.attack, collision.hits_navis
local HIT_MOD, STRONG_HIT_MOD = 1, 3
-- A hit registered by something (`dword_80CA540`).
local HIT = 0xFF800000

-- The panel flags `sub_80CA510` reads: a panel that can grow grass, and
-- the side 1 bit.
local CAN_GROW, SIDE_1 = 0x10, 0x20

-- The legs.
local OUT, ACROSS, BACK = 0, 1, 2

-- TomahawkCross Beast's tomahawk (the original's row 4).
boomerang.tomahawk = define.record("boomerang-variant", {
    speed = 0x8_0000,
    turn_speed = 314573,
    grass = false,
    tomahawk = true,
} :: Variant)

-- A boomerang variant.
function boomerang.variant(v: Variant): Variant
    return define.record("boomerang-variant", v)
end

-- `sub_80CA510`: a grass boomerang turns the other side's panel it is on
-- to grass, if the panel can grow it.
local function grow_grass(me: Object, s: State)
    if not s.variant.grass then
        return
    end
    local x, y = me.panel_x, me.panel_y
    local f = field.flags(x, y)
    if not bit32.btest(f, CAN_GROW) then
        return
    end
    if bit32.band(f, SIDE_1) == me.alliance * SIDE_1 then
        return
    end
    field.set_type(x, y, "grass")
end

-- `sub_80CA2D4`.
local function init(me: Object, s: State)
    local v = s.variant
    local tomahawk = v.tomahawk == true
    local sprite = me.sprite
    sprite:load(v.sprite or (if tomahawk then TOMAHAWK_SPRITE else SPRITE))
    sprite.shadow = if tomahawk then "with_sprite" else "ground"
    me.visible = true
    local anim = if tomahawk and me.panel_y ~= 1 then 1 else 0
    me.anim, me.anim_loaded = anim, anim
    sprite:set_animation(anim)
    local vel = me.vel
    local vy = vel.y
    if s.turns then
        vy = if me.panel_y == 3 then -v.turn_speed else v.turn_speed
    end
    me.vel = Vec3.new(v.speed, vy, vel.z)
    me.timer = 0
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, if s.strong then STRONG_HIT_MOD else HIT_MOD)
    local c = me.collision
    c:set_region(regions.single)
    c:set_hit_effect(sparks.wood)
    c:present()
    grow_grass(me, s)
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
end

-- Turn onto the next leg, back on the center of the panel it was on.
local function turn(me: Object, s: State, leg: number): boolean
    s.leg = leg
    me:set_coordinates_from_panel()
    return true
end

-- `sub_80CA408`: one tick of flight.
local function fly(me: Object, s: State)
    me.timer += 1
    local p = me.pos
    local turned = false
    if s.leg == BACK then
        local back = -me.facing
        me.pos = Vec3.new(p.x + me.vel.x * back, p.y, p.z)
    elseif s.leg == ACROSS then
        local dy = me.vel.y
        local y = int.i32(p.y + dy)
        me.pos = Vec3.new(p.x, y, p.z)
        if dy < 0 then
            if y <= TOP then
                turned = turn(me, s, BACK)
            end
        elseif y >= BOTTOM then
            turned = turn(me, s, BACK)
        end
    else
        local dx = me.vel.x * me.facing
        local x = int.i32(p.x + dx)
        me.pos = Vec3.new(x, p.y, p.z)
        if me.vel.y ~= 0 then
            local left, right = TURN_LEFT[s.column], TURN_RIGHT[s.column]
            if left == nil then
                error(string.format("boomerang column %d reads past byte_80CA4B4", s.column))
            end
            if dx < 0 then
                if x <= left then
                    turned = turn(me, s, ACROSS)
                end
            elseif x >= right then
                turned = turn(me, s, ACROSS)
            end
        end
    end
    if not turned then
        me:set_panel_from_coordinates()
    end
    me:update_collision_panels()
    if me.collision.direction ~= 0 then
        grow_grass(me, s)
    end
end

-- The panel as the game's halfword (x | y << 8).
local function panel_word(me: Object): number
    return me.panel_x + bit32.lshift(me.panel_y, 8)
end

-- `sub_80CA3A0`.
local function tick(me: Object, s: State)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if bit32.btest(c.hit_flags, HIT) then
        c:set_region(nil)
        s.hit_panel = panel_word(me)
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("boomerang action %#x reads past its table", a))
    end
    fly(me, s)
    if c.region == nil and panel_word(me) ~= s.hit_panel then
        c:set_region(regions.single)
    end
    if field.valid(me.panel_x, me.panel_y) then
        c:present()
        me:highlight_collision_panels()
        return
    end
    c:set_region(nil)
    me:set_lifecycle("destroy")
end

boomerang.kind = define.kind {
    id = "boomer/boomerang",
    pool = "attack",
    state = {
        variant = "record:boomerang-variant",
        turns = "bool",
        column = "u8",
        strong = "bool",
        leg = "u8",
        hit_panel = "u16",
    },
    -- `sub_80CA2A8`.
    update = function(me: Object)
        local s = me.state :: State
        local l = me.lifecycle
        if l == "init" then
            init(me, s)
        elseif l == "update" then
            tick(me, s)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- `sub_80CA4D8`: a boomerang of `owner`'s on (x, y), thrown as `throw`
-- says, with the damage word `damage`. (It spawns at the registers'
-- values, which its init replaces but for Z.)
function boomerang.spawn(owner: Object, x: number, y: number, element: number, z: number, throw: Throw, damage: number): Object?
    local o = battle.spawn(boomerang.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        local s = o.state :: State
        s.variant = throw.variant
        s.turns = throw.turns ~= false
        s.column = throw.column or 0
        s.strong = throw.strong == true
    end
    return o
end

-- `sub_80CA4F6`, instant chip effect 1: a chip's `instant` hook that
-- throws a boomerang of `variant` from the user's back column, row 3, 16
-- pixels up, turning at the far column.
function boomerang.instant(variant: Variant): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local x = bit32.bxor(user.alliance, user.flip) * 5 + 1
        boomerang.spawn(user, x, 3, spec.element, 0x10_0000, { variant = variant }, spec.damage)
    end
end

return boomerang
