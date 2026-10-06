--!strict
-- SearchMan's own chip, Satelity (EXE5 chip 0x195), which his custom screen offers until it is picked (the navi's
-- `own_chip`): his own action 7 (0x080F0DFC). A dimming chip, but not by the dimming chips' action: he spawns its
-- controller (./controller) and starts the dimming himself, which the other side can't cut in on (0x0800A512 with
-- its flag); the dimming holds him in the action, and 10 ticks after it he is back to idle. Since its action isn't
-- the dimming chips', it cuts in on nothing itself (0x0801454C tests the action). The controller sends out a sight
-- for its user to stop on a panel, and a satellite fires on that panel and the four beside it (./satellite). Its
-- damage is by his level (the chips' damage formula 54: the damage rows' row 5, 100 at every level).
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

local satelity: Chip

local action = new.action {
    state = { timer = "u16" },
    -- 0x080F0DFC
    update = function(me: Object, s: State)
        local step = me.step
        if step == CALL then
            -- 0x080F0E18: the controller, with the attack's element, damage word and bonus; its telop names the
            -- chip.
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
            local c = controller.spawn(me, me.attack_element, damage, me.extra)
            dimming.start(me.alliance, true, c, me, { chip = satelity, bonus = me.extra })
            me.step, me.step_init = LEAVE, 0
        elseif step == LEAVE then
            -- 0x080F0E44
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
            error(string.format("SearchMan's Satelity's step %#x reads past its table (0x080F0E10)", step))
        end
    end,
}

satelity = {
    codes = { "S" },
    element = "null",
    family = "cursor",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = navi_level.row({ 100, 100, 100, 100, 100, 100, 100 }),
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = action,
} :: Chip

return { satelity = satelity }
