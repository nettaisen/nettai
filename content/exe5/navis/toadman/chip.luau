--!strict
-- ToadMan's own chip, S-Melody (EXE5 chip 0x1a6), which his custom screen offers until it is picked (rules: the
-- navi's `own_chip`), and its use, his action 7 (0x080FCA36): he sings (a 10-tick counter window, the melody's
-- sound), and a chain of paralyzing Elec notes (the ToadMan chips' note, chips/toadman/note) steps from the panel
-- ahead toward its target, each note 10 ticks with the next 6 ticks in; 10 ticks, then back to idle. Its damage is by
-- his level (the chips' damage formula 72: the damage rows' row 23, lib/navi_level), with the Atk+ bonus. Its record
-- is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.
--
-- The chip's record names the notes' chain (its +0x10 to +0x12: no dimming, the next note's 6 ticks, a note's 10);
-- its own routine (+0x1F: routine 4 of 0x0800EC80) the target (0x080BC92C: ToadMan's, the enemy combatant nearest
-- the panel ahead in his cone), no wait before he sings and the 10 ticks he does.

local navi_level = require("../../lib/navi_level")
local note = require("../../chips/toadman/note")
local toadman = require("../../chips/toadman/navi")

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
    -- AIAttackVars +0x2C: what the notes head for.
    target: Object?,
    -- AIAttackVars +0x30 and +0x34: the wait before he sings, and the ticks he sings (the chip's routine's).
    wait: number,
    sing: number,
}

local READY_ANIM, SING_ANIM = 20, 21
local SOUND = asset.sound("toad-melody")
local COUNTER_TICKS = 10
-- The record's parameters (0x0A0600): no dimming, the next note 6 ticks in, each note 10 ticks.
local CHAIN: { life: number, delay: number, dimmed: boolean } = { life = 10, delay = 6, dimmed = false }
local WAIT_TICKS, SING_TICKS = 0, 10

-- The steps, on the attack's step.
local READY, SING = 0x0, 0x4

-- 0x080FCA50: ready, the action his; the wait (none: the same tick moves on).
local function ready(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(READY_ANIM)
        s.timer = s.wait
        me:set_status("using_action", true)
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me.step, me.step_init = SING, 0
    end
end

-- 0x080FCA7C: he sings: the counter window, the first note on the panel ahead, the melody; then back to idle.
local function sing(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(SING_ANIM)
        me:open_counter_window(COUNTER_TICKS)
        s.timer = s.sing
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        note.spawn(me, me.panel_x + me.facing, me.panel_y, CHAIN, s.target, damage)
        battle.play_sound(SOUND)
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me:set_status("using_action", false)
        me:exit_attack()
    end
end

local action = new.action {
    state = { timer = "u16", target = "object", wait = "u16", sing = "u16" },
    -- 0x080FCA36
    update = function(me: Object, s: State)
        local step = me.step
        if step == READY then
            ready(me, s)
        elseif step == SING then
            sing(me, s)
        else
            error(string.format("S-Melody's step %#x reads past its table (0x080FCA48)", step))
        end
    end,
}

local chip: Chip = {
    codes = { "T" },
    element = "null",
    family = "elec",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = navi_level.row({ 80, 80, 80, 80, 80, 80, 80 }),
    hit_param = 148,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = action,
    -- 0x0800EC80's routine 4 (0x0800ECCE): the target, no wait, 10 ticks singing.
    setup = function(navi: Object)
        local s = navi:action_state(action) :: State
        s.target = toadman.find_target(navi)
        s.wait = WAIT_TICKS
        s.sing = SING_TICKS
    end,
}

return { ["s-melody"] = chip }
