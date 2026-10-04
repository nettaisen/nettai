--!strict
-- BN5's BugFix and the Liberation chips' controllers (the dimming handler's subtypes 22 and 66), and their glow:
--
-- - the glow (actor object #0x5D, 0x080C42E8): BN6's (@exelib/bugfix/glow) a pixel up the field and lower than the
--   navi (BN6's down and higher), a player navi's parts stepping even while paused (0x0800F0EC with r2 1);
-- - BugFix's controller (effect object #0x3B, 0x080E4940): BN6's (@exelib/bugfix/controller) without Beast Out;
-- - the Liberation chips' (LCrsShld, LStepSwd, LCounter: effect object #0x8A, 0x080EA740, its effect 0x080EA784):
--   the usual dimming phases; the glow's log-in variant over the user's side's navi; the side's Liberation ability
--   set, five uses of the chip's (its first parameter: 0 CrsShld, 1 StepSwd, 2 Counter), and the user's AIData+0x44
--   bit 0x800000; the timer at 60; done when the glow is. (The ability's state, its side state's +0x51 and +0x52
--   and that bit, is read only by the operation battle's specials, 0x0802FA9C and 0x0802E77E: a Liberation Mission's,
--   never a netbattle's. The engine models neither, so the chip sets nothing a battle reads.)

local common_glow = require("@exelib/bugfix/glow")
local controller = require("@exelib/bugfix/controller")
local dimming_chips = require("@exelib/dimming")

local bugfix = {}

bugfix.glow = common_glow.make {
    id = "bugfix/glow",
    sounds = {
        flash = asset.sound("bugfix-flash"),
        log_in = asset.sound("log-in"),
        submerge = asset.sound("invisible"),
    },
    offset = -1,
    bn5_parts = true,
}

bugfix.controller = controller.make { id = "bugfix/controller", glow = bugfix.glow.spawn, beast_out = false }

-- The Liberation abilities (the chips' first parameter).
export type Ability = "crsshld" | "stepswd" | "counter"

export type LiberationState = {
    -- The glow runs (Param2, which the glow clears).
    busy: boolean,
    ability: Ability,
}

-- What the effect's first tick sets the timer to (nothing counts it).
local TIMER = 60

-- 0x080EA784.
local function liberation(me: Object)
    if me.phase_init == 0 then
        -- sub_80C4AEC
        bugfix.glow.spawn(me, common_glow.LOG_IN, me)
        me.timer = TIMER
        me.phase_init = 4
    end
    if not (me.state :: LiberationState).busy then
        dimming_chips.done(me)
    end
end

local LIBERATION = define.kind {
    id = "liberation/controller",
    pool = "effect",
    state = { busy = "bool", ability = { "crsshld", "stepswd", "counter" } },
    -- 0x080EA740.
    update = dimming_chips.phases { name = "Liberation", effect = liberation },
}

-- 0x080EA7CC: a Liberation chip's `dimming` hook, giving `ability` (the controller takes the user's side, not its
-- facing).
function bugfix.liberation(ability: Ability): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(LIBERATION, user, spec, true)
        if o then
            (o.state :: LiberationState).ability = ability
        end
        return o
    end
end

return bugfix
