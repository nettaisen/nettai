--!strict
-- A Guardian statue's strike-back dimming (the original's effect object
-- #0x53, `sub_80E680C`; docs/engine/dimming-chip-effects.md §5): the
-- controller the statue (./statue) starts when an attack breaks it. The
-- usual dimming phases (its telop names Punisher), with an effect that
-- tells the statue to strike and waits 60 ticks.

local dimming_chips = require("@exelib/dimming")
local punisher = require("../punisher/chip")

local strike = {}

-- What its telop names (`sub_80D4DF0` spawns it with Punisher, bonus 0).
strike.telop = { chip = punisher } :: DimmingTelop

-- What it tells the statue (the statue's state).
type StatueState = { strike: boolean }

local WAIT = 0x3C

-- `sub_80E6850`: the statue strikes; 60 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local statue = me.related1
        if not statue then
            error("the Guardian strike's controller has no statue (sub_80E6850 writes through a null pointer)")
        end
        (statue.state :: StatueState).strike = true
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "bn6:guardian/strike",
    pool = "effect",
    -- `sub_80E680C`.
    update = dimming_chips.phases { name = "the Guardian strike", effect = effect },
}

-- `sub_80E6878`: the controller for `statue`, on its panel and side, with
-- its element and damage word. (It sits at the spawner's registers: the
-- panel's Y, the element, 0.)
function strike.spawn(statue: Object): Object?
    local o = battle.spawn(kind, Vec3.new(statue.panel_y, statue.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = statue.panel_x, statue.panel_y
    o.element = statue.element
    o.related1 = statue
    o.alliance = statue.alliance
    o.damage, o.stamina = statue.damage, statue.stamina
    return o
end

return strike
