--!strict
-- WindRack's action (action 0x26, 0x080ED098: EXE6's WindRack, `sub_80EE192`, but for its look and its blow): the navi
-- swings a rack with a swirl of wind around it. On the ninth tick it hits the column ahead, 16 pixels up, for 10
-- ticks, pushing (hit modifier 0x49). 22 ticks, then 6 of recovery. Unlike EXE6's: the hit is null-element whatever
-- the chip's, no gusts blow down the rows, and the rack's animation is EXE5's (0x080ED17A: a navi's or a player's AI
-- index, but 0x16's and 0x17's 0).

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local windrack = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

windrack.STATE = { timer = "u16" }

local SWING, RECOVER = 0x0, 0x4

local ANIM_SWING = 5
local SOUND = asset.sound("windrack")
-- The rack (the attachment table's row 0x10), and the swirl around the navi (effect 0x27), 16 pixels up.
local RACK = attachment.look { sprite = asset.sprite("wind-rack"), palette = 0 }
local SWIRL = define.effect { sprite = asset.sprite("swirl"), anim = 0, palette = 0 }
local HEIGHT = 0x10_0000
local SWING_TICKS, HIT_AT, RECOVER_TICKS = 0x15, 0x0C, 5
local HIT_MOD, HIT_TICKS = 0x49, 0x0A

-- 0x080ED17A: the rack's animation.
local function rack_anim(me: Object): number
    local t, ai = me.actor_type, me.ai_index
    if t == "virus" then
        return 0
    elseif t == "navi" then
        return if ai == 0x16 then 0 else ai
    end
    return if ai == 0x17 then 0 else ai
end

windrack.action = define.action {
    state = windrack.STATE,
    update = function(me: Object, s: State)
        local step = me.step
        if step == SWING then
            -- 0x080ED0B4: the swing; on the timer's 0xC the hit.
            if me.step_init == 0 then
                me:set_animation(ANIM_SWING)
                me:open_counter_window()
                battle.play_sound(SOUND)
                attachment.attach(me, RACK, "related", { anim = rack_anim(me) })
                me:set_status("using_action", true)
                local cx, cy = field.center(me.panel_x, me.panel_y)
                battle.effect(Vec3.new(cx, cy, HEIGHT), SWIRL, bit32.bxor(me.alliance, me.flip))
                s.timer = SWING_TICKS
                me.step_init = 4
            end
            if s.timer == HIT_AT then
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                local hit = battle.hitbox(me, {
                    panel_x = me.panel_x + me.facing,
                    panel_y = me.panel_y,
                    element = 0,
                    z = HEIGHT,
                    region = regions.wide,
                    hit_effect = nil,
                    target = collision.hits_navis,
                    self_type = collision.pushing,
                    damage = bit32.band(damage, 0xFFFF),
                    stamina = bit32.rshift(damage, 16),
                    hit_mod = HIT_MOD,
                })
                if hit then
                    hit.timer = HIT_TICKS
                end
            end
            local t = s.timer - 1
            s.timer = t
            if t < 0 then
                me.step, me.step_init = RECOVER, 0
            end
        elseif step == RECOVER then
            -- 6 ticks, then the rack goes and back to idle.
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
            error(string.format("WindRack phase %#x reads past its phase table", step))
        end
    end,
}

return windrack
