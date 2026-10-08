--!strict
-- PopUp's dimming controller (effect object #0x3D, 0x080E6470: EXE4's own): the usual dimming phases
-- (@exelib/dimming), with an effect (0x080E64B4) that puts its glow over the user's side's navi (./glow: the navi goes
-- under the ground) and waits for it. Its spawner (0x080E64DA, the chip's `dimming` hook) copies the user's side, not
-- its facing.

local dimming_chips = require("@exelib/dimming")
local glow = require("./glow")

export type State = {
    -- The glow runs (the original's Param2, which the glow clears).
    busy: boolean,
}

-- 0x080E64B4.
local function effect(me: Object)
    if me.phase_init == 0 then
        glow.spawn(me, me)
        me.phase_init = 4
    end
    if not (me.state :: State).busy then
        dimming_chips.done(me)
    end
end

local KIND = new.kind {
    id = "popup/controller",
    pool = "effect",
    state = { busy = "bool" },
    -- 0x080E6470.
    update = dimming_chips.phases { name = "PopUp", effect = effect },
}

-- 0x080E64DA.
return function(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(KIND, user, spec, true)
end
