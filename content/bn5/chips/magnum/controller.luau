--!strict
-- Magnum's dimming controller (BN5's effect object #0x30, 0x080E3D84; the dimming handler's subtype 15, 0x080E3E04):
-- the usual dimming phases (@exelib/dimming), with an effect (0x080E3DC8) that sends out the gunner (./gunner) with
-- the damage plus the bonus and the chip's mode (Param1), and lasts until the gunner has fired and gone (its stage,
-- Param2, at 2).

local dimming_chips = require("@exelib/dimming")
local gunner = require("./gunner")

local controller = {}

export type State = {
    -- The Atk+ bonus the telop shows (+0x32), which the shot's damage adds.
    bonus: number,
    -- The gunner's mode (Param1: Magnum's 0) and its stage (Param2: 1 fired, 2 gone).
    mode: number,
    stage: number,
}

local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        gunner.spawn(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus), s.mode)
        me.timer = 60
        me.phase_init = 4
    end
    if s.stage >= 2 then
        dimming_chips.done(me)
    end
end

controller.kind = define.kind {
    id = "bn5:magnum/controller",
    pool = "effect",
    state = { bonus = "u16", mode = "u8", stage = "u8" },
    update = dimming_chips.phases { name = "Magnum", effect = effect },
}

-- 0x080E3E04: Magnum's `dimming` hook, of mode `mode` (its record's Param1).
function controller.hook(mode: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(controller.kind, user, spec)
        if o then
            local s = o.state :: State
            s.bonus, s.mode = spec.bonus, mode
        end
        return o
    end
end

return controller
