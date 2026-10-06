--!strict
-- TomahawkCross's charged shot (`tomahawkcross/charge`): the original's
-- weapon routine 0x16 (`sub_8011D38`) and action 0x4A (`sub_80EEC70`). The
-- navi winds up for 16 ticks, then swings its tomahawk: on the 10th tick
-- of the swing a one-tick Wood hit over the column in front and the one
-- past it, with a swipe there; 6 ticks after the swing's 22, idle.

local collision = require("../../../../rules/collision")
local regions = require("@exelib/regions")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks left in the phase.
    timer: number,
}

local charge = {}

-- The phases, on the attack's step.
local WIND_UP, SWING, END = 0x0, 0x4, 0x8

-- The hit parameter the setup gives the attack.
local HIT_PARAM = 0x9E

local SOUND = asset.sound("tomahawk-man")

-- The swing's hit: Wood, a slash at navis over two columns, no spark; the
-- swipe effect.
local WOOD, HIT_MOD = 4, 1
local SWIPE = new.effect { sprite = asset.sprite("big-slash"), anim = 1, palette = 0 }

-- The swing's tick (of its count down from 21) the hit lands on.
local HIT_TICK = 12

-- `sub_8011D38`: 40 damage and 20 more per point of buster Attack (up to
-- 5). (It leaves the element and variant as they were.)
local function setup(navi: Object): Action
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = weapon.charge_damage(navi, 0x28, 0x14)
    return charge.action
end

-- `sub_80EEC90`: the wind-up.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x12)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0xE
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    me:set_animation(0x13)
    s.timer = 0x15
    me.step = SWING
    me.step_init = 0
end

-- `sub_80EECCC`: the swing and its hit.
local function swing(me: Object, s: State)
    if s.timer == HIT_TICK then
        battle.play_sound(SOUND)
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
        battle.effect(Vec3.new(cx, cy, 0x10_0000), SWIPE, bit32.bxor(me.alliance, me.flip))
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    me.related1 = nil
    me.overlay = nil
    me.step = END
    me.step_init = 0
end

-- `sub_80EED30`: 6 ticks, then idle.
local function finish(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 5
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    me.related1 = nil
    me.overlay = nil
    me:exit_attack()
end

charge.action = new.action {
    id = "tomahawkcross/charge/action",
    state = { timer = "u16" },
    -- `sub_80EEC70`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == WIND_UP then
            wind_up(me, s)
        elseif step == SWING then
            swing(me, s)
        elseif step == END then
            finish(me, s)
        else
            error(string.format("TomahawkCross's charged shot phase %#x reads past its phase table (sub_80EEC70)", step))
        end
    end,
}

charge.weapon = new.weapon {
    id = "tomahawkcross/charge",
    -- The charge table's row 0x16, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 120, 110, 100, 95, 90, 50, 50, 50 },
    setup = setup,
}

return charge
