--!strict
-- DblHero's dimming controller (the original's effect object #0x6A,
-- `sub_80E7FDC`; docs/engine/dimming-chip-effects.md §17): the usual
-- dimming phases (@common/dimming), with an effect that sends the heroes
-- (./heroes) out in the user's place and waits until MegaMan is done.

local dimming_chips = require("@common/dimming")
local heroes = require("./heroes")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus (object +0x32): it adds to the heroes'
    -- damage word.
    bonus: number,
    -- MegaMan is out (the original's second parameter, whose address he
    -- holds).
    busy: boolean,
}

-- `sub_80E8020`: the heroes; once MegaMan is done, the effect is over.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        heroes.spawn(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus))
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

controller.kind = define.kind {
    id = "bn6:dblhero/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool" },
    -- `sub_80E7FDC`.
    update = dimming_chips.phases { name = "DblHero", effect = effect },
}

-- `sub_80E8058`: DblHero's `dimming` hook. (The spawner copies the user's
-- side, not its facing.)
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller.kind, user, spec, true)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return controller
