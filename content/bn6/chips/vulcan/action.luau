--!strict
-- The Vulcans' action (the original's action 0x17, `sub_80EBF10`;
-- docs/engine/shot-chips.md §4): a burst of bullets (objects/bullet) from
-- the panel ahead, one every 11 ticks, each at a height the RNG picks. A
-- bullet hits the first thing in its row and the panel behind it.

local attachment = require("../../objects/attachment/attachment")
local bullet = require("../../objects/bullet/bullet")

local vulcan = {}

-- (The bullets left are the attack's word at AIAttackVars+0x12, the
-- navi's `attack_count`, which a later thrown obstacle waits out.)
export type State = {
    -- The phase's countdown (AIAttackVars+0x10).
    timer: number,
}

-- One state for the four vulcans.
vulcan.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local RAISE, FIRE, LOWER = 0x0, 0x4, 0x8

local ANIM_HOLD, ANIM_FIRE = 0xA, 0xD
local SOUND = asset.sound("bn6:col-army-2")
-- Ticks between bullets (less one), and of the lowering.
local INTERVAL, LOWER_TICKS = 10, 0xA
-- The bullets' heights, by the RNG's draw (`dword_80EBFF0`), 16.16.
local HEIGHTS = { 0x08_0000, 0x10_0000, 0x18_0000, 0x20_0000 }

-- The vulcan (the original's attachment row 0xD).
local VULCAN = attachment.look { sprite = asset.sprite("bn6:vulcan"), palette = 0, attach_point = 14 }

-- What a chip's vulcan fires: how many bullets, and which.
export type VulcanSpec = { shots: number, bullet: BulletVariant }
vulcan.bullets = { plain = bullet.variants.vulcan, super = bullet.variants.super_vulcan }

-- A chip's vulcan.
function vulcan.action(spec: VulcanSpec): Action
    -- `sub_80EBF30`: raise the vulcan; 2 ticks.
    local function raise(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_HOLD)
            me:open_counter_window()
            attachment.attach(me, VULCAN, "overlay")
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 1
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.step, me.step_init = FIRE, 0
        end
    end

    -- `sub_80EBF6E`: a bullet at once and every 11 ticks after, each at a
    -- height the RNG picks.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me.attack_count = spec.shots
            local arm = me.overlay
            if arm then
                arm.anim += 1
            end
            me:set_animation(ANIM_FIRE)
            s.timer = 0
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        s.timer = INTERVAL
        local z = HEIGHTS[bit32.band(battle.rng_positive(), 3) + 1]
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        bullet.fire(me, me.panel_x + me.facing, me.panel_y, z, spec.bullet, me.attack_element, damage)
        battle.play_sound(SOUND)
        local left = me.attack_count - 1
        me.attack_count = left
        if left <= 0 then
            me.step, me.step_init = LOWER, 0
        end
    end

    -- `sub_80EBFF4`: lower the vulcan; 11 ticks, then back to idle.
    local function lower(me: Object, s: State)
        if me.step_init == 0 then
            local arm = me.overlay
            if arm then
                arm.anim += 1
            end
            me:set_animation(ANIM_HOLD)
            s.timer = LOWER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        end
    end

    return define.action {
        state = vulcan.STATE,
        args = spec,
        -- `sub_80EBF10`.
        update = function(me: Object, s: State)
            local step = me.step
            if step == RAISE then
                raise(me, s)
            elseif step == FIRE then
                fire(me, s)
            elseif step == LOWER then
                lower(me, s)
            else
                error(string.format("vulcan phase %#x reads past off_80EBF24", step))
            end
        end,
    }
end

return vulcan
