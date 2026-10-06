--!strict
-- EXE5's Katanas (action 0x54, 0x080EF98A: EXE5's own action, built on the swords' slash, EXE6's `sub_80EB776`): the
-- navi swings twice, a wide slash (the column ahead, hit modifier 1) and then a long one (two panels ahead, hit
-- modifier 3), each of the chip's damage and no element, holding the sword's blade (EXE5's lib/swords) in its
-- overlay slot.
--
-- A charged Katana steps first and back after (lib/stepsword: EXE5's step before a charged sword, EXE6's StepSwrd's).
--
-- Unlike EXE6's slash: no moving state set or cleared, two swings always, its own hits and slashes by swing
-- (0x080EFBC4, 0x080EFBE0), the blade in the overlay slot, and no recovery ticks.

local attachment = require("../../objects/attachment")
local regions = require("@exelib/regions")
local stepsword = require("../../lib/stepsword")
local swords = require("../../lib/swords")

local katana = {}

export type State = {
    -- Ticks left in the swing (AIAttackVars+0x10). (The swings left are the navi's `attack_count`.)
    timer: number,
    -- It turned round to step behind the enemy (+0x34).
    turned: number,
}

katana.STATE = { timer = "u16", turned = "u32" }

-- The phases, on the attack's step.
local START, SET_UP, SWING, STEP_BACK, RECOVER = 0x0, 0x4, 0x8, 0xC, 0x10

-- The swings' hits and slashes, the last swing first (0x080EFBC4, 0x080EFBE0): the long slash (region 2, hit
-- modifier 3, effect 0x17) and before it the wide one (region 4, hit modifier 1, effect 0x16).
local SWINGS = {
    [1] = { region = regions.long, hit_mod = 3, effect = swords.effects.long },
    [2] = { region = regions.wide, hit_mod = 1, effect = swords.effects.wide },
}

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

katana.action = define.action {
    state = katana.STATE,
    update = function(me: Object, s: State)
        local step = me.step
        if step == START then
            -- 0x080EF9B0: straight to the swings, or for a charged Katana, the step.
            stepsword.start(me, s, false)
        elseif step == SET_UP then
            -- 0x080EFA6C: two swings.
            me.attack_count = 2
            me.marker = 0
            set_step(me, SWING)
        elseif step == SWING then
            -- 0x080EFA7A: swing (22 ticks), a charged one leaving two afterimages on the 6th tick; the slash lands on
            -- the 10th.
            if me.step_init == 0 then
                me:set_animation(5)
                me:open_counter_window()
                battle.play_sound(swords.swing)
                me:refresh_form_overlay()
                -- sub_80B8E58: the overlay's attachment ends; a fresh blade.
                local old = me.overlay
                if old then
                    old:set_lifecycle("destroy")
                end
                attachment.attach(me, swords.blade, "overlay", { anim = swords.blade_anim(me) })
                me:set_status("using_action", true)
                s.timer = 0x15
                me.step_init = 4
            end
            if s.timer == 0x10 and me.charged ~= 0 then
                stepsword.afterimage(me, 5)
                battle.afterimage(me, me.pos, {
                    sprite = swords.blade.sprite,
                    anim = swords.blade_anim(me),
                    flip = stepsword.flip(me),
                    color_shader = 0xFFE0,
                    lifetime = 0x1E,
                    shadow = "with_sprite",
                })
            end
            if s.timer == 0x0C then
                local sw = SWINGS[me.attack_count]
                if not sw then
                    error(string.format("Katana swing %d reads past its tables", me.attack_count))
                end
                local x, y = me.panel_x + me.facing, me.panel_y
                local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.hitbox(me, {
                    panel_x = x,
                    panel_y = y,
                    element = 0,
                    region = sw.region,
                    hit_effect = nil,
                    target = swords.slashes_navis.hits,
                    self_type = swords.slashes_navis.is,
                    damage = bit32.band(word, 0xFFFF),
                    stamina = bit32.rshift(word, 16),
                    hit_mod = sw.hit_mod,
                    bug = 0,
                    bug_arg = 0,
                })
                local cx, cy = field.center(x, y)
                battle.effect(Vec3.new(cx, cy, 0x10 * 0x10000), sw.effect, stepsword.flip(me), 0)
            end
            local t = s.timer - 1
            s.timer = t
            if t >= 0 then
                return
            end
            local left = me.attack_count - 1
            me.attack_count = left
            if left > 0 then
                me.step_init = 0
                return
            end
            stepsword.after_swings(me)
        elseif step == STEP_BACK then
            -- 0x080EFBE4: back where it stepped from once its animation ends.
            stepsword.step_back(me, s)
        elseif step == RECOVER then
            -- 0x080EFC20: back to idle.
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        else
            error(string.format("Katana phase %#x reads past its phase table", step))
        end
    end,
}

return katana
