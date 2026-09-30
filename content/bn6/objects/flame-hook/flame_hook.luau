--!strict
-- FlmHook's hook (effect object #0x8C, `sub_80EA010`) and the chips'
-- effect (instant chip effect 14, `sub_80EA11C`): it swings two flames
-- (objects/flame-hook-fire), one from the column in front of the user
-- (Param1 0; else column 4, or 3 on side 1) and 20 ticks later one from
-- the column two ahead (else column 5, or 2 on side 1), then goes 10 ticks
-- later. Param1 and Param2 go to the flames as their kind and hit spark.
-- It has no sprite.

local flame_hook_fire = require("../flame-hook-fire/flame_hook_fire")

local flame_hook = {}

-- The rows the flames start from.
local FIRST_ROW, SECOND_ROW = 4, 0
local SWING_TICKS, END_TICKS = 0x14, 0xA

-- The phases.
local FIRST, SECOND, DONE = 0, 4, 8

-- `sub_80EA11C`, instant chip effect 14 (FlmHook1-3): a hook of the
-- user's on its panel. (It spawns at the registers' values. The game also
-- keeps the panel in the header's two spare bytes, which nothing reads, and
-- with Param1 set writes to the ROM through a stale register, which does
-- nothing.)
function flame_hook.instant_chip(user: Object, spec: InstantChipSpec)
    local o = battle.spawn_kind("flame-hook", Vec3.new(spec.panel_y, spec.element, spec.z), spec.params)
    if o then
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        o.element = spec.element
        o.alliance, o.flip = user.alliance, user.flip
        o.damage = spec.damage
        o.stamina = bit32.rshift(spec.damage, 16)
        o.related1 = user
    end
end

-- A flame of swing `swing` on column `x`.
local function flame(me: Object, x: number, row: number, swing: number)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    flame_hook_fire.spawn(me, x, row, me.element, { swing, me:param(1), me:param(2), 0 }, damage)
end

-- Wait out the phase, then the next.
local function wait(me: Object, next: number)
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me.phase = next
        me.phase_init = 0
    end
end

-- `sub_80EA05C`.
local function first(me: Object)
    if me.phase_init == 0 then
        local x = if me:param(1) == 0 then me.panel_x + me.facing else 4 - me.alliance
        flame(me, x, FIRST_ROW, 0)
        me.timer = SWING_TICKS
        me.phase_init = 4
    end
    wait(me, SECOND)
end

-- `sub_80EA0A0`.
local function second(me: Object)
    if me.phase_init == 0 then
        local x = if me:param(1) == 0 then me.panel_x + 2 * me.facing else 5 - me.alliance * 3
        flame(me, x, SECOND_ROW, 1)
        me.timer = SWING_TICKS
        me.phase_init = 4
    end
    wait(me, DONE)
end

-- `sub_80EA0EC`.
local function done(me: Object)
    if me.phase_init == 0 then
        me.timer = END_TICKS
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80EA03C`: by phase.
local function tick(me: Object)
    local p = me.phase
    if p == FIRST then
        first(me)
    elseif p == SECOND then
        second(me)
    elseif p == DONE then
        done(me)
    else
        error(string.format("FlmHook's hook phase %#x reads past its table", p))
    end
end

-- `sub_80EA010`.
function flame_hook.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        me:set_lifecycle("update")
        tick(me)
    elseif l == "update" then
        tick(me)
    else
        -- sub_80EA10A (its write through ExtraVars+4 lands outside RAM).
        me:free()
    end
end

return flame_hook
