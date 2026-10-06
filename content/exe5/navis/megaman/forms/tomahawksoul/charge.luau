--!strict
-- TomahawkSoul's charged shot (`tomahawksoul/charge`): the original's weapon routine 0x1A (0x0800F86C) sets 70 damage
-- and 10 more per point of buster Attack plus one (0x0800FE5E), its own counter byte and no bonus, and starts action
-- 0x45 (0x080EEE84): MegaMan swings his tomahawk (the counter window open, a war cry) and on its tenth tick strikes
-- the panel ahead and the column past it (region 0x11: a Wood slash against navis, no spark) under a big slash
-- (effect 0x45). 22 ticks, then 6 of recovery.

local collision = require("../../../../rules/collision")
local regions = require("@exelib/regions")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
}

local HIT_PARAM = 0x9E
local ANIM = 16
local SOUND = asset.sound("tomahawk-man")
local SWING_TICKS, STRIKE_AT, RECOVER_TICKS = 21, 12, 5
local HIT_MOD = 3
local WOOD = 4
-- Effect 0x45: the big slash (sprite 0C-15, animation 1), 16 pixels up.
local SLASH = define.effect { sprite = asset.sprite("big-slash"), anim = 1, palette = 0 }
local SLASH_Z = 16 * 0x1_0000

-- 0x080EEEA0: the swing; the strike on the count's 12; 22 ticks.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM)
        me:open_counter_window()
        battle.play_sound(SOUND)
        me:set_status("using_action", true)
        s.timer = SWING_TICKS
        me.step_init = 4
    end
    if s.timer == STRIKE_AT then
        local x, y = me.panel_x + me.facing, me.panel_y
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = WOOD,
            z = 0,
            region = regions.wide_long,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.slash,
            damage = me.attack_damage,
            stamina = me.hit_param,
            hit_mod = HIT_MOD,
        })
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, SLASH_Z), SLASH, bit32.bxor(me.alliance, me.flip))
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me.related1 = nil
        me.overlay = nil
        me.step, me.step_init = 4, 0
    end
end

-- 0x080EEF24: 6 ticks; then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

charge.action = define.action {
    state = { timer = "u16" },
    -- 0x080EEE84
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            swing(me, s)
        elseif step == 4 then
            recover(me, s)
        else
            error(string.format("TomahawkSoul's charged shot's step %#x reads past its table (0x080EEE98)", step))
        end
    end,
}

-- 0x0800F86C
local function setup(navi: Object): Action
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = 70 + 10 * (battle.navi(navi.alliance).attack + 1)
    return charge.action
end

charge.weapon = define.weapon {
    -- The charge table's row 0x1A (0x0801CB70); Charge 5 reads on into row 0x1B's first entry.
    charge_ticks = { 160, 150, 140, 130, 120, 20 },
    setup = setup,
}

return charge
