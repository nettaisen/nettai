--!strict
-- The boomerang (attack object #0x32, `sub_80CA2A8`): Boomer, HiBoomer and
-- M-Boomer's (instant chip effect 1, `sub_80CA4F6`) and TomahawkCross
-- Beast's tomahawks (action 0x4E, weapons/1b-tomahawk-throw). It starts on
-- the edge of a row and flies along it at its kind's speed
-- (data.objects.boomerangs[Param1]); past the column Param2 picks it flies
-- along that column to the other edge row, and back along that row until
-- it leaves the field. It hits what it passes, once per panel, and the kinds
-- that do turn the other side's panels it crosses to grass. Param3 is its
-- leg (0 out, 1 across, 2 back); Param4 makes the hit stronger.

local boomerang = {}

export type State = {
    -- The panel (x | y << 8) where it last hit something: its region
    -- comes back once it is on another (ExtraVars).
    hit_panel: number,
}

boomerang.state = { hit_panel = "u16" }

local SPRITE, TOMAHAWK_SPRITE = "10-07", "10-57"
-- Param1: the tomahawk, which looks different.
local TOMAHAWK = 4
-- Param2: it never turns.
local NO_TURN = 3

-- Where a leg along a row ends, by Param2 (`byte_80CA4B4` flying left,
-- `byte_80CA4C4` flying right), 16.16.
local TURN_LEFT = { [0] = -0x64_0000, -0x3C_0000, -0x14_0000 }
local TURN_RIGHT = { [0] = 0x64_0000, 0x3C_0000, 0x14_0000 }
-- Where the leg along the column ends: row 1's and row 3's centers.
local TOP, BOTTOM = 0x4_0000, 0x34_0000

-- Its collision types, and its hit's.
local SELF_TYPE, TARGET_TYPE = 4, 5
local HIT_SPARK = 4
-- A hit registered by something (`dword_80CA540`).
local HIT = 0xFF800000

-- The panel flags `sub_80CA510` reads: a panel that can grow grass, and
-- the side 1 bit.
local CAN_GROW, SIDE_1 = 0x10, 0x20

-- Its kind (`byte_80CA26C[Param1]`).
local function kind(me: Object): BoomerangKind
    local k = data.objects.boomerangs[me:param(1)]
    if k == nil then
        error(string.format("boomerang kind %d reads past byte_80CA26C", me:param(1)))
    end
    return k
end

-- `sub_80CA4D8`: a boomerang of `owner`'s on (x, y), with the spawn
-- parameters and damage word it is given. (It spawns at the registers'
-- values, which its init replaces but for Z.)
function boomerang.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    params: { number },
    damage: number
): Object?
    local o = battle.spawn_kind("boomerang", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- `sub_80CA4F6`, instant chip effect 1 (Boomer and the like): from the
-- user's back column, row 3, 16 pixels up.
function boomerang.instant_chip(user: Object, spec: InstantChipSpec)
    local x = bit32.bxor(user.alliance, user.flip) * 5 + 1
    boomerang.spawn(user, x, 3, spec.element, 0x10_0000, spec.params, spec.damage)
end

-- `sub_80CA510`: a grass boomerang turns the other side's panel it is on
-- to grass, if the panel can grow it.
local function grow_grass(me: Object)
    if not kind(me).grass then
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
local function init(me: Object)
    local tomahawk = me:param(1) == TOMAHAWK
    local sprite = me.sprite
    sprite:load(if tomahawk then TOMAHAWK_SPRITE else SPRITE)
    sprite.shadow = if tomahawk then "with_sprite" else "ground"
    me.visible = true
    local anim = if tomahawk and me.panel_y ~= 1 then 1 else 0
    me.anim, me.anim_loaded = anim, anim
    sprite:set_animation(anim)
    local k = kind(me)
    local vel = me.vel
    local vy = vel.y
    if me:param(2) ~= NO_TURN then
        vy = if me.panel_y == 3 then -k.turn_speed else k.turn_speed
    end
    me.vel = Vec3.new(k.speed, vy, vel.z)
    me.timer = 0
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, if me:param(4) ~= 0 then 3 else 1)
    local c = me.collision
    c.region = 1
    c.hit_effect = HIT_SPARK
    c:present()
    grow_grass(me)
    battle.play_sound(0xB7)
    me:set_lifecycle("update")
end

-- Turn onto the next leg, back on the center of the panel it was on.
local function turn(me: Object, leg: number): boolean
    me:set_param(3, leg)
    me:set_coordinates_from_panel()
    return true
end

-- `sub_80CA408`: one tick of flight.
local function fly(me: Object)
    me.timer += 1
    local leg = me:param(3)
    local p = me.pos
    local turned = false
    if leg == 2 then
        -- Back along the row.
        local back = -me.facing
        me.pos = Vec3.new(p.x + me.vel.x * back, p.y, p.z)
    elseif leg == 1 then
        local dy = me.vel.y
        local y = int.i32(p.y + dy)
        me.pos = Vec3.new(p.x, y, p.z)
        if dy < 0 then
            if y <= TOP then
                turned = turn(me, 2)
            end
        elseif y >= BOTTOM then
            turned = turn(me, 2)
        end
    else
        local dx = me.vel.x * me.facing
        local x = int.i32(p.x + dx)
        me.pos = Vec3.new(x, p.y, p.z)
        if me.vel.y ~= 0 then
            local column = me:param(2)
            local left, right = TURN_LEFT[column], TURN_RIGHT[column]
            if left == nil then
                error(string.format("boomerang column %d reads past byte_80CA4B4", column))
            end
            if dx < 0 then
                if x <= left then
                    turned = turn(me, 1)
                end
            elseif x >= right then
                turned = turn(me, 1)
            end
        end
    end
    if not turned then
        me:set_panel_from_coordinates()
    end
    me:update_collision_panels()
    if me.collision.direction ~= 0 then
        grow_grass(me)
    end
end

-- The panel as the game's halfword (x | y << 8).
local function panel_word(me: Object): number
    return me.panel_x + bit32.lshift(me.panel_y, 8)
end

-- `sub_80CA3A0`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    local s = me.state :: State
    if bit32.btest(c.hit_flags, HIT) then
        c.region = 0
        s.hit_panel = panel_word(me)
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("boomerang action %#x reads past its table", a))
    end
    fly(me)
    if c.region == 0 and panel_word(me) ~= s.hit_panel then
        c.region = 1
    end
    if field.valid(me.panel_x, me.panel_y) then
        c:present()
        me:highlight_collision_panels()
        return
    end
    c.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80CA2A8`.
function boomerang.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite()
end

return boomerang
