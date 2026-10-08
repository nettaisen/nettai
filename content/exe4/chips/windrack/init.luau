--!strict
-- WindRack (EXE4 chip 0x035; its record at 0x080197EC + 0x35 * 0x2C): action 0x32 (0x080ED330: EXE4's own; EXE5's
-- action 0x26 grew from it). The navi swings a rack (animation 5, its load not forced; a counter window; the rack,
-- attachment row 0x17, animating by the navi's number) with its sound (0x127) and a swirl of wind around it (effect row
-- 0x40, 16 pixels up, turned to its side); on the tenth tick of 22 it hits the column ahead, 16 pixels up, for 10
-- ticks (the region word 0x1205FF04: a wide region, no spark, pushing; hit modifier 9), null-element whatever the
-- chip's; then 6 ticks, and back to idle without the rack. EXE5's but: its rack's animation and its hit modifier (EXE5's
-- 0x49), and no arm to drop.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local collision = require("../rules/collision")
local regions = require("@exelib/regions")

local ANIM = 5
local SOUND = asset.sound("windrack")
-- The rack (attachment row 0x17) and the swirl (effect row 0x40), 16 pixels up.
local RACK = attachment.look { sprite = asset.sprite("wind-rack"), palette = 0 }
local SWIRL = new.effect { sprite = asset.sprite("swirl"), anim = 0, palette = 0 }
local HEIGHT = 0x10_0000
local SWING_TICKS, HIT_AT, RECOVER_TICKS = 0x15, 0x0C, 5
local HIT_MOD, HIT_TICKS = 9, 0x0A
-- The navi's number: the rack animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local SWING, RECOVER = 0x0, 0x4

type State = {
    -- Ticks left in the phase (AIAttackVars +0x10).
    timer: number,
}

-- 0x080ED350: the swing; on the timer's 12 the hit.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me.anim = ANIM
        me:open_counter_window()
        battle.play_sound(SOUND)
        attachment.attach(me, RACK, "related", NAVI_NUMBER)
        me:set_status("using_action", true)
        local cx, cy = field.center(me.panel_x, me.panel_y)
        battle.effect(Vec3.new(cx, cy, HEIGHT), SWIRL, me.alliance)
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
    s.timer = int.u16(t)
    if t < 0 then
        me.step, me.step_init = RECOVER, 0
    end
end

-- 0x080ED3F0: 6 ticks, then back to idle without the rack.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
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

local windrack: Chip = {
    action = new.action {
        id = "windrack/action",
        state = { timer = "u16" },
        -- 0x080ED330.
        update = function(me: Object, s: State)
            local step = me.step
            if step == SWING then
                swing(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("WindRack's phase %#x reads past its table (0x080ED344)", step))
            end
        end,
    },
    codes = { "A", "L", "R", "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 23,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { windrack = windrack }
