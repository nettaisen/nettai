--!strict
-- NumberSoul's charged shot (`numbersoul/charge`): the original's weapon routine 7 (0x0800F672) sets 5 damage and 5
-- more per point of buster Attack plus one (0x0800FE5E), its own counter byte and the die's parameters (20 HP, no
-- status, hit modifier 3, a rolled face), and starts action 0x3A (0x080EE77C): MegaMan throws (the counter window
-- open, a sound) and on the tenth tick a die (objects/dice) leaves 24 pixels above him for the panel three ahead, its
-- face rolled (0x080EE820: 1 to 6, the middle faces likelier), a body the other side can hit. 22 ticks, then 31
-- standing.

local dice = require("../../../../chips/numbrman/dice")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: the step's ticks (counted up, then down).
    timer: number,
}

local HIT_PARAM = 0x94
local THROW_ANIM = 6
local SOUND = asset.sound("minibomb-throw")
local THROW_AT, THROW_TICKS, STAND_TICKS = 9, 21, 30
-- The die leaves this high (16.16), for the panel this far ahead.
local UP, REACH = 24 * 0x1_0000, 3
-- The routine's parameters (0x030014): the die's HP, its hits' status (none) and hit modifier.
local HP, HIT_MOD = 20, 3
-- The faces, by a draw's low four bits (0x080EE820).
local FACES = { [0] = 1, 2, 2, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 6, 6, 6 }

-- 0x080EE798: the throw; the die on the count's 9; 22 ticks.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(THROW_ANIM)
        battle.play_sound(SOUND)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0
        me.step_init = 4
    end
    if s.timer == THROW_AT then
        -- (AIAttackVars +3 1, which nothing sets here, would throw 9's die.)
        local face = FACES[bit32.band(battle.rng(), 15)]
        local p = me.pos
        dice.spawn(me, Vec3.new(p.x, p.y, int.i32(p.z + UP)), {
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
    end
    s.timer += 1
    if s.timer > THROW_TICKS then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080EE830: 31 ticks; then back to idle.
local function stand(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = STAND_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me:set_animation(0)
        me:exit_attack()
    end
end

charge.action = new.action {
    id = "numbersoul/charge/action",
    state = { timer = "u16" },
    -- 0x080EE77C
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            throw(me, s)
        elseif step == 4 then
            stand(me, s)
        else
            error(string.format("NumberSoul's charged shot's step %#x reads past its table (0x080EE790)", step))
        end
    end,
}

-- 0x0800F672
local function setup(navi: Object): Action
    navi.attack_damage = 5 + 5 * (battle.navi(navi.alliance).attack + 1)
    navi.hit_param = HIT_PARAM
    return charge.action
end

charge.weapon = new.weapon {
    -- The charge table's row 7 (0x0801CAB2); Charge 5 reads on into row 8's first entry.
    charge_ticks = { 130, 125, 120, 115, 110, 110 },
    setup = setup,
}

return charge
