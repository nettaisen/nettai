--!strict
-- CornFsta's dimming controller (the original's effect object #0x68,
-- `sub_80E7E9C`; docs/engine/dimming-chip-effects.md §16): the usual
-- dimming phases (lib/dimming), with an effect that sends the farmer
-- (./farmer) out in the user's place and waits until he is done.

local dimming_chips = require("@common/dimming")
local farmer = require("./farmer")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus (object +0x32): it adds to the farmer's
    -- damage word.
    bonus: number,
    -- The farmer is out (the original's second parameter, whose address
    -- he holds).
    busy: boolean,
}

-- `sub_80E7EE0`: the farmer; once he is done, the effect is over. (The
-- chip's parameters, [2, 1], are read by nothing.)
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        farmer.spawn(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus))
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

controller.kind = define.kind {
    id = "bn6:cornfsta/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool" },
    -- `sub_80E7E9C`.
    update = dimming_chips.phases { name = "CornFsta", effect = effect },
}

-- `sub_80E7F16`: CornFsta's `dimming` hook. (The spawner copies the
-- user's side, not its facing.)
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller.kind, user, spec, true)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return controller
