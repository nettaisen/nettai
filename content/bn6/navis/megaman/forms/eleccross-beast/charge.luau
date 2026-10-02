--!strict
-- ElecCross Beast's charged chip (`eleccross-beast/charge`): the original's
-- weapon routine 9 (`sub_8011C00`) and action 0x3C (`sub_80EDF5C`). A
-- charged Null-family chip becomes a field of lightning: when the navi's
-- wind-up animation ends, lightning pillars (objects/element-pillar: they
-- paralyze and crack their panels) strike the panels ahead (the panel in
-- front and the two columns past it); the navi holds for 90 ticks,
-- crackling every 17 from the second, and the lightning dies down once it
-- stops.

local element_pillar = require("../../../../objects/element-pillar/element_pillar")
local regions = require("../../../../lib/regions")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- Ticks left (AIAttackVars+0x10). (The ticks to the next crackle are
    -- the navi's `attack_count`.)
    timer: number,
}

local elec_beast_charge = {}

-- The phases, on the attack's step.
local WIND_UP, STRIKE = 0x0, 0x4

-- Elec (the attack's element) and the hits' counter byte.
local ELEC, HIT_PARAM = 3, 0x8A

-- The pillars: lightning, 90 ticks, 9 once the navi stops, on the panel in
-- front and the two columns past it.
local PILLARS: ElementPillar = { lightning = true, ticks = 0x5A, after = 9 }
local REGION = regions.deep_fan
local TICKS = 0x5A

local CRACKLE = asset.sound("beast-over")

-- `sub_8011C00`: 40 damage and 30 more per point of buster Attack (up to
-- 5), Elec.
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x28, 0x1E)
    navi.hit_param = HIT_PARAM
    navi.attack_element = ELEC
    return elec_beast_charge.action
end

-- `sub_80EDF78`: the wind-up; when its animation ends, the lightning.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x12)
        me:open_counter_window()
        me:set_status("using_action", true)
        me:set_status("moving", true)
        me.step_init = 4
        return
    end
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    me:set_animation(0x13)
    element_pillar.spawn_region(me, REGION, PILLARS)
    s.timer = TICKS
    me.attack_count = 1
    me.step = STRIKE
    me.step_init = 0
end

-- `sub_80EDFC0`: crackle every 17 ticks; after 90, idle.
local function strike(me: Object, s: State)
    local h = me.attack_count - 1
    me.attack_count = h
    if h < 0 then
        me.attack_count = 0x10
        battle.play_sound(CRACKLE)
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:set_status("moving", false)
        me:exit_attack()
    end
end

elec_beast_charge.action = define.action {
    id = "eleccross-beast/charge/action",
    state = { timer = "u16" },
    -- `sub_80EDF5C`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == WIND_UP then
            wind_up(me, s)
        elseif step == STRIKE then
            strike(me, s)
        else
            error(string.format("ElecCross Beast charge phase %#x reads past its phase table", step))
        end
    end,
}

elec_beast_charge.weapon = define.weapon {
    id = "eleccross-beast/charge",
    name = "ElecCross Beast charge",
    -- The charge table's row 9, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 110, 110, 110, 110, 110, 60, 60, 60 },
    setup = setup,
}

return elec_beast_charge
