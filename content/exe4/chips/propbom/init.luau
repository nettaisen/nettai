--!strict
-- PropBom1, PropBom2 and PropBom3 (EXE4 chips 0x09D to 0x09F; their records at 0x080197EC + n * 0x2C): action 0x62
-- (0x080EE532: EXE4's own). The navi (animation 0x12, a counter window) waits 5 ticks, then with the spreader's sound
-- sends a propeller bomb (./bomb) from the panel ahead, 20 pixels up (0x080EE5BC, by the navi's number: 20 for every
-- navi), flying at the chip's speed (its parameters' second byte, 0x18: 1.5 pixels a tick); then 31 ticks, and back
-- to idle.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bomb = require("@self/bomb")

local ANIM = 0x12
local WAIT_TICKS, RECOVER_TICKS = 4, 0x1E
local HEIGHT = 0x14_0000
local SPEED = 0x18
local SOUND = asset.sound("spreader")

type State = { timer: number }

-- 0x080EE554: the pose (its animation alone, its load not forced); 5 ticks.
local function wait(me: Object, s: State)
    if me.step_init == 0 then
        me.anim = ANIM
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = WAIT_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080EE582: the bomb out.
local function send(me: Object)
    battle.play_sound(SOUND)
    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    bomb.spawn(me, me.panel_x + me.facing, me.panel_y, HEIGHT, me.attack_element, damage, SPEED)
    me.step, me.step_init = 8, 0
end

-- 0x080EE5CC: 31 ticks, then back to idle.
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
        state = { timer = "u16" },
        -- 0x080EE532.
        update = function(me: Object, s: State)
            local step = me.step
            if step == 0 then
                wait(me, s)
            elseif step == 4 then
                send(me)
            elseif step == 8 then
                recover(me, s)
            else
                error(string.format("PropBom's phase %#x reads past its table (0x080EE548)", step))
            end
        end,
    }
end

local propbom1: Chip = {
    action = action("propbom1/action"),
    codes = { "E", "G", "P", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 18,
    damage = 120,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
}

local propbom2: Chip = {
    action = action("propbom2/action"),
    codes = { "F", "V", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 26,
    damage = 160,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
}

local propbom3: Chip = {
    action = action("propbom3/action"),
    codes = { "H", "J", "Q" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 34,
    damage = 200,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
}

return { propbom1 = propbom1, propbom2 = propbom2, propbom3 = propbom3 }
