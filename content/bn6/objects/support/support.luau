--!strict
-- The NaviCust supports' dimming controller (effect object #0x79,
-- `sub_80E8FE0`). In link battles a NaviCust program's support acts
-- once: Rush eats a chip the opponent uses and Beat snatches a Mega or
-- Giga chip (their host is the chip user's opponent), Tango heals her
-- host when its HP drops to a quarter. The ruleset turns the chip back
-- (`sub_8010740`, `sub_80106C0`) or holds the navi (`sub_8010660`),
-- spawns this controller on the host's panel (`sub_80E90FE`) and starts a
-- dimming no one can cut in on (`sub_800BF16`).
--
-- The controller dims the screen, shows the telop (the support's chip
-- record, 0x179..0x17B, which the game keeps at +0x30), then: Rush's and
-- Beat's host warps out for 31 ticks (Tango's doesn't), the support comes
-- and the controller waits for it to go (its second parameter is the
-- support's flag), waits 31 ticks, the host warps back in (not
-- Tango's), 31 ticks, and the screen undims. See docs/engine/chips.md
-- §2.10.
--
-- Parameters: the first picks the support (0 Rush, 1 Beat, 2 Tango), the
-- second is its out flag, the third and fourth are the chip Rush
-- eats. Related object 1 is the host.

local rush = require("../rush/rush")
local beat = require("../beat/beat")
local tango = require("../tango/tango")

export type State = {
    -- The chip record the telop names (+0x30: Rush 0x179, Beat 0x17A,
    -- Tango 0x17B), which the ruleset sets. (The port's telop banner
    -- doesn't show a name yet.)
    telop_chip: number,
}

local support = {
    state = {
        telop_chip = "u16",
    },
}

-- The supports' spawners (`off_80E90A4`), by the first parameter.
local SPAWNERS: { [number]: (controller: Object, params: { number }) -> Object? } = {
    [0] = rush.spawn,
    [1] = beat.spawn,
    [2] = tango.spawn,
}

-- The support that doesn't send its host away.
local TANGO = 2

-- How long each wait lasts: the timer counts down past zero, so 31 ticks.
local WAIT = 0x1E

-- A wait's tick (`ldrh`, `sub`, `strh`, `bge`): false once it counts past
-- zero.
local function waiting(me: Object): boolean
    local t = me.timer - 1
    me.timer = int.u16(t)
    return t >= 0
end

local function next_phase(me: Object, phase: number)
    me.phase, me.phase_init = phase, 0
end

-- `sub_80E9024` (action 8), by phase.
local function effect(me: Object)
    local p = me.phase
    if p == 0 then
        -- sub_80E9048: the host warps out.
        if me.phase_init == 0 then
            if me:param(1) == TANGO then
                next_phase(me, 4)
                return
            end
            navi_chip.warp(me.related1 :: Object, true)
            me.timer = WAIT
            me.phase_init = 4
        end
        if not waiting(me) then
            next_phase(me, 4)
        end
    elseif p == 4 then
        -- sub_80E9074: the support comes (with the controller's
        -- parameters); the controller waits for it to go.
        if me.phase_init == 0 then
            local kind = me:param(1)
            local spawn = SPAWNERS[kind]
            if not spawn then
                error(string.format("support %d reads past its spawners (off_80E90A4)", kind))
            end
            spawn(me, { me:param(1), me:param(2), me:param(3), me:param(4) })
            me.phase_init = 4
        end
        if me:param(2) == 0 then
            next_phase(me, 8)
        end
    elseif p == 8 then
        -- sub_80E90B0
        if me.phase_init == 0 then
            me.timer = WAIT
            me.phase_init = 4
        end
        if not waiting(me) then
            next_phase(me, 0xC)
        end
    elseif p == 0xC then
        -- sub_80E90CE: the host warps back in; then the undim.
        if me.phase_init == 0 then
            if me:param(1) ~= TANGO then
                navi_chip.warp(me.related1 :: Object, false)
            end
            me.timer = WAIT
            me.phase_init = 4
        end
        if not waiting(me) then
            me:set_action(0xC)
        end
    else
        error(string.format("the supports' phase %#x reads past its table (off_80E9038)", p))
    end
end

-- `sub_80E8FE0`: the dimming's phases around the effect.
function support.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        dimming.begin(me)
    elseif l == "update" then
        local a = me.action
        if a == 0 then
            dimming.dim_screen(me)
        elseif a == 4 then
            dimming.show_telop(me)
        elseif a == 8 then
            effect(me)
        elseif a == 0xC then
            dimming.undim_screen(me)
        else
            error(string.format("the supports' action %#x reads past its table (off_80E9014)", a))
        end
    else
        dimming.finish(me)
    end
end

return support
