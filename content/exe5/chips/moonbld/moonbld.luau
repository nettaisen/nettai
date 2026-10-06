--!strict
-- EXE5's MoonBlds (action 0x53, 0x080EF78A: EXE5's own action, EXE6's MoonBld swing, `sub_80EE2A0`, with EXE5's Katana
-- step): the navi spins once with a blade (the shared moon blade, @exelib/moonbld/blade: EXE5's attack object #0x69,
-- 0x080D31C4, in EXE5's sprite and collision rows) that sweeps the eight panels around it from the swing's fourth
-- tick, a slash of the chip's damage with bug 0x18. A charged MoonBld steps first and back after (lib/stepsword),
-- marked moving from the step until the action ends, and leaves an afterimage on the swing's sixth tick.

local collision = require("../../rules/collision")
local common = require("@exelib/moonbld/blade")
local stepsword = require("../../lib/stepsword")

local moonbld = {}

export type State = {
    -- Ticks left in the swing (AIAttackVars+0x10). (The swings left are the navi's `attack_count`.)
    timer: number,
    -- It turned round to step behind the enemy (+0x34).
    turned: number,
}

moonbld.STATE = { timer = "u16", turned = "u32" }

-- The blade (attack object #0x69): sprite 10-38, EXE5's rows 7 and 5 (0x0705FF01).
moonbld.blade = common.make_kind("moonbld/blade", {
    sprite = asset.sprite("moon-blade"),
    is = collision.slash,
    hits = collision.hits_navis,
})

-- The phases, on the attack's step.
local START, SET_UP, SWING, STEP_BACK, RECOVER = 0x0, 0x4, 0x8, 0xC, 0x10

local ANIM_SPIN = 0x0C
local SOUND = asset.sound("spin")
-- The swing's ticks, the tick (timer value) a charged swing leaves its afterimage, and the one the blade comes out.
local SWING_TICKS, AFTERIMAGE_AT, BLADE_AT = 0x15, 0x10, 0x12
-- The swing routine's address (with the Thumb bit), which the phase's dispatch leaves in a register: the blade's
-- spawner leaves it in the blade's X and its low byte in its panel Y (@exelib/moonbld/blade).
-- (The address is Team ProtoMan US's ROM's; compat/games.toml has the other ROMs'.)
local SWING_ROUTINE = 0x080EF87F

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

moonbld.action = new.action {
    state = moonbld.STATE,
    update = function(me: Object, s: State)
        local step = me.step
        if step == START then
            -- 0x080EF7B0: straight to the swing, or for a charged MoonBld, the step.
            stepsword.start(me, s, true)
        elseif step == SET_UP then
            -- 0x080EF870: one swing.
            me.attack_count = 1
            me.marker = 0
            set_step(me, SWING)
        elseif step == SWING then
            -- 0x080EF87E: the spin (22 ticks); the blade on the timer's 0x12; again while swings are left.
            if me.step_init == 0 then
                me:set_animation(ANIM_SPIN)
                me:open_counter_window()
                battle.play_sound(SOUND)
                me:set_status("using_action", true)
                s.timer = SWING_TICKS
                me.step_init = 4
            end
            if s.timer == AFTERIMAGE_AT and me.charged ~= 0 then
                stepsword.afterimage(me, ANIM_SPIN)
            end
            if s.timer == BLADE_AT then
                -- The damage word (the hit parameter in its high half), plus the Atk+ bonus.
                local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
                common.spawn(moonbld.blade, me, bit32.band(me.extra, 0xFF), SWING_ROUTINE, damage, false)
            end
            local t = s.timer - 1
            s.timer = t
            if t >= 0 then
                return
            end
            local left = me.attack_count - 1
            me.attack_count = left
            if left > 0 then
                me.related1 = nil
                me.overlay = nil
                me.step_init = 0
                return
            end
            stepsword.after_swings(me)
        elseif step == STEP_BACK then
            -- 0x080EF938: back where it stepped from once its animation ends.
            stepsword.step_back(me, s)
        elseif step == RECOVER then
            -- 0x080EF974: no longer moving; back to idle.
            me:set_status("moving", false)
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        else
            error(string.format("MoonBld phase %#x reads past its phase table", step))
        end
    end,
}

return moonbld
