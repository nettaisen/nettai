--!strict
-- TwnFng1, TwnFng2 and TwnFng3 (EXE4 chips 0x045 to 0x047; their records at 0x080197EC + n * 0x2C): action 0x34
-- (0x080ED4C6: EXE4's own, its variant 0). The navi (animation 0x12, a counter window) sends two fangs (./fang) from
-- its own panel, one a row down and one a row up, then each forward along its row; then 31 ticks, and back to idle.
-- (The variant's fang pairs, seven more for each, 10 ticks apart: no chip has another.)
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local fang = require("@self/fang")

local ANIM = 0x12
local INTERVAL, RECOVER_TICKS = 0x0A, 0x1E

type State = {
    -- The countdown to the next pair (AIAttackVars +0x10) and the pairs left (+0x12).
    timer: number,
    pairs: number,
}

-- 0x080ED4E8: the pose; the first pair at once.
local function start(me: Object, s: State)
    me:set_animation(ANIM)
    me:open_counter_window()
    me:set_status("using_action", true)
    -- (The variant times 7, and one.)
    s.pairs = 1
    s.timer = 0
    me.step, me.step_init = 4, 0
end

-- 0x080ED512: a pair of fangs when the countdown ends, 10 ticks apart (their hits strong: the parameters' second
-- byte, 0); after the last, the recovery.
local function send(me: Object, s: State)
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t > 0 then
        return
    end
    s.timer = INTERVAL
    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    fang.spawn(me, me.panel_x, me.panel_y, me.attack_element, damage, false, false)
    fang.spawn(me, me.panel_x, me.panel_y, me.attack_element, damage, true, false)
    local left = s.pairs - 1
    s.pairs = int.u16(left)
    if left <= 0 then
        me.step, me.step_init = 8, 0
    end
end

-- 0x080ED554: 31 ticks, then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me:exit_attack()
    end
end

local function action(id: string): Action
    return new.action {
        id = id,
        state = { timer = "u16", pairs = "u16" },
        -- 0x080ED4C6.
        update = function(me: Object, s: State)
            local step = me.step
            if step == 0 then
                start(me, s)
            elseif step == 4 then
                send(me, s)
            elseif step == 8 then
                recover(me, s)
            else
                error(string.format("TwnFng's phase %#x reads past its table (0x080ED4DC)", step))
            end
        end,
    }
end

local twnfng1: Chip = {
    action = action("twnfng1/action"),
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 13,
    damage = 70,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local twnfng2: Chip = {
    action = action("twnfng2/action"),
    codes = { "O", "P", "Q" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 26,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local twnfng3: Chip = {
    action = action("twnfng3/action"),
    codes = { "F", "G", "H" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 39,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { twnfng1 = twnfng1, twnfng2 = twnfng2, twnfng3 = twnfng3 }
