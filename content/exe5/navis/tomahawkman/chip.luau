--!strict
-- TomahawkMan's own chip, T-Swing (EXE5 chip 0x1a2), which his custom screen offers until it is picked (rules: the
-- navi's `own_chip`), and its use, his action 8 (0x080F17D0): he raises his tomahawk for 30 ticks and brings it down
-- for 30, a war cry as it starts down; 10 ticks into the swing it hits the panel ahead and the column past it (region
-- 0x11: a slash of the chip's element, Wood, with no spark) under a big slash (effect 0x45, 16 pixels up); then 30
-- ticks of recovery. It is EXE6's TomahawkMan's charged swing but for the slash's hit modifier (3) and the fixed
-- waits. No counter window opens. Its damage is by his level (the chips' damage formula 68: the damage rows' row 19,
-- lib/navi_level). Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi_level = require("../../lib/navi_level")
local collision = require("../../rules/collision")
local marker = require("../../lib/navi_chips/marker")
local regions = require("@exelib/regions")

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
}

local RAISE_ANIM, SWING_ANIM, RECOVER_ANIM = 16, 17, 7
local RAISE_TICKS, SWING_TICKS, RECOVER_TICKS = 30, 30, 30
-- The swing's count when the slash lands.
local SLASH_AT = 20
local SOUND = asset.sound("tomahawk-man")
local HIT_MOD = 3
-- Effect 0x45: the big slash (sprite 0C-15, animation 1), 16 pixels up.
local SLASH = define.effect { sprite = asset.sprite("big-slash"), anim = 1, palette = 0 }
local SLASH_Z = 16 * 0x1_0000

-- The slash over the panel ahead and the column past it (the damage word plus the Atk+ bonus), and its effect.
local function slash(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.attack_element,
        z = 0,
        region = regions.wide_long,
        hit_effect = nil,
        target = collision.nothing,
        self_type = collision.slash,
        damage = bit32.band(word, 0xFFFF),
        stamina = bit32.rshift(word, 16),
        hit_mod = HIT_MOD,
    })
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, SLASH_Z), SLASH, bit32.bxor(me.alliance, me.flip))
end

-- 0x080F17F0: the tomahawk goes up; 30 ticks. (In the operation battle a marker over the panel ahead.)
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = RAISE_ANIM
        marker.spawn(me, me.panel_x + me.facing, me.panel_y, RAISE_TICKS, 4, true)
        s.timer = RAISE_TICKS
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080F182E: the swing, with his cry; the slash when 20 ticks are left; 30 ticks. (In the operation battle a
-- marker over the panel ahead in its region's style.)
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        battle.play_sound(SOUND)
        me.anim = SWING_ANIM
        marker.spawn(me, me.panel_x + me.facing, me.panel_y, SWING_TICKS, 0x11, true)
        s.timer = SWING_TICKS
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t == SLASH_AT then
        slash(me)
    elseif t <= 0 then
        me.step, me.step_init = 8, 0
    end
end

-- 0x080F18C8: the tomahawk comes back; 30 ticks; then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = RECOVER_ANIM
        s.timer = RECOVER_TICKS
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me:exit_attack()
    end
end

local action = define.action {
    id = "t-swing/action",
    state = { timer = "u16" },
    -- 0x080F17D0
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            raise(me, s)
        elseif step == 4 then
            swing(me, s)
        elseif step == 8 then
            recover(me, s)
        else
            error(string.format("T-Swing's step %#x reads past its table (0x080F17E4)", step))
        end
    end,
}

return define.chip {
    id = "t-swing",
    codes = { "T" },
    element = "wood",
    family = "wood",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = navi_level.row({ 100, 100, 100, 100, 100, 120, 120 }),
    hit_param = 158,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = action,
}
