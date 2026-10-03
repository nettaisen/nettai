--!strict
-- SlashCross Beast's charged Null chip (`slashcross-beast/lunge`): the
-- original's weapon routine 0x1A (`sub_8011D9C`) and action 0x4C
-- (`sub_80EEE60`). The navi vanishes from its panel (which it keeps
-- reserved), appears by its lock-on target (`ho_8026554`: mode 2 in the
-- target's row, else mode 0xB in its own), slashes the panel in front (a
-- hit, and ./lunge_slash), and comes back. The ruleset runs the same
-- action for a Cross navi's knock-out (the role `cross_death`).

local collision = require("../../../../rules/collision")
local lockon = require("../../../../rules/lockon")
local lunge_slash = require("./lunge_slash")
local regions = require("../../../../lib/regions")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks of the phase.
    timer: number,
}

local lunge = {}

-- The phases, on the attack's step.
local LEAVE, APPEAR, SLASH, RETURN, RECOVER = 0x0, 0x4, 0x8, 0xC, 0x10

-- Sword (secondary element 0x80), the hit's counter byte.
local SWORD, HIT_PARAM = 0x80, 0x9E

-- The lock-on searches: beside a target in the navi's row, and in the
-- navi's row otherwise.
local SAME_ROW, OWN_ROW = lockon.thunder, lockon.beast_lunge

local SOUND = asset.sound("bn6:flame-hook-fire")

-- The slash's effect and its height, 16.16.
local SLASH_EFFECT = define.effect { sprite = asset.sprite("bn6:cross-slash"), anim = 0, palette = 0 }
local SLASH_Z = 0x10_0000

-- `sub_8011D9C`: 50 damage and 30 more per point of buster Attack (up to
-- 5), Sword.
local function setup(navi: Object): Action
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = weapon.charge_damage(navi, 0x32, 0x1E)
    navi.attack_element = SWORD
    return lunge.action
end

-- `sub_80EEFBC`: where to strike from, by the lock-on marker's panel
-- (`sub_80E164A`).
local function target(me: Object): (number, number)
    local marker = me.target_marker
    if marker == nil then
        error("SlashCross Beast's lunge reads the lock-on marker the navi doesn't have (sub_80E164A)")
    end
    local mx, my = marker.panel_x, marker.panel_y
    if my == me.panel_y then
        return me:lockon_panel(mx, my, SAME_ROW)
    end
    return me:lockon_panel(mx, me.panel_y, OWN_ROW)
end

-- `sub_80EEE88`: fade out (its panel reserved); 3 ticks later, over to
-- the target.
local function leave(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 1
        me:set_status("moving", false)
        s.timer = 3
        me:set_animation(4)
        me:reserve_panel(me.panel_x, me.panel_y)
    end
    local left = s.timer - 1
    s.timer = left
    if left > 0 then
        return
    end
    me.panel_x, me.panel_y = target(me)
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    s.timer = 0xB
    me:set_animation(3)
    me.step = APPEAR
    me.step_init = 0
end

-- `sub_80EEED4`: appear, and raise the claws.
local function appear(me: Object, s: State)
    if s.timer == 8 then
        me:set_animation(0x12)
    end
    local left = s.timer - 1
    s.timer = left
    if left > 0 then
        return
    end
    s.timer = 0x14
    me:set_status("using_action", true)
    me:set_animation(0x13)
    me.step = SLASH
    me.step_init = 0
end

-- `sub_80EEF02`: slash the panel in front at once; 20 ticks later, fade
-- out.
local function slash(me: Object, s: State)
    if s.timer == 0x14 then
        local x, y = me.panel_x + me.facing, me.panel_y
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, SLASH_Z), SLASH_EFFECT)
        local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
        lunge_slash.spawn(me, x, y, me.attack_element, damage)
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.attack_element,
            z = 0,
            region = regions.single,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.slash,
            damage = me.attack_damage,
            stamina = me.hit_param,
            hit_mod = 3,
        })
        me:open_counter_window()
        battle.play_sound(SOUND)
    end
    local left = s.timer - 1
    s.timer = left
    if left > 0 then
        return
    end
    me:set_animation(4)
    s.timer = 3
    me.step = RETURN
    me.step_init = 0
end

-- `sub_80EEF70`: back on the panel it left.
local function come_back(me: Object, s: State)
    local left = s.timer - 1
    s.timer = left
    if left > 0 then
        return
    end
    me:set_animation(3)
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me:unreserve_panel(me.panel_x, me.panel_y)
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    me:set_status("moving", false)
    me.step = RECOVER
    me.step_init = 0
    s.timer = 3
end

-- `sub_80EEFA4`: 3 ticks, then idle.
local function recover(me: Object, s: State)
    local left = s.timer - 1
    s.timer = left
    if left > 0 then
        return
    end
    me:set_status("using_action", false)
    me:exit_attack()
end

lunge.action = define.action {
    id = "bn6:slashcross-beast/lunge/action",
    state = { timer = "u16" },
    -- `sub_80EEE60`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == LEAVE then
            leave(me, s)
        elseif step == APPEAR then
            appear(me, s)
        elseif step == SLASH then
            slash(me, s)
        elseif step == RETURN then
            come_back(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("SlashCross Beast's lunge phase %#x reads past its phase table (sub_80EEE60)", step))
        end
    end,
}

lunge.weapon = define.weapon {
    id = "bn6:slashcross-beast/lunge",
    -- The charge table's row 0x1A, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 50, 50, 50, 50, 50, 40, 40, 40 },
    setup = setup,
}

return lunge
