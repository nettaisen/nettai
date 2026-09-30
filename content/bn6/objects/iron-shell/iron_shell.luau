--!strict
-- An iron shell (attack object #0x34, `sub_80CA6B8`): IronShl's and
-- ParaShl's. It slides from its panel to the opponent's back column (four
-- panels for ParaShl's), hitting what it meets; after a hit its region is
-- off until it reaches another panel. There it stops for 8 ticks (16 for
-- ParaShl's) with its region back on, its second parameter times; then an
-- IronShl flies up and away, a ParaShl turns and slides back. Its first
-- parameter picks the palette and speed. It bursts (effect 0x14) off
-- solid ground.
--
-- It hits with modifier 3 (knocking back) while between the side columns,
-- 1 (not) when thrown onto a side column, and from when it reaches the
-- opponent's back column, where it no longer pushes.

local object_setup = require("../../lib/object_setup")

export type State = {
    -- The panel (x | y << 8) it last hit on (ExtraVars+0).
    hit_panel: number,
    -- Its hit modifier dropped at the far column (ExtraVars+4).
    softened: boolean,
}

local iron_shell = {
    state = { hit_panel = "u32", softened = "bool" },
}

local SPRITE, ANIM = "04-1b", 3
-- Speed by the first parameter, and ParaShl's (`byte_80CA7A0`), 16.16.
local SPEEDS = { [0] = 0x6_0000, 0x7_0000, 0x8_0000 }
local PARA_SPEED = 2
-- A panel's width, 16.16.
local PANEL_WIDTH = 0x28_0000
-- The rise as it flies off, 16.16 per tick.
local FLY_UP = 0x4_0000
-- The collision types, and the hit modifiers.
local SELF_TYPE, TARGET_TYPE = 6, 5
local PUSHES, STILL = 3, 1
local SOLID = 0x10

-- The actions.
local SLIDE, BUMP, FLY = 0, 4, 8

-- `sub_80CA90E`: a shell for `owner` on (x, y), with the damage word
-- `damage` and the chip's `params`. Its position is the spawner's
-- registers (the panel Y, the element and 0) until its init places it.
function iron_shell.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    damage: number,
    params: { number }
): Object?
    local o = battle.spawn_kind("iron-shell", Vec3.new(y, element, 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- The panel as the game's halfword (x | y << 8).
local function panel_word(me: Object): number
    return me.panel_x + me.panel_y * 0x100
end

-- The opponent's back column.
local function far_column(me: Object): number
    return bit32.bxor(me.alliance, 1) * 5 + 1
end

-- `sub_80CA834`: slide; at the far column the push goes; stop when the
-- slide's time is up.
local function slide(me: Object)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local s = me.state :: State
    if not s.softened and me.panel_x == far_column(me) then
        s.softened = true
        me:reset_collision_types(SELF_TYPE, TARGET_TYPE, STILL)
    end
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me:set_coordinates_from_panel()
    me.timer = if me:param(3) ~= 0 then 0x10 else 8
    me:set_action(BUMP)
end

-- `sub_80CA88E`: stand, the region back on each 8 ticks, Param2 times;
-- then fly off (IronShl) or turn back (ParaShl).
local function bump(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.collision.region = 1
    local left = me:param(2) - 1
    me:set_param(2, left)
    if int.u8(left) ~= 0 then
        me.timer = 8
        return
    end
    if me:param(3) ~= 0 then
        local v = me.vel
        me.vel = Vec3.new(-v.x, v.y, v.z)
        me.flip = bit32.bxor(me.flip, 1)
        me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    else
        local v = me.vel
        me.vel = Vec3.new(v.x, v.y, FLY_UP)
    end
    me:reset_collision_types(SELF_TYPE, TARGET_TYPE, PUSHES)
    me:set_action(FLY)
end

-- `sub_80CA8E2`: fly on; gone off the field.
local function fly(me: Object)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), p.y, int.i32(p.z + v.z))
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision.region = 0
        me:set_lifecycle("destroy")
    end
end

-- `sub_80CA7AC`: resolve its hits; move; burst off solid ground; its
-- region comes back on a new panel.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        (me.state :: State).hit_panel = panel_word(me)
        c.region = 0
    end
    local a = me.action
    if a == SLIDE then
        slide(me)
    elseif a == BUMP then
        bump(me)
    elseif a == FLY then
        fly(me)
    else
        error(string.format("iron shell action %#x reads past off_80CA828", a))
    end
    if not bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) then
        object_setup.puff(me)
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    local highlight = true
    if c.region == 0 then
        if panel_word(me) == (me.state :: State).hit_panel then
            highlight = false
        else
            c.region = 1
        end
    end
    if highlight then
        field.highlight(me.panel_x, me.panel_y)
    end
    me:update_sprite()
    c:present()
end

-- `sub_80CA6D8`: on solid ground, a speed and a slide to the far column.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local x, y = me.panel_x, me.panel_y
    local para = me:param(3) ~= 0
    local ok = field.valid(x, y) and bit32.btest(field.flags(x, y), SOLID)
    if ok then
        -- It knocks back only thrown onto a middle column.
        local hit_mod = if para or x == 1 or x >= 6 then STILL else PUSHES
        ok = object_setup.sprite_and_collision(me, SPRITE, ANIM, true, SELF_TYPE, TARGET_TYPE, hit_mod)
    end
    if not ok then
        object_setup.puff(me)
        me:free()
        return
    end
    me.visible = true
    me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.sprite.palette = me:param(1)
    local kind = if me:param(3) == 1 then PARA_SPEED else me:param(1)
    local speed = SPEEDS[kind]
    if speed == nil then
        error(string.format("iron shell speed %d reads past byte_80CA7A0", kind))
    end
    local v = me.vel
    me.vel = Vec3.new(me.facing * speed, v.y, v.z)
    me.collision.hit_effect = 0x0A
    local panels = if para then 4 else math.abs(far_column(me) - x)
    me.timer = int.tdiv(PANEL_WIDTH * panels, speed) + 1
    me.collision:present()
    battle.play_sound(0x187)
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80CA6B8`.
function iron_shell.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return iron_shell
