--!strict
-- NrthWind's controller, as EXE6's, EXE5's and EXE4's code share it (effect object #0x39: EXE6's `sub_80E47D4`, which
-- no EXE6 chip uses; EXE5's 0x080E4804; EXE4's 0x080E622C): the usual dimming phases (@exelib/dimming), with an effect
-- that blows its game's north wind (@exelib/nrthwind/wind: of the controller's side, running while dimmed) and waits
-- for it, then undims.
--
-- Each game makes its chip's (`controller.make`): its kind's id, its wind, and whether its spawn copies the user's side
-- alone (EXE4's 0x080E629C; EXE5's copies its facing too).

local dimming_chips = require("../dimming")

local controller = {}

type State = {
    -- The wind blows (the original's Param2, which the wind clears).
    busy: boolean,
}

export type Spec = {
    id: string,
    -- Its game's north wind: `blow` from the controller, telling it when done.
    wind: { blow: (owner: Object, holder: Object?) -> Object? },
    side_only: boolean,
}

-- A game's NrthWind controller: the chip's `dimming` hook.
function controller.make(spec: Spec): (Object, DimmingChipSpec) -> Object?
    -- EXE6's `sub_80E47F8`, EXE4's 0x080E6270: the wind, 60 ticks on its timer; once it is done, undim.
    local function effect(me: Object)
        if me.phase_init == 0 then
            spec.wind.blow(me, me);
            (me.state :: State).busy = true
            me.timer = 0x3C
            me.phase_init = 4
        end
        if not (me.state :: State).busy then
            dimming_chips.done(me)
        end
    end

    local kind = new.kind {
        id = spec.id,
        pool = "effect",
        state = { busy = "bool" },
        update = dimming_chips.phases { name = "NrthWind", effect = effect },
    }

    return function(user: Object, dimming: DimmingChipSpec): Object?
        return dimming_chips.spawn(kind, user, dimming, spec.side_only)
    end
end

return controller
