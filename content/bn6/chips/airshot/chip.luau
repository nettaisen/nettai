--!strict
-- AirShot: a wind shot from the panel ahead that pushes what it hits (the
-- original's action 0x21, `sub_80EC884`; docs/engine/shot-chips.md §3).
-- The navi raises its air shooter and fires on the 7th tick; 11 ticks of
-- recovery after the 11th.

local attachment = require("../../objects/attachment/attachment")
local lockon = require("../../rules/lockon")
local projectile = require("../../lib/projectile")
local variants = require("../../objects/projectile/variants")

export type State = {
    -- Ticks since the shooter came up, then the recovery's countdown.
    timer: number,
}

local STATE = { timer = "u16" }

-- The phases, on the attack's step.
local FIRE, RECOVER = 0x0, 0x4

local ANIM = 9
local SOUND = asset.sound("h-burst")
-- The shot leaves on this tick of the count, and the phase ends on this
-- one; the recovery's ticks.
local SHOT_TICK, FIRE_TICKS, RECOVER_TICKS = 5, 0xA, 0xA
local SHOT_Z = 0x18_0000

-- The air shooter (the original's attachment row 0x13).
local SHOOTER = attachment.look { sprite = asset.sprite("air-shooter"), palette = 0, attach_point = 10 }

-- `sub_80EC934`: the shooter's animation by the holder's AI index (the
-- link navis HeatMan, GroundMan and DustMan, 1, 9 and 10: 1; the others
-- 0). Unlike the cannon's it doesn't ask whether the holder is a player.
local function arm_anim(me: Object): number
    local ai = me.ai_index
    if ai == 1 or ai == 9 or ai == 10 then
        return 1
    end
    if ai > 27 then
        error(string.format("AI index %d reads past off_80EC944", ai))
    end
    return 0
end

-- `sub_80EC8A0`: raise the shooter (and its sound); fire on the count's 5;
-- the next phase at 0xA.
local function fire(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM)
        me:open_counter_window()
        s.timer = 0
        me:set_status("using_action", true)
        attachment.attach(me, SHOOTER, "overlay", { anim = arm_anim(me) })
        battle.play_sound(SOUND)
        me.step_init = 4
        return
    end
    if s.timer == SHOT_TICK then
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        projectile.spawn(me, me.panel_x + me.facing, me.panel_y, { variant = variants.air, damage = damage, z = SHOT_Z })
    end
    local t = s.timer + 1
    s.timer = t
    if t == FIRE_TICKS then
        me.step, me.step_init = RECOVER, 0
    end
end

-- `sub_80EC90E`: 11 ticks, then the shooter goes and back to idle.
local function recover(me: Object, s: State)
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
end

local action = define.action {
    state = STATE,
    -- `sub_80EC884`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == FIRE then
            fire(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("AirShot phase %#x reads past off_80EC898", step))
        end
    end,
}

return define.chip {
    id = "airshot",
    name = "AirShot",
    description = "Knock\nenmy back\n1 square",
    codes = { "*" },
    element = "null", family = "wind", class = "standard",
    rarity = 1, mb = 4, damage = 20, hit_param = 20, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    extra_flags = { "free_slot_in" },
    library = { number = 4, index = 4, sort = 5 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.cannon },
    action = action,
}
