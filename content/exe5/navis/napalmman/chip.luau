--!strict
-- NapalmMan's own chip, Napalm (EXE5 chip 0x196), which his custom screen offers until it is picked (the navi's
-- `own_chip`): his own action 8 (0x080F1098). Its first step (0x080F10BC) adds the attack's bonus into its damage,
-- aims at the panel four columns ahead in his row (the field's edge column at most) and goes on: he raises his
-- cannon (animation 18, a counter window of 16 ticks) and 21 ticks on fires (animation 19, a sound) a napalm bomb,
-- the NapalmMan chips' (chips/napalmmn/bomb, its variant 1), from 8 pixels toward the enemy's side and 56 up; 11
-- ticks after the shot he waits 71 more and stands. Its damage is by his level (the chips' damage formula 56: the
-- damage rows' row 7).
--
-- The action's later steps are the ones his auto battle's bombing (0x080F61D0) runs: the shot's (0x080F628A),
-- which takes its target from where the attack points (here the attack's own cell) and the bomb's variant from the
-- attack, and the wait (0x080F6068).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi_level = require("../../lib/navi_level")
local bomb = require("../../chips/napalmmn/bomb")
local panels = require("@exelib/panels")

export type State = {
    -- AIAttackVars +0x10: the step's ticks left.
    timer: number,
    -- AIAttackVars +0x34: the panel aimed at (+0x30 points to it).
    target_x: number,
    target_y: number,
}

-- The steps, on the attack's step (the table's first two entries are the same routine).
local AIM, AIM_TOO, FIRE, WAIT = 0x0, 0x4, 0x8, 0xC
local ANIM_RAISE, ANIM_FIRE = 18, 19
-- The shot's ticks, the tick left at which the bomb flies, and the wait after (AIAttackVars +0x18).
local FIRE_TICKS, FIRE_AT, WAIT_TICKS = 32, 11, 70
local COUNTER_TICKS = 16
-- The bomb's fall (16.16 a tick: its variant's, AIAttackVars +0x0D, 1), and where it starts from his position (16.16).
local GRAVITY = -0xA000
local MUZZLE_X, MUZZLE_Z = 0x8_0000, 0x38_0000
local SOUND = asset.sound("cannon")

local action = new.action {
    state = { timer = "u16", target_x = "u8", target_y = "u8" },
    -- 0x080F1098
    update = function(me: Object, s: State)
        local step = me.step
        if step == AIM or step == AIM_TOO then
            -- 0x080F10BC
            me.attack_damage = bit32.band(me.attack_damage + me.extra, 0xFFFF)
            s.target_x = math.clamp(me.panel_x + me.facing * 4, 0, 7)
            s.target_y = me.panel_y
            me:set_status("using_action", true)
            me.step, me.step_init = FIRE, 0
        elseif step == FIRE then
            -- 0x080F628A
            if me.step_init == 0 then
                s.timer = FIRE_TICKS
                me.anim = ANIM_RAISE
                me:open_counter_window(COUNTER_TICKS)
                me.step_init = 4
            end
            if s.timer == FIRE_AT then
                me.anim = ANIM_FIRE
                -- 0x080F62D2
                local p = me.pos
                local from = Vec3.new(int.i32(p.x + panels.side_direction(me.alliance) * MUZZLE_X), p.y, int.i32(p.z + MUZZLE_Z))
                bomb.lob(me, from, {
                    gravity = GRAVITY,
                    target_x = s.target_x,
                    target_y = s.target_y,
                    damage = int.u32(me.attack_damage + me.hit_param * 0x10000),
                })
                battle.play_sound(SOUND)
            end
            local t = s.timer - 1
            s.timer = bit32.band(t, 0xFFFF)
            if t < 0 then
                me.step, me.step_init = WAIT, 0
                s.timer = WAIT_TICKS
            end
        elseif step == WAIT then
            -- 0x080F6068
            local t = s.timer - 1
            s.timer = bit32.band(t, 0xFFFF)
            if t < 0 then
                me.anim = 0
                me:set_status("using_action", false)
                me:exit_attack()
            end
        else
            error(string.format("NapalmMan's Napalm's step %#x reads past its table (0x080F10AC)", step))
        end
    end,
}

local chip: Chip = {
    codes = { "N" },
    element = "null",
    family = "fire",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = navi_level.row({ 100, 100, 100, 100, 100, 120, 120 }),
    hit_param = 158,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = action,
}

return { napalm = chip }
