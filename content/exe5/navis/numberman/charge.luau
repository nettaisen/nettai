--!strict
-- NumberMan's charged attack (`numberman/charge`): the original's weapon routine 0x3D (0x0800FAE6) sets the damage his
-- level gives (the damage rows' row 16, lib/navi_level), his own counter byte, no element and a 30-tick wait, and
-- starts his action 7 (0x080F15D8): he throws for 22 ticks, and six ticks in, with the throw's sound, a die
-- (chips/numbrman/dice) leaves 24 pixels over his place for the panel three ahead: its face drawn (1 to 6, the
-- middle faces likelier: his own table), 40 HP, hit modifier 3, a body the other side can hit, which bursts for its
-- face times his damage. Then he stands out the wait. No counter window opens, and he can be moved off it: the
-- action sets no status.

local dice = require("../../chips/numbrman/dice")
local navi_level = require("../../lib/navi_level")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
    -- AIAttackVars +0x18: the ticks he stands after the throw (the routine's).
    wait: number,
}

local HIT_PARAM = 0x94
-- The damage rows' row 16, by his level.
local DAMAGE: { number } = { 10, 10, 10, 10, 10, 20, 20 }
local THROW_ANIM = 6
local SOUND = asset.sound("minibomb-throw")
local THROW_TICKS, THROW_AT, WAIT = 22, 16, 30
-- The die leaves this high (16.16), for the panel this far ahead.
local UP, REACH = 24 * 0x1_0000, 3
-- The throw's parameters (0x00030028): the die's HP, its hits' status (none) and hit modifier.
local HP, HIT_MOD = 40, 3
-- The faces, by a draw's low four bits (0x080F1664).
local FACES = { [0] = 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 6 }

-- 0x080F15F4: the throw; the die when 16 ticks are left; 22 ticks.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = THROW_ANIM
        s.timer = THROW_TICKS
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t == THROW_AT then
        battle.play_sound(SOUND)
        local face = FACES[bit32.band(battle.rng_positive(), 15)]
        local p = me.pos
        dice.spawn(me, Vec3.new(p.x, p.y, UP), {
            face = face,
            target_x = me.panel_x + REACH * me.facing,
            target_y = me.panel_y,
            dimmed = false,
            hp = HP,
            status = nil,
            hit_mod = HIT_MOD,
            damage = int.u32(me.attack_damage + me.hit_param * 0x10000),
            holder = nil,
        })
    elseif t <= 0 then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080F1678: he stands out the routine's wait; then back to idle.
local function stand(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0
        s.timer = s.wait
        return
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t <= 0 then
        me:exit_attack()
    end
end

charge.action = define.action {
    id = "numberman/charge/action",
    state = { timer = "u16", wait = "u16" },
    -- 0x080F15D8
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            throw(me, s)
        elseif step == 4 then
            stand(me, s)
        else
            error(string.format("NumberMan's charged attack's step %#x reads past its table (0x080F15EC)", step))
        end
    end,
}

-- 0x0800FAE6
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.attack_element = 0
    navi.attack_variant = 0
    local s = navi:action_state(charge.action) :: State
    s.wait = WAIT
    return charge.action
end

charge.weapon = define.weapon {
    id = "numberman/charge",
    -- The charge table's row 0x3D (0x0801CCCE), by Charge 0 to 5: past 4 it reads on into row 0x3E's first entry.
    charge_ticks = { 130, 125, 120, 115, 110, 60 },
    setup = setup,
}

return charge
