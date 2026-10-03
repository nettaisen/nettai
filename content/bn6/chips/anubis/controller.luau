--!strict
-- Anubis's and PoisPhar's dimming controller (the original's effect object
-- #0x33, `sub_80E40EC`; docs/engine/dimming-chip-effects.md §7): the usual
-- dimming phases (@common/dimming), with an effect that drops the chip's
-- statue (./statue) on the panel in front of the user, whatever is there,
-- and waits 120 ticks.

local dimming_chips = require("@common/dimming")
local statue = require("./statue")

export type State = { variant: AnubisVariant }

local WAIT = 0x78

-- `sub_80E4130`: the statue, then 120 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        statue.spawn(me, (me.state :: State).variant, me.panel_x + me.facing, me.panel_y, damage)
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "bn6:anubis/controller",
    pool = "effect",
    state = { variant = "record:anubis-variant" },
    -- `sub_80E40EC`.
    update = dimming_chips.phases { name = "Anubis", effect = effect },
}

-- A chip's `dimming` hook (`sub_80E4164`): the controller dropping a
-- statue of `variant`.
local function hook(variant: AnubisVariant): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec)
        if o then
            (o.state :: State).variant = variant
        end
        return o
    end
end

return { hook = hook }
