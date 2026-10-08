--!strict
-- PopUp's glow (EXE4's glow, ../../objects/glow: the original's first parameter 1, 0x080CD0E2; EXE6's `sub_80C49E4`,
-- which nothing there spawns): the navi vanishes, out from under the ground if it was (0x0800C13C, EXE6's
-- `sub_80101C4`), the copy white for 31 ticks; then the navi is back and goes under the ground for 480 ticks
-- (0x0800C126, EXE6's `sub_80101AE`: hidden, colliding only with what reaches under it, until a hit, an action or the
-- time ends it: the navi's status tick, 0x0800C0DA; its tail keeps the hole over it, 0x0800C0A6), with its sound.

local glow = require("../../objects/glow")

-- The white copy's ticks (less one), the time under the ground.
local WHITE, UNDER = 30, 480

local SOUND = asset.sound("invisible")

-- 0x080CD0E2.
local function dive(me: Object)
    if me.phase_init == 0 then
        local navi = glow.navi_of(me, "0x080E2D56")
        dimming.hide_user_sparing(navi)
        navi:set_status_timer("submerged", 0)
        navi:set_status("submerged", false)
        me.sprite.white = true
        me.timer = WHITE
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    glow.back(me)
    local navi = glow.navi_of(me, "0x0800C126")
    navi:set_status_timer("submerged", UNDER)
    navi:set_status("submerged", true)
    navi.visible = false
    battle.play_sound(SOUND)
    glow.done(me)
end

return glow.make { id = "popup/glow", act = dive }
