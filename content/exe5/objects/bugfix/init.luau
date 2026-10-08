--!strict
-- EXE5's BugFix and the Liberation chips' controllers (the dimming handler's subtypes 22 and 66), and their glow:
--
-- - the glow (actor object #0x5D, 0x080C42E8): EXE6's (@exelib/bugfix/glow) a pixel up the field and lower than the
--   navi (EXE6's down and higher), a player navi's parts stepping even while paused (0x0800F0EC with r2 1);
-- - BugFix's controller (effect object #0x3B, 0x080E4940): EXE6's (@exelib/bugfix/controller) without Beast Out;
-- - the Liberation chips' (LCrsShld, LStepSwd, LCounter: effect object #0x8A, 0x080EA740, its effect 0x080EA784):
--   the usual dimming phases; the glow over the user's side's navi, starting with its log in (`LOG_IN`); the side's
--   Liberation ability set, five uses of the chip's (its first parameter: 0 CrsShld, 1 StepSwd, 2 Counter), and the
--   user's AIData+0x44 bit 0x800000; the timer at 60; done when the glow is. (The ability's state, its side state's
--   +0x51 and +0x52 and that bit, is read only by the operation battle's specials, 0x0802FA9C and 0x0802E77E: a
--   Liberation Mission's, never a netbattle's. The engine models neither, so the chips set nothing: one hook.)

local common_glow = require("@exelib/bugfix/glow")
local controller = require("@exelib/bugfix/controller")
local dimming_chips = require("@exelib/dimming")

local bugfix = {}

local FLASH, LOG_IN_SOUND = asset.sound("bugfix-flash"), asset.sound("log-in")

bugfix.glow = common_glow.make {
    id = "bugfix/glow",
    offset = -1,
    -- A player navi's overlay (0x0800F0EC: its soul's put-on, NumberSoul's layer).
    wear = function(me: Object, navi: Object)
        if navi.actor_type == "player" then
            me:add_overlay_of(navi)
        end
    end,
}

-- The glow's flashes (`off_80C4930`'s first row, `sub_80C4958`: EXE5's 0x080C4420).
local FLASHES = common_glow.start(controller.flashes(FLASH, common_glow.hide_navi))

bugfix.controller = controller.make {
    id = "bugfix/controller",
    glow = function(spawner: Object, holder: Object): Object?
        return bugfix.glow.spawn(spawner, FLASHES, holder)
    end,
    fix = controller.clear_bugs,
    beast_out = false,
}

export type LiberationState = {
    -- The glow runs (Param2, which the glow clears).
    busy: boolean,
}

-- What the effect's first tick sets the timer to (nothing counts it).
local TIMER = 60

-- How long the log in shows white.
local LOG_IN_TICKS = 0x1E

-- The glow's start (`off_80C4930`'s third row, `sub_80C4A6C`): the navi vanishes, with the log-in sound and a flash;
-- white for 30 ticks.
local LOG_IN = common_glow.start(function(me: Object)
    me.sprite.white = false
    if me.phase_init == 0 then
        common_glow.hide_navi(me)
        me.timer = LOG_IN_TICKS
        battle.play_sound(LOG_IN_SOUND)
        battle.play_sound(FLASH)
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_action(4)
        return
    end
    me.sprite.white = true
end)

-- 0x080EA784.
local function liberation(me: Object)
    if me.phase_init == 0 then
        -- sub_80C4AEC
        bugfix.glow.spawn(me, LOG_IN, me)
        me.timer = TIMER
        me.phase_init = 4
    end
    if not (me.state :: LiberationState).busy then
        dimming_chips.done(me)
    end
end

local LIBERATION = new.kind {
    id = "liberation/controller",
    pool = "effect",
    state = { busy = "bool" },
    -- 0x080EA740.
    update = dimming_chips.phases { name = "Liberation", effect = liberation },
}

-- 0x080EA7CC: a Liberation chip's `dimming` hook (the controller takes the user's side, not its facing).
function bugfix.liberation(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(LIBERATION, user, spec, true)
end

return bugfix
