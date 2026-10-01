--!strict
-- BugFix's dimming controller (the original's effect object #0x3B,
-- `sub_80E4910`; docs/engine/dimming-chips.md §5.1): the usual dimming
-- phases (lib/dimming), with an effect (`sub_80E4954`) that puts the glow
-- over the user's side's navi (glow.luau), fixes the user's side's
-- NaviCust bugs, and waits for the glow.

local controller = require("../../lib/dimming/controller")
local glow = require("./glow")

local bugfix = {}

export type State = {
    -- The glow runs (the original's Param2, which the glow clears).
    busy: boolean,
}

-- What the effect's first tick sets the timer to (nothing counts it).
local TIMER = 0x3C

-- `sub_80E4954`.
local function effect(me: Object)
    if me.phase_init == 0 then
        -- sub_80C4AEC
        glow.spawn(me, glow.BUG_FIX, me)
        -- sub_80E49C4
        local user = controller.user(me, "sub_80E49C4")
        battle.clear_navicust_bugs(user.alliance)
        -- sub_801E658: the emotion portrait's bug face goes (the HUD,
        -- which the engine doesn't draw).
        -- sub_8014446 / sub_801443C: the Beast Out counter is spent
        -- again unless the navi has one.
        user.beast_out_spent = battle.navi(user.alliance).beast_out_counter == 0
        me.timer = TIMER
        me.phase_init = 4
    end
    if not (me.state :: State).busy then
        controller.undim(me)
    end
end

bugfix.kind = define.kind {
    id = "bugfix/controller",
    pool = "effect",
    state = { busy = "bool" },
    update = controller.update("BugFix's controller", effect),
}

-- `sub_80E49A2`: the chip's `dimming` hook.
function bugfix.spawn(user: Object, spec: DimmingChipSpec): Object?
    return controller.spawn(bugfix.kind, user, spec)
end

return bugfix
