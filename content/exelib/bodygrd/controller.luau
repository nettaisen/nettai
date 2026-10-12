--!strict
-- BodyGrd's dimming controller, as EXE4 and EXE5 share it (EXE5's effect object #0x6D, 0x080E8620; EXE4's #0x6D,
-- 0x080E9A50, the same code), where EXE6's BodyGrd is a trap: the usual dimming phases (@exelib/dimming); its effect
-- (EXE5's 0x080E8664, EXE4's 0x080E9A94) sends out the game's striker (./striker) at once with the controller's
-- element and damage word plus the Atk+ bonus, and is done when its second parameter is 0.

local dimming_chips = require("../dimming")
local striker = require("./striker")

export type State = {
    -- The Atk+ bonus (+0x32): it adds to the striker's damage word.
    bonus: number,
    -- The chip's second parameter (Param2): the effect waits for it to be 0.
    wait: number,
}

-- A game's controller (kind `id`) sending out its `striker_kind`: its kind, and the chips' hook.
local function make(id: string, striker_kind: Kind)
    -- The striker out, then done once Param2 is 0.
    local function effect(me: Object)
        local s = me.state :: State
        if me.phase_init == 0 then
            striker.spawn(striker_kind, me, me.element, dimming_chips.damage_plus(me, s.bonus))
            me.phase_init = 4
        end
        if s.wait == 0 then
            dimming_chips.done(me)
        end
    end

    local kind = new.kind {
        id = id,
        pool = "effect",
        state = { bonus = "u16", wait = "u8" },
        update = dimming_chips.phases { name = "BodyGrd", effect = effect },
    }

    -- The chip's `dimming` hook (EXE5's 0x080E868C, EXE4's 0x080E9ABC: only the user's side is copied); `wait`: the
    -- chip's second parameter.
    local function hook(wait: number): (user: Object, spec: DimmingChipSpec) -> Object?
        return function(user: Object, spec: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, spec, true)
            if o then
                local s = o.state :: State
                s.bonus, s.wait = spec.bonus, wait
            end
            return o
        end
    end

    return { kind = kind, hook = hook }
end

return { make = make }
