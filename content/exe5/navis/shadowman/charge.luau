--!strict
-- ShadowMan's charged attack (`shadowman/charge`): the original's weapon routine 0x24 (0x0800F98E) takes the damage his
-- level gives (the damage rows' row 14, lib/navi_level) and his counter byte, the attack's variant 1 and one throw,
-- and starts his own action 9 (0x080F95F0). He vanishes from his panel in a puff (the panel stays his, and nothing
-- reaches him), is back 48 pixels over it two ticks later, and throws a shuriken (AntiDmg's, attack object #0xC2:
-- chips/antidmg) from 60 pixels above himself at the nearest enemy body ahead, his own row first; no enemy ahead,
-- no shuriken. 15 ticks a throw, 20 more up there, then he is down and stands 20 ticks.
--
-- The action is also his in an auto battle, by the attack's variant: 0 throws at the other side's player and waits
-- 60 and 50 ticks, 2 throws at a panel the attack names. (Variant 0's start also tells the two shadows the battle
-- state keeps, +0x94 and +0x98, to vanish, 0x080F9B9C: only his auto battle's own move puts shadows there,
-- 0x080F99EA, which isn't ported; SplitUp's shadow isn't one of them.)

local aim = require("./aim")
local antidmg = require("../../chips/antidmg/counter")
local effects = require("../../lib/effects")
local navi_level = require("../../lib/navi_level")
local regions = require("@exelib/regions")
local shuriken = require("@exelib/antidmg/shuriken")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
    -- AIAttackVars +0x16 and +0x17: the panel variant 2 throws at.
    target_x: number,
    target_y: number,
}

local HIT_PARAM = 0x94
-- The damage rows' row 14, by his level.
local DAMAGE: { number } = { 60, 60, 60, 80, 80, 100, 100 }
local ANIM_APPEAR, ANIM_THROW = 3, 12
local SOUND_VANISH = asset.sound("trap-vanish")
local SOUND_THROW = asset.sound("boomerang")
-- Where the puff is, how far up he goes meanwhile, where he is back, and where the shuriken leaves, over him.
local PUFF_Z, AWAY, UP, THROW_Z = 0x20_0000, 0xA0_0000, 0x30_0000, 0x3C_0000
local APPEAR_TICKS, THROW_TICKS = 3, 15
-- The waits up there and down again: variant 0's, and the others'.
local HOLD_TICKS, HOLD_TICKS_SHORT = 60, 20
local LAND_TICKS, LAND_TICKS_SHORT = 50, 20
local HIT_MOD = 1
-- The steps, on the attack's step.
local VANISH, APPEAR, THROW, HOLD, LAND = 0x0, 0x4, 0x8, 0xC, 0x10

-- Whom a throw is at, by the attack's variant.
local function target(me: Object, s: State): (number?, number?)
    local variant = me.attack_variant
    if variant == 1 then
        return aim.nearest_enemy(me)
    elseif variant == 2 then
        return s.target_x, s.target_y
    end
    -- 0x0800E148: the other side's player.
    local enemy = battle.player(bit32.bxor(me.alliance, 1))
    if enemy == nil then
        return nil, nil
    end
    return enemy.panel_x, enemy.panel_y
end

-- 0x080F9618: onto the panel he was moving to, which he keeps; a puff, and he is away. The next tick moves on (its
-- 15 ticks are never counted: the step's second tick leaves it).
local function vanish(me: Object, s: State)
    if me.step_init ~= 0 then
        me.step, me.step_init = APPEAR, 0
        return
    end
    me:set_status("sliding", false)
    me:set_status("using_action", true)
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    me:set_status("moving", true)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + PUFF_Z)), effects.bubbles)
    me.collision:set_region(nil)
    me.pos = Vec3.new(p.x, p.y, AWAY)
    battle.play_sound(SOUND_VANISH)
    s.timer = 15
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    me.step_init = 4
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me.step, me.step_init = APPEAR, 0
    end
end

-- 0x080F9696: back, 48 pixels over his panel; 3 ticks.
local function appear(me: Object, s: State)
    if me.step_init == 0 then
        me.anim = ANIM_APPEAR
        s.timer = APPEAR_TICKS
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, UP)
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me.step, me.step_init = THROW, 0
    end
end

-- 0x080F96C0: a throw, 15 ticks; one for each of the attack's count.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me.anim = ANIM_THROW
        s.timer = THROW_TICKS
        battle.play_sound(SOUND_THROW)
        local x, y = target(me, s)
        if x and y then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
            shuriken.spawn(antidmg.shuriken, me, x, y, 0, HIT_MOD, damage, me.flip, int.i32(me.pos.z + THROW_Z))
        end
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t > 0 then
        return
    end
    local left = me.attack_count - 1
    me.attack_count = int.u16(left)
    if left > 0 then
        me.step_init = 0
    else
        me.step, me.step_init = HOLD, 0
    end
end

-- 0x080F9736: still up there.
local function hold(me: Object, s: State)
    if me.step_init == 0 then
        me.anim = ANIM_THROW
        s.timer = if me.attack_variant ~= 0 then HOLD_TICKS_SHORT else HOLD_TICKS
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me.step, me.step_init = LAND, 0
    end
end

-- 0x080F9762: down on his panel, his again to hit and to leave; then back to idle.
local function land(me: Object, s: State)
    if me.step_init == 0 then
        me.anim = 0
        s.timer = if me.attack_variant ~= 0 then LAND_TICKS_SHORT else LAND_TICKS
        me:set_status("moving", false)
        me.collision:set_region(regions.single)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me:set_status("using_action", false)
        me:exit_attack()
    end
end

local STEPS: { [number]: (me: Object, s: State) -> () } = {
    [VANISH] = vanish,
    [APPEAR] = appear,
    [THROW] = throw,
    [HOLD] = hold,
    [LAND] = land,
}

charge.action = define.action {
    id = "shadowman/charge/action",
    state = { timer = "u16", target_x = "u8", target_y = "u8" },
    -- 0x080F95F0
    update = function(me: Object, s: State)
        local step = STEPS[me.step]
        if not step then
            error(string.format("ShadowMan's charged attack's step %#x reads past its table (0x080F9604)", me.step))
        end
        step(me, s)
    end,
}

-- 0x0800F98E (the count's low byte alone is written: its high byte is what an earlier action left).
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.attack_variant = 1
    navi.attack_count = bit32.bor(bit32.band(navi.attack_count, 0xFF00), 1)
    return charge.action
end

charge.weapon = define.weapon {
    id = "shadowman/charge",
    -- The charge table's row 0x24 (0x0801CBD4), by Charge 0 to 5: past 4 it reads on into row 0x25's first entry.
    charge_ticks = { 70, 65, 60, 55, 50, 60 },
    setup = setup,
}

return charge
