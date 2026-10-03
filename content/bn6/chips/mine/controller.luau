--!strict
-- The Mine chip's dimming controller (the original's effect object #0x29,
-- `sub_80E342C`): the usual dimming phases (lib/dimming), with an effect
-- that lays a mine (./land_mine) for the user's side with the damage plus
-- the bonus, and waits 121 ticks.
--
-- The chip's `dimming` hook (`sub_80E349E`, `off_802CCB4[11]`) is
-- `controller.hook`.

local dimming_chips = require("@common/dimming")
local land_mine = require("./land_mine")

export type State = {
    -- The Atk+ / cross bonus the telop shows (object +0x32), which adds to
    -- the mine's damage.
    bonus: number,
}

local WAIT = 0x78

-- `sub_80E3470`: the mine, then 121 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        local s = me.state :: State
        land_mine.spawn(me.alliance, dimming_chips.damage_plus(me, s.bonus))
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "bn6:mine/controller",
    pool = "effect",
    state = { bonus = "u16" },
    -- `sub_80E342C`.
    update = dimming_chips.phases { name = "Mine", effect = effect },
}

-- The chip's `dimming` hook (`sub_80E349E`): the controller for `user`, on
-- its side. (The spawner copies the user's side only, not its facing.)
local function hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(kind, user, spec, true)
    if o then
        local s = o.state :: State
        s.bonus = spec.bonus
    end
    return o
end

return { kind = kind, hook = hook }
