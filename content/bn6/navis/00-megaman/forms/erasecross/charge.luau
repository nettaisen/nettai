--!strict
-- EraseCross's charged shot (`erasecross/charge`): the original's weapon
-- routine 0x14 (`sub_8011CF8`) and action 0x45 (`sub_80EE7D0`). Once the
-- navi is still, it fires its beam (./ray) from the panel in front and
-- holds the pose for 71 ticks; the beam lasts as long as the navi stays in
-- this action. In the Beast form the beam is its second ray, 4 pixels
-- lower.

local erase_ray = require("./ray")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks of the pose left.
    timer: number,
    -- The Beast form's beam (the attack's variant 1).
    beast: boolean,
}

local charge = {}

-- The phases, on the attack's step.
local FIRE, HOLD = 0x0, 0x4

-- EraseCross's form; its hit parameter.
local ERASE_CROSS, HIT_PARAM = 4, 0x94

local SOUND = asset.sound("moon-beam")

-- How much lower the Beast form's beam is, 16.16.
local BEAST_Z = -0x4_0000

-- `sub_8011CF8`: 40 damage and 20 more per point of buster Attack (up to
-- 5), Null; the beam: EraseCross's, or in any other form (its Beast form)
-- the second.
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x28, 0x14)
    navi.hit_param = HIT_PARAM
    local s = navi:action_state(charge.action) :: State
    s.beast = battle.navi(navi.alliance).form ~= ERASE_CROSS
    navi.attack_element = 0
    return charge.action
end

-- `sub_80EE7EC`: once neither moving nor sliding, the beam.
local function fire(me: Object, s: State)
    if me:status("moving") or me:status("sliding") then
        return
    end
    me:set_animation(0xF)
    me:open_counter_window()
    me:set_status("using_action", true)
    me:set_status("moving", true)
    local beast = s.beast
    local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
    erase_ray.spawn(
        me,
        me.panel_x + me.facing,
        me.panel_y,
        me.attack_element,
        if beast then BEAST_Z else 0,
        if beast then erase_ray.rays.beast else erase_ray.rays.cross,
        damage
    )
    battle.play_sound(SOUND)
    s.timer = 0x46
    me.step = HOLD
    me.step_init = 0
end

-- `sub_80ED998`: hold, then idle.
local function hold(me: Object, s: State)
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    me:set_status("moving", false)
    me:exit_attack()
end

charge.action = define.action {
    id = "erasecross/charge/action",
    state = { timer = "u16", beast = "bool" },
    -- `sub_80EE7D0`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == FIRE then
            fire(me, s)
        elseif step == HOLD then
            hold(me, s)
        else
            error(string.format("EraseCross's charged shot phase %#x reads past its phase table (sub_80EE7D0)", step))
        end
    end,
}

charge.weapon = define.weapon {
    id = "erasecross/charge",
    name = "Erase charge",
    -- The charge table's row 0x14, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 110, 100, 90, 85, 80, 120, 120, 120 },
    setup = setup,
    -- (Transitional: the routine number the pack's forms name it by.)
    legacy = { routines = { 0x14 } },
}

return charge
