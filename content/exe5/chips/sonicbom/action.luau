--!strict
-- SonicBom's action (EXE5's action 0x2B, 0x080ED622: EXE5's own, EXE6's SonicBom swing, `sub_80EF970`, with the
-- Katanas' step): the navi swings its sword (the blade in its related slot) and on the swing's tenth tick throws EXE5's
-- sonic boom (lib/swords: attack object #0x58) from the panel ahead, of the record's hit modifier (its first
-- parameter: SonicBom's 3, SprSonic's 1) and going through by its second (SprSonic's). A charged one steps first and
-- back after (lib/stepsword), its boom of hit modifier 1, leaving two afterimages as it throws. Each chip makes its
-- own (`action.make`: SonicBom's is `action.action`).

local attachment = require("../../objects/attachment")
local stepsword = require("../../lib/stepsword")
local sonic_boom = require("@exelib/swords/sonic_boom")
local swords = require("../../lib/swords")

local action = {}

export type State = {
    -- Ticks left in the swing (AIAttackVars+0x10). (The swings left are the navi's `attack_count`.)
    timer: number,
    -- It turned round to step behind the enemy (+0x34).
    turned: number,
}

action.STATE = { timer = "u16", turned = "u32" }

-- The phases, on the attack's step.
local START, SET_UP, SWING, STEP_BACK, RECOVER = 0x0, 0x4, 0x8, 0xC, 0x10

local ANIM_SWING = 5
local SWING_TICKS, THROW_TICK, RECOVER_TICKS = 0x15, 0xC, 5
-- A charged one's boom's hit modifier (its first parameter becomes 1).
local CHARGED_HIT_MOD = 1

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- The action of a chip whose parameters are `hit_mod` (its boom's hit modifier) and `pierces` (it goes through).
function action.make(hit_mod: number, pierces: boolean): Action
    return new.action {
        state = action.STATE,
        update = function(me: Object, s: State)
            local step = me.step
            if step == START then
                -- 0x080ED648.
                stepsword.start(me, s, false)
            elseif step == SET_UP then
                -- 0x080ED700: one swing.
                me.attack_count = 1
                set_step(me, SWING)
            elseif step == SWING then
                -- 0x080ED712.
                if me.step_init == 0 then
                    me:set_animation(ANIM_SWING)
                    me:open_counter_window()
                    battle.play_sound(swords.swing)
                    attachment.attach(me, swords.blade, "related", { anim = swords.blade_anim(me) })
                    me:set_status("using_action", true)
                    s.timer = SWING_TICKS
                    me.step_init = 4
                end
                if s.timer == THROW_TICK then
                    local charged = me.charged ~= 0
                    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    local x, y = me.panel_x + me.facing, me.panel_y
                    sonic_boom.throw(swords.sonic_boom, me, x, y, me.attack_element, damage, {
                        hit_mod = if charged then CHARGED_HIT_MOD else hit_mod,
                        pierces = pierces,
                    })
                    if charged then
                        stepsword.afterimage(me, ANIM_SWING)
                        battle.afterimage(me, me.pos, {
                            sprite = swords.blade.sprite,
                            anim = swords.blade_anim(me),
                            flip = stepsword.flip(me),
                            color_shader = 0xFFE0,
                            lifetime = 0x1E,
                            shadow = "with_sprite",
                        })
                    end
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
                -- 0x080ED824.
                stepsword.step_back(me, s)
            elseif step == RECOVER then
                -- 0x080ED860: 6 ticks, then the sword goes and back to idle.
                if me.step_init == 0 then
                    s.timer = RECOVER_TICKS
                    me.step_init = 4
                end
                local t = s.timer - 1
                s.timer = t
                if t < 0 then
                    me.related1 = nil
                    me.overlay = nil
                    me:exit_attack()
                end
            else
                error(string.format("SonicBom phase %#x reads past its phase table", step))
            end
        end,
    }
end

-- SonicBom's: hit modifier 3, not going through.
action.action = action.make(3, false)

return action
