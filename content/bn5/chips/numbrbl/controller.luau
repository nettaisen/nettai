--!strict
-- NumbrBl's dimming controller (BN5's effect object #0x69, 0x080E82DC): BN6's (@common/numbrbl/controller) with
-- BN5's effect (0x080E8324), by its phase: the user warps out (30 ticks); NumberMan himself (./numberman) on the
-- user's panel, until he is done; 30 ticks; the user warps back in (30 ticks); then the undim.

local common = require("@common/numbrbl/controller")
local dimming_chips = require("@common/dimming")
local numberman = require("./numberman")

local controller = {}

local WARP_TICKS = 0x1E

-- 30 ticks from the phase's first, then phase `next`.
local function wait(me: Object, next: number)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.phase, me.phase_init = next, 0
    end
end

-- 0x080E8324, by its phase.
local function effect(me: Object)
    local s = me.state :: any
    local user = me.related1 :: Object
    local phase = me.phase
    if phase == 0 then
        -- 0x080E8348: the warp out.
        if me.phase_init == 0 then
            navi_chip.warp(user, true)
            me.timer = WARP_TICKS
            me.phase_init = 4
        end
        wait(me, 4)
    elseif phase == 4 then
        -- 0x080E836E: NumberMan, the chip's first parameter his level; once he is done, on.
        if me.phase_init == 0 then
            numberman.spawn(me, me.panel_x, me.panel_y, dimming_chips.damage_plus(me, s.bonus), s.balls)
            me.phase_init = 4
        end
        if not s.busy then
            me.phase, me.phase_init = 8, 0
        end
    elseif phase == 8 then
        -- 0x080E839E: 30 ticks.
        if me.phase_init == 0 then
            me.timer = WARP_TICKS
            me.phase_init = 4
        end
        wait(me, 0xC)
    elseif phase == 0xC then
        -- 0x080E83BC: the warp back in; 30 ticks, then the undim.
        if me.phase_init == 0 then
            navi_chip.warp(user, false)
            me.timer = WARP_TICKS
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me:set_action(common.UNDIM)
        end
    else
        error(string.format("NumbrBl's controller: phase %#x of its effect reads past its table (0x080E8338)", phase))
    end
end

controller.kind = common.make("bn5:numbrbl/controller", effect)

-- 0x080E83E6: NumbrBl's `dimming` hook, NumberMan of level `level`.
function controller.hook(level: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return common.hook(controller.kind, level)
end

return controller
