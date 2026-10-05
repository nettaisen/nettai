--!strict
-- A flame leaping off DeathPhoenix's fire (EXE5's effect object #0x71, 0x080E8ACC; EXE5's own): on the fire's panel,
-- a pixel up the field, 4 pixels up; from its next tick it leaps 40 pixels ahead over 16 ticks in an arc 20 pixels
-- high (slow on the way up, quick on the way down), and goes. It faces the fire's way, turned back for a far fire.
-- (Its action table, 0x080E8B3C, holds the leap alone: the 10-tick wait that would go before it, 0x080E8B40, is
-- unreachable.)

local trajectory = require("../../lib/trajectory")

local flame = {}

export type State = {
    -- The arc's angle (+4) and its step a tick (+6).
    angle: number,
    step: number,
}

local SPRITE = asset.sprite("phoenix-flame")
local ANIM = 2
-- Its height before it leaps (the spawn's Z, which its init keeps).
local Z = 0x4_0000
local LEAP = 0x10

-- 0x080E8AF0: on its panel, a pixel up the field, animation 2, no shadow.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y - 0x1_0000, p.z)
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim = ANIM
    sprite:set_animation(ANIM)
    sprite.shadow = "with_sprite"
    sprite.palette = 0
    me.visible = true
    me:set_lifecycle("update")
end

-- Action 0 (0x080E8B62): the leap: 2.5 pixels ahead a tick (its facing's way), the arc's angle 0x600 a tick for 8
-- ticks then 0xA00 (half a turn in all), its height 4 pixels plus 20 times the sine.
local function leap(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 1
        s.angle = 0
        me.timer = LEAP
        s.step = 0x800
        me.vel = Vec3.new(0x28000 * me.facing, 0, 0)
    end
    local p = me.pos
    local x = p.x + me.vel.x
    local step = s.step
    if me.timer > 8 then
        step -= bit32.rshift(step, 2)
    else
        step += bit32.rshift(step, 2)
    end
    local angle = s.angle + step
    s.angle = int.u16(angle)
    local z = trajectory.sine(bit32.rshift(angle, 8)) * 20 * 0x100 + Z
    me.pos = Vec3.new(x, p.y, z)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

flame.kind = define.kind {
    id = "dethphnx/flame",
    pool = "effect",
    state = { angle = "u16", step = "u16" },
    -- 0x080E8ACC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local a = me.action
            if a == 0 then
                leap(me)
            else
                error(string.format("DethPhnx's flame: action %#x reads past its table (0x080E8B3C)", a))
            end
        else
            me:free()
        end
        me:update_sprite()
    end,
}

-- 0x080E8BF4: a flame off `fire` (on its panel, of its side and facing; turned back when `far`), running while
-- dimmed.
function flame.spawn(fire: Object, far: boolean): Object?
    local o = battle.spawn(flame.kind, Vec3.new(0, 0, Z))
    if o then
        o.panel_x, o.panel_y = fire.panel_x, fire.panel_y
        o.alliance, o.flip = fire.alliance, fire.flip
        o.run_while_dimmed = true
        if far then
            o.flip = bit32.bxor(o.flip, 1)
        end
    end
    return o
end

return flame
