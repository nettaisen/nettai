--!strict
-- Invisibl's dimming controller (the original's effect object #0x5D,
-- `sub_80E74D4`), which the second WhiCapsl (chips/whicapsl-invisible)
-- uses too: the usual dimming phases (lib/dimming),
-- with an effect that turns the user invisible, flashing for the chip's
-- time (`sub_8010474`), then waits 31 ticks. See docs/engine/chips.md
-- §3.6.10.
--
-- A chip's `dimming` hook (`sub_80E7546`, `off_802CCB4[1]`) is
-- `invisible.hook(ticks)`: the time the original's first two chip
-- parameters gave (their halfword) is the hook's argument.

local dimming_chips = require("@common/dimming")

local invisible = {}

export type State = {
    -- How long the user stays invisible.
    ticks: number,
}

local SOUND = asset.sound("bn6:invisible")
local WAIT = 0x1E

-- `sub_80E7518`: the user turns invisible, then 31 ticks pass.
local function effect(me: Object)
    if me.phase_init == 0 then
        local user = me.related1
        if not user then
            error("Invisibl's controller has no user (sub_8010474 writes through a null pointer)")
        end
        -- sub_8010474
        user:set_status_timer("flash", (me.state :: State).ticks)
        user:set_status("invisible", true)
        battle.play_sound(SOUND)
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

invisible.kind = define.kind {
    id = "bn6:invisibl/controller",
    pool = "effect",
    state = { ticks = "u16" },
    -- `sub_80E74D4`.
    update = dimming_chips.phases { name = "Invisibl", effect = effect },
}

-- A chip's `dimming` hook (`sub_80E7546`): the controller for `user`, on
-- its side, turning it invisible for `ticks`. (The spawner copies the
-- user's side only, not its facing.)
function invisible.hook(ticks: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(invisible.kind, user, spec, true)
        if o then
            local s = o.state :: State
            s.ticks = ticks
        end
        return o
    end
end

return invisible
