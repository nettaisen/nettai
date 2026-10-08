--!strict
-- BugFix (EXE4 chip 0x0CE): a dimming chip that fixes the user's NaviCust bugs while a copy of its navi glows over it:
-- the dimming chips' action 0x0C, its variant 0x1D (0x080220E0's), whose controller (effect object #0x3B, 0x080E6388:
-- @exelib/bugfix/controller, EXE5's and EXE6's but for its effect, 0x080E63CC) puts EXE4's glow over the user's side's
-- navi (objects/glow with BugFix's flashes, its first row: 0x080CD058, then 30 ticks, 0x080CD0A6), fixes the
-- side's stats (0x080E641E: rules/navicust's `fix_bugs`) and waits for the glow; no Beast Out, and no emotion window
-- to clear (EXE4's has no bug flicker). Its spawner (0x080E63FC) is the usual one. Its record is written by
-- tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local controller = require("@exelib/bugfix/controller")
local glow = require("../objects/glow")
local navicust = require("../rules/navicust")

-- The glow's wait before it leaves (0x080CD0A6).
local WAIT = 0x1E

-- The glow's flashes (0x080CD058): the navi vanishes (0x080E2D56) and the copy flashes, with its sound (0x15C).
local flashes = controller.flashes(asset.sound("bugfix-flash"), function(me: Object)
    dimming.hide_user_sparing(glow.navi_of(me, "0x080E2D56"))
end)

-- 0x080CD03C: the glow's first row, by its action: the flashes, then 30 ticks, and it leaves.
local GLOW = glow.make {
    id = "bugfix/glow",
    act = function(me: Object)
        local a = me.action
        if a == 0 then
            flashes(me)
        elseif a == 4 then
            -- 0x080CD0A6.
            if me.phase_init == 0 then
                me.timer = WAIT
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = t
            if t <= 0 then
                glow.back(me)
                glow.done(me)
            end
        else
            error(string.format("BugFix's glow action %#x reads past 0x080CD050's table", a))
        end
    end,
}

local CONTROLLER = controller.make {
    id = "bugfix/controller",
    glow = GLOW.spawn,
    fix = function(user: Object)
        navicust.fix_bugs(user.alliance)
    end,
    beast_out = false,
}

local chip: Chip = {
    codes = { "B", "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 62,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = CONTROLLER.spawn,
}

return { bugfix = chip }
