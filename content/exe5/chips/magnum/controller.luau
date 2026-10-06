--!strict
-- Magnum's dimming controller (EXE5's effect object #0x30, 0x080E3D84; the dimming handler's subtype 15, 0x080E3E04):
-- the usual dimming phases (@exelib/dimming), with an effect (0x080E3DC8) that sends out the gunner (./gunner) with
-- the damage plus the bonus (and Magnum's mode 0, Param1), and lasts until the gunner has fired and gone (its stage,
-- Param2, at 2).

local dimming_chips = require("@exelib/dimming")
local gunner = require("./gunner")

local controller = {}

export type State = {
    -- The Atk+ bonus the telop shows (+0x32), which the shot's damage adds.
    bonus: number,
    -- The gunner's stage (Param2: 1 fired, 2 gone).
    stage: number,
}

local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        gunner.spawn(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus))
        me.timer = 60
        me.phase_init = 4
    end
    if s.stage >= 2 then
        dimming_chips.done(me)
    end
end

controller.kind = new.kind {
    id = "magnum/controller",
    pool = "effect",
    state = { bonus = "u16", stage = "u8" },
    update = dimming_chips.phases { name = "Magnum", effect = effect },
}

-- 0x080E3E04: Magnum's `dimming` hook.
function controller.hook(): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(controller.kind, user, spec)
        if o then
            local s = o.state :: State
            s.bonus = spec.bonus
        end
        return o
    end
end

return controller
