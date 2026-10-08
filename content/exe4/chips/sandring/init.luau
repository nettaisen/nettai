--!strict
-- SandRing (EXE4 chip 0x02F; its record at 0x080197EC + 0x2F * 0x2C): action 0x47 (0x080EDC5A: EXE4's own), as its
-- parameter (3) has it. The navi throws (animation 0x12, a counter window), and 6 ticks on a sand ring (./ring) flies
-- from the panel ahead, 26 pixels up, with its damage word; 11 ticks, then 11 more, and back to idle (animation 0).
-- A parameter other than 3 (which no chip passes: ./ring) would hold a ring (attachment row 0x1D, by the navi's number)
-- in animation 13, its animation one on as the ring flies.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local ring = require("@self/ring")

local ANIM, IDLE_ANIM = 0x12, 0
local TICKS, THROW_AT = 0x0A, 5
local HEIGHT = 0x1A_0000

local THROW, RECOVER = 0x0, 0x4

type State = {
    -- Ticks left in the phase (AIAttackVars +0x10).
    timer: number,
}

-- 0x080EDC78: the throw; on the timer's 5 the ring.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(ANIM)
        me:open_counter_window()
        s.timer = TICKS
        me.step_init = 4
        return
    end
    if s.timer == THROW_AT then
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        ring.spawn(me, me.panel_x + me.facing, me.panel_y, HEIGHT, me.attack_element, damage)
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me.step, me.step_init = RECOVER, 0
    end
end

-- 0x080EDD00: 11 ticks, then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me:set_status("using_action", false)
        me.anim = IDLE_ANIM
        me.related1 = nil
        me:exit_attack()
    end
end

local sandring: Chip = {
    action = new.action {
        id = "sandring/action",
        state = { timer = "u16" },
        -- 0x080EDC5A.
        update = function(me: Object, s: State)
            local step = me.step
            if step == THROW then
                throw(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("SandRing's phase %#x reads past its table (0x080EDC6C)", step))
            end
        end,
    },
    codes = { "C", "R", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 8,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

return { sandring = sandring }
