--!strict
-- The Guardian chip's dimming controller (effect object #0x52,
-- `sub_80E6758`): the usual dimming phases (the dimming service), with an
-- effect that places a Guardian statue (objects/guardian-statue) on the
-- panel in front of the user if it is free solid ground, with the damage
-- plus the bonus, and waits 30 ticks.
--
-- The chip's script (dimming chip subtype 14): `dimming_chip` is its
-- controller's spawner, `off_802CCB4[14]`.

local guardian_statue = require("../guardian-statue/guardian_statue")

export type State = {
    -- The chip and the Atk+ / cross bonus the telop shows (object +0x30,
    -- +0x32); the bonus adds to the statue's damage.
    chip: number,
    bonus: number,
}

local guardian = {
    state = { chip = "u16", bonus = "u16" },
}

local WAIT = 0x1E
-- Where the statue goes: a solid panel with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080

-- `sub_80E67E6`: the controller for `user`, on its panel, with the chip's
-- parameters and damage word. (Its position is the spawner's registers,
-- which nothing reads.)
function guardian.dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local o = battle.spawn_kind("guardian", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = user.panel_x, user.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.chip, s.bonus = spec.chip, spec.bonus
    return o
end

-- `sub_80E679C`: the statue, then 30 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local x, y = me.panel_x + me.facing, me.panel_y
        if field.check(x, y, SOLID, OCCUPIED) then
            local s = me.state :: State
            local params = { me:param(1), me:param(2), me:param(3), me:param(4) }
            local damage = int.u32(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)) + s.bonus)
            guardian_statue.spawn(me, x, y, params, damage, Vec3.new(y, me.element, s.bonus))
        end
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me:set_action(0xC)
end

-- `sub_80E6758`: the dimming's phases around the effect.
function guardian.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        dimming.begin(me)
    elseif l == "update" then
        local a = me.action
        if a == 0 then
            dimming.dim_screen(me)
        elseif a == 4 then
            dimming.show_telop(me)
        elseif a == 8 then
            effect(me)
        elseif a == 0xC then
            dimming.undim_screen(me)
        else
            error(string.format("Guardian's action %#x reads past its table", a))
        end
    else
        dimming.finish(me)
    end
end

return guardian
