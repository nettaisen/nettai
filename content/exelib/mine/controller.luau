--!strict
-- The Mine chip's dimming controller, as EXE5 and EXE6 share it (EXE6's
-- effect object #0x29, `sub_80E342C`; EXE5's #0x29): each game makes its
-- own (`controller.make`: its kind and its mine's). The usual dimming
-- phases (@exelib/dimming), with an effect that lays a mine (./land_mine)
-- for the user's side with the damage plus the bonus, and waits 121 ticks.
--
-- The chip's `dimming` hook (`sub_80E349E`, `off_802CCB4[11]`) is the made
-- controller's `hook`.

local dimming_chips = require("../dimming")
local land_mine = require("./land_mine")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus the telop shows (object +0x32), which adds to
    -- the mine's damage.
    bonus: number,
}

local WAIT = 0x78

-- `sub_80E3470`: the mine (of `mine`), then 121 ticks.
local function effect(me: Object, mine: Kind)
    if me.phase_init == 0 then
        me.timer = WAIT
        local s = me.state :: State
        land_mine.spawn(mine, me.alliance, dimming_chips.damage_plus(me, s.bonus))
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

-- A game's controller: its kind's key, and its mine's kind.
function controller.make(spec: { id: string, mine: Kind }): { kind: Kind, hook: (user: Object, chip: DimmingChipSpec) -> Object? }
    local kind = new.kind {
        id = spec.id,
        pool = "effect",
        state = { bonus = "u16" },
        -- `sub_80E342C`.
        update = dimming_chips.phases {
            name = "Mine",
            effect = function(me: Object)
                effect(me, spec.mine)
            end,
        },
    }

    -- The chip's `dimming` hook (`sub_80E349E`): the controller for `user`,
    -- on its side. (The spawner copies the user's side only, not its
    -- facing.)
    local function hook(user: Object, chip: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, chip, true)
        if o then
            local s = o.state :: State
            s.bonus = chip.bonus
        end
        return o
    end

    return { kind = kind, hook = hook }
end

return controller
