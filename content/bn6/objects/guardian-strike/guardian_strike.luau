--!strict
-- A Guardian statue's strike-back dimming (effect object #0x53,
-- `sub_80E680C`): the controller the statue (objects/guardian-statue)
-- starts when an attack breaks it. The usual dimming phases (the dimming
-- service), with the telop of chip 0x175, and an effect that tells the
-- statue to strike (its Param3) and waits 60 ticks.

export type State = {
    -- The chip the telop names (object +0x30) and its bonus (+0x32).
    chip: number,
    bonus: number,
}

local guardian_strike = {
    state = { chip = "u16", bonus = "u16" },
}

local WAIT = 0x3C
-- The chip whose name the telop shows.
local TELOP_CHIP = 0x175

-- `sub_80E6878`: the controller for `statue`, on its panel and side, with
-- its parameters, element and damage word. (It sits at the spawner's
-- registers: the panel's Y, the element, 0.)
function guardian_strike.spawn(statue: Object): Object?
    local params = { statue:param(1), statue:param(2), statue:param(3), statue:param(4) }
    local o = battle.spawn_kind("guardian-strike", Vec3.new(statue.panel_y, statue.element, 0), params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = statue.panel_x, statue.panel_y
    o.element = statue.element
    o.related1 = statue
    o.alliance = statue.alliance
    o.damage, o.stamina = statue.damage, statue.stamina
    local s = o.state :: State
    s.chip, s.bonus = TELOP_CHIP, 0
    return o
end

-- `sub_80E6850`: the statue strikes; 60 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local statue = me.related1
        if not statue then
            error("the Guardian strike's controller has no statue (sub_80E6850 writes through a null pointer)")
        end
        statue:set_param(3, 1)
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

-- `sub_80E680C`: the dimming's phases around the effect.
function guardian_strike.update(me: Object)
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
            error(string.format("The Guardian strike's action %#x reads past its table", a))
        end
    else
        dimming.finish(me)
    end
end

return guardian_strike
