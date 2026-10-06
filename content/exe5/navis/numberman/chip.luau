--!strict
-- NumberMan's own chip, NumTrap (EXE5 chip 0x1a1), which his custom screen offers until it is picked (the navi's
-- `own_chip`): his own action 8 (0x080F169C). A dimming chip, but not by the dimming chips' action: he spawns its
-- controller (./controller) and starts the dimming himself, which the other side can't cut in on (0x0800A512 with
-- its flag); the dimming holds him in the action, and 10 ticks after it he is back to idle. Since its action isn't
-- the dimming chips', it cuts in on nothing itself (0x0801454C tests the action). The controller sets a trap on the
-- panel two ahead of him (./trap), which brings a press down on whoever of the other side steps on it (./press).
-- Its damage is by his level (the chips' damage formula 66: the damage rows' row 17).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi_level = require("../../lib/navi_level")
local controller = require("./controller")

export type State = {
    -- AIAttackVars +0x10: the ticks left.
    timer: number,
}

-- The steps, on the attack's step.
local CALL, LEAVE = 0x0, 0x4
local LEAVE_TICKS = 10

local numtrap: Chip

local action = define.action {
    id = "numtrap/action",
    state = { timer = "u16" },
    -- 0x080F169C
    update = function(me: Object, s: State)
        local step = me.step
        if step == CALL then
            -- 0x080F16B8: the controller on his panel, with the attack's element, damage word and bonus; its telop
            -- names the chip.
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
            local c = controller.spawn(me, me.attack_element, damage, me.extra)
            dimming.start(me.alliance, true, c, me, { chip = numtrap, bonus = me.extra })
            me.step, me.step_init = LEAVE, 0
        elseif step == LEAVE then
            -- 0x080F16E8
            if me.step_init == 0 then
                me.step_init = 4
                s.timer = LEAVE_TICKS
                return
            end
            local t = s.timer - 1
            s.timer = bit32.band(t, 0xFFFF)
            if t <= 0 then
                me:exit_attack()
            end
        else
            error(string.format("NumberMan's NumTrap's step %#x reads past its table (0x080F16B0)", step))
        end
    end,
}

numtrap = define.chip {
    id = "numtrap",
    codes = { "N" },
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = navi_level.row({ 120, 120, 120, 120, 120, 140, 140 }),
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = action,
}

return numtrap
