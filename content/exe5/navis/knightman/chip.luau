--!strict
-- KnightMan's own chip, KCrusher (EXE5 chip 0x1a5), which his custom screen offers until it is picked (rules: the
-- navi's `own_chip`), and its use, his action 8 (0x080FBEB2): a tick in he throws (the counter window open, the
-- cannon's sound): his iron ball (./crusher, attack object #0x15) leaves 40 pixels ahead of him and 42 up and flies
-- down his row through everything, the one he wears hidden; 50 ticks, then 8 as he draws it back, shown again. Its
-- damage is by his level (the chips' damage formula 70: the damage rows' row 21, lib/navi_level), with the Atk+
-- bonus. Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.
--
-- The chip's record names the attack's variant 1 (its +0x0C) and the ball's speed (+0x10: 0x50, five pixels a
-- tick); its own routine (+0x1F: routine 1 of 0x0800EC80) the ball he wears, no wait and the 50 ticks. The action's
-- variant 0 is his auto battle's: he glows 20 ticks first, his guard down, waits what the attack says, and has his
-- guard back up at the end. The action's end sets his sprite's palette 2 whatever the variant (0x080FBFE0); a
-- navi's own palette, set later in every tick (0x0800DA98), replaces it before it is drawn.

local chain = require("./chain")
local crusher = require("./crusher")

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
}

local THROW_ANIM, BACK_ANIM = 17, 19
local SOUND = asset.sound("cannon")
-- Where the ball leaves, from his place: ahead and up.
local AHEAD, UP = 40 * 0x1_0000, 42 * 0x1_0000
-- The record's first parameter: the ball's speed.
local SPEED = 0x50
-- The chip's routine's ticks after the throw, and the ticks he draws the ball back.
local HOLD_TICKS, BACK_TICKS = 50, 8
local END_PALETTE = 2

-- The steps, on the attack's step.
local START, THROW, HOLD, BACK = 0x0, 0x8, 0xC, 0x10

-- 0x080FBED8: the action is his; the chip's variant goes straight to the throw.
local function start(me: Object)
    me:set_status("using_action", true)
    me.step, me.step_init = THROW, 0
end

-- 0x080FBF46: the throw.
local function throw(me: Object)
    me:set_animation(THROW_ANIM)
    me:open_counter_window()
    chain.hide(me)
    local p = me.pos
    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    crusher.throw(me, Vec3.new(int.i32(p.x + me.facing * AHEAD), p.y, UP), SPEED, damage)
    battle.play_sound(SOUND)
    me.step, me.step_init = HOLD, 0
end

-- 0x080FBF86: his arm out, the routine's 50 ticks.
local function hold(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = HOLD_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me.step, me.step_init = BACK, 0
    end
end

-- 0x080FBFA4: he draws it back, the ball he wears shown and let go; 8 ticks; then back to idle. (The step's entry
-- flag is the low byte of the worn ball's address, which is no 0 for any actor slot but the first, a player's:
-- without the ball he would start the step over every tick.)
local function back(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(BACK_ANIM)
        s.timer = BACK_TICKS
        chain.show(me)
        chain.let_go(me)
        me.step_init = if me.related2 then 4 else 0
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t > 0 then
        return
    end
    me:set_animation(0)
    me.sprite.palette = END_PALETTE
    me:set_status("using_action", false)
    me:exit_attack()
end

local action = define.action {
    id = "kcrusher/action",
    state = { timer = "u16" },
    -- 0x080FBEB2
    update = function(me: Object, s: State)
        local step = me.step
        if step == START then
            start(me)
        elseif step == THROW then
            throw(me)
        elseif step == HOLD then
            hold(me, s)
        elseif step == BACK then
            back(me, s)
        else
            error(string.format("KCrusher's step %#x isn't one a player's reaches (0x080FBEC4)", step))
        end
    end,
}

return define.chip {
    id = "kcrusher",
    codes = { "K" },
    element = "null",
    family = "break",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = { formula = "level", by_level = { 80, 80, 80, 100, 100, 120, 120 } },
    hit_param = 158,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = action,
}
