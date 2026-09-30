--!strict
-- A tomahawk strike (attack object #0x6A, `sub_80D1A08`): after Param1
-- ticks, a hit on its panel (30 pixels up; Param2 and Param3 its collision
-- types, Param4 its hit spark) for 10 ticks, with a flash (effect 0x34),
-- and the panel breaks (cracks while something stands on it). It waits
-- while dimmed; a hit ends its region. [unverified: no lab scenario
-- reaches it; docs/engine/standard-chips.md]

local tomahawk_strike = {}

local HEIGHT = 0x30_0000
local FLASH = 0x34
local HIT_TICKS = 0x0F - 5
local WAIT, STRIKE = 0x0, 0x4

-- `sub_80D1B22`: a strike for `owner` on (x, y), with the damage word
-- `damage` and parameters { delay, self type, target type, spark }. It runs
-- while dimmed; its position (the registers: the panel Y, the element and
-- `z`) lasts until its init.
function tomahawk_strike.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number, z: number): Object?
    local o = battle.spawn_kind("tomahawk-strike", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_800E258`: the panel at a position.
local function panel_at(x: number, y: number): (number, number)
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28), int.tdiv(int.asr(y, 16) + 0x20, 0x18)
end

-- `sub_801A100`: after a hit on something (not a guard), its spark at the
-- center of the panel it stands over, 16 pixels up, jittered.
local function panel_spark(me: Object)
    local c = me.collision
    local f = c.hit_flags
    if not bit32.btest(f, 0x3F80_0000) or bit32.btest(f, 1) or c.hit_effect == 0xFF then
        return
    end
    local px, py = panel_at(me.pos.x, me.pos.y)
    local cx, cy = field.center(px, py)
    battle.spark(me, battle.jitter(0x0F, Vec3.new(cx, cy, 0x10_0000)), c.hit_effect)
end

-- `sub_80D1AB0`: Param1 ticks.
local function wait(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = me:param(1)
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    me.action = STRIKE
    me.phase = 0
    me.phase_init = 0
end

-- `sub_80D1AD2`: the hit, the flash and the panel; 10 ticks.
local function strike(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.collision.region = 1
        local cx, cy = field.center(me.panel_x, me.panel_y)
        battle.effect(Vec3.new(cx, cy, 0), FLASH, bit32.bxor(me.alliance, me.flip))
        field.break_panel(me.panel_x, me.panel_y)
        me.timer = HIT_TICKS
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80D1A64`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    panel_spark(me)
    if battle.over() then
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    if not battle.dimmed() then
        local a = me.action
        if a == WAIT then
            wait(me)
        elseif a == STRIKE then
            strike(me)
        else
            error(string.format("tomahawk strike action %#x reads past off_80D1AA8", a))
        end
    end
    c:present()
end

-- `sub_80D1A28`: over its panel, its region off until it strikes.
local function init(me: Object)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, HEIGHT)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(me:param(2), me:param(3), 3)
    me.collision.hit_effect = me:param(4)
    me.collision.region = 0
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D1A08`.
function tomahawk_strike.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return tomahawk_strike
