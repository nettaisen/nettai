--!strict
-- WideSht1, WideSht2 and WideSht3 (EXE4 chips 0x009 to 0x00B; their records at 0x080197EC + n * 0x2C): action 0x31
-- (0x080ED25C: EXE4's own, its variant 0). The navi (animation 13, a counter window) raises the wide shooter
-- (attachment row 0x1C: 0c-2a, its animation twice the navi's number); 5 ticks later the shooter's animation goes one
-- on and a wave rises from the panel ahead and slides on over a column of three (attack 0x3B: @exelib/widesht/wave
-- with EXE4's look), one wave for each (the variant twice, and one); then the shooter's animation one back and 21
-- ticks, and back to idle without it. Each chip's wave is its record's parameter: 3, 4, 5, each faster (0x080D49E8).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local collision = require("../rules/collision")
local wave = require("@exelib/widesht/wave")

-- EXE4's wide wave (attack object #0x3B, 0x080D4914): 10-1d, its sound 0xE7, hitting navis.
local WAVE = wave.make_kind {
    id = "widesht/wave",
    sprite = asset.sprite("wide-wave"),
    sound = asset.sound("wide-shot"),
    target = collision.hits_navis,
}

local SHOOTER = attachment.look { sprite = asset.sprite("wide-shooter"), palette = 0 }

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local ANIM = 13
local FIRST_TICKS, INTERVAL, RECOVER_TICKS = 5, 0x0E, 0x14

type State = {
    -- The countdown to the next wave (AIAttackVars +0x10) and the waves left (+0x12).
    timer: number,
    waves: number,
}

-- A plain wide wave of `speed` (16.16 pixels a tick) that ends on its first hit.
local function wide(speed: number): WaveVariant
    return wave.variant { speed = speed, pierces = false, holds = false, trails = false, collision = collision.attack }
end

-- A chip's wide shot: its key, its wave and how many.
local function action(id: string, variant: WaveVariant, waves: number): Action
    -- 0x080ED280: the shooter up; the first wave in 5 ticks.
    local function raise(me: Object, s: State)
        me:set_animation(ANIM)
        me:open_counter_window()
        attachment.attach(me, SHOOTER, "related", 2 * NAVI_NUMBER)
        me:set_status("using_action", true)
        me.step, me.step_init = 4, 0
        s.waves = waves
        s.timer = FIRST_TICKS
    end

    -- 0x080ED2BE: a wave when the countdown ends (the first steps the shooter's animation on), 14 ticks apart.
    local function fire(me: Object, s: State)
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t > 0 then
            return
        end
        if me.step_init == 0 then
            local shooter = me.related1
            if shooter then
                shooter.anim += 1
            end
            me.step_init = 4
        end
        s.timer = INTERVAL
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        wave.fire(WAVE, me, me.panel_x + me.facing, me.panel_y, 0, variant, me.attack_element, damage, 0)
        local left = s.waves - 1
        s.waves = int.u16(left)
        if left <= 0 then
            me.step, me.step_init = 8, 0
        end
    end

    -- 0x080ED306: the shooter's animation back; 21 ticks, then back to idle without it.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            local shooter = me.related1
            if shooter then
                shooter.anim -= 1
            end
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = id,
        state = { timer = "u16", waves = "u16" },
        -- 0x080ED25C.
        update = function(me: Object, s: State)
            local step = me.step
            if step == 0 then
                raise(me, s)
            elseif step == 4 then
                fire(me, s)
            elseif step == 8 then
                recover(me, s)
            else
                error(string.format("a wide shot's phase %#x reads past its table (0x080ED274)", step))
            end
        end,
    }
end

local widesht1: Chip = {
    -- Its wave 3.
    action = action("widesht1/action", wide(0x6_0000), 1),
    codes = { "C", "D", "E", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 60,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local widesht2: Chip = {
    -- Its wave 4.
    action = action("widesht2/action", wide(0x6_8000), 1),
    codes = { "L", "M", "N" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 80,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local widesht3: Chip = {
    -- Its wave 5.
    action = action("widesht3/action", wide(0x7_0000), 1),
    codes = { "S", "T", "U" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { widesht1 = widesht1, widesht2 = widesht2, widesht3 = widesht3 }
