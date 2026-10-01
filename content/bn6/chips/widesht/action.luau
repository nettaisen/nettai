--!strict
-- WideSht and SuprSpr: the navi raises its wide shooter and sends a wave
-- (wave.luau) from the panel ahead over a column of three; SuprSpr sends
-- three that go through what they hit, 14 ticks apart, the last
-- staggering harder (the original's action 0x30, `sub_80ED55C`;
-- docs/engine/shot-chips.md §10).
--
-- These are their actions; the chips are ./chip and chips/suprspr
-- (WideSht is SuprSpr's Program Advance ingredient).

local attachment = require("../../objects/attachment/attachment")
local wave = require("./wave")

local widesht = {}

export type State = {
    -- The countdown to the next wave (AIAttackVars+0x10).
    timer: number,
}

widesht.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local RAISE, FIRE, RECOVER = 0x0, 0x4, 0x8

local ANIM_HOLD = 0xA
-- The wait before the first wave, between waves, and the recovery's.
local FIRST_TICKS, INTERVAL, RECOVER_TICKS = 5, 0xE, 0x14
local WAVE_Z = 0x10_0000
-- A wave's hit modifier by the waves left (`dword_80ED618`).
local HIT_MODS = { [0] = 0, [1] = 3, [2] = 1, [3] = 1 }

-- The wide shooter (the original's attachment row 0x23).
local SHOOTER = attachment.look { sprite = asset.sprite("wide-shooter"), palette = 0, attach_point = 12 }

export type WideShotSpec = { id: string, wave: WaveVariant, waves: number }

local function action(spec: WideShotSpec): Action
    -- `sub_80ED57C`: raise the shooter; the first wave in 5 ticks.
    local function raise(me: Object, s: State)
        me:set_animation(ANIM_HOLD)
        me:open_counter_window()
        attachment.attach(me, SHOOTER, "overlay")
        me:set_status("using_action", true)
        me.step, me.step_init = FIRE, 0
        me.recovery = spec.waves
        s.timer = FIRST_TICKS
    end

    -- `sub_80ED5BE`: a wave when the countdown ends (the first raises the
    -- shooter's next animation), its hit modifier by the waves left.
    local function fire(me: Object, s: State)
        local t = s.timer - 1
        s.timer = t
        if t > 0 then
            return
        end
        if me.step_init == 0 then
            local shooter = me.overlay
            if shooter then
                shooter.anim += 1
            end
            me.step_init = 4
        end
        s.timer = INTERVAL
        -- (The game keeps it in the attack's second parameter, which the
        -- wave reads among its parameters.)
        local hit_mod = HIT_MODS[me.recovery]
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        wave.fire(me, me.panel_x + me.facing, me.panel_y, WAVE_Z, spec.wave, me.attack_element, damage, hit_mod)
        local left = me.recovery - 1
        me.recovery = left
        if left <= 0 then
            me.step, me.step_init = RECOVER, 0
        end
    end

    -- `sub_80ED61C`: the shooter's animation back; 21 ticks, then it goes
    -- and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            local shooter = me.overlay
            if shooter then
                shooter.anim -= 1
            end
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
    end

    return define.action {
        id = spec.id,
        state = widesht.STATE,
        args = spec,
        -- `sub_80ED55C`.
        update = function(me: Object, s: State)
            local step = me.step
            if step == RAISE then
                raise(me, s)
            elseif step == FIRE then
                fire(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("WideSht phase %#x reads past off_80ED570", step))
            end
        end,
    }
end

widesht.actions = {
    widesht = action { id = "widesht/action", wave = wave.variants.wide, waves = 1 },
    suprspr = action { id = "suprspr/action", wave = wave.variants.super, waves = 3 },
}

return widesht
