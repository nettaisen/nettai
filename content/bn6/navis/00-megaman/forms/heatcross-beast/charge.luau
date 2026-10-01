--!strict
-- HeatCross Beast's charged chip (`heatcross-beast/charge`): the original's
-- weapon routine 7 (`sub_8011BD0`) and action 0x35 (`sub_80ED8C4`). A
-- charged Null-family chip becomes a wall of flames: when the navi's
-- wind-up animation ends, fire pillars (objects/element-pillar) rise on
-- the panels ahead (the three rows of the two columns past the panel in
-- front, and that panel); the navi holds for 90 ticks, humming every 17,
-- and the flames die down once it stops.

local element_pillar = require("../../../../objects/element-pillar/element_pillar")
local regions = require("../../../../lib/regions")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- Ticks left (AIAttackVars+0x10).
    timer: number,
    -- Ticks to the next hum (AIAttackVars+0x12). The game never sets it
    -- before counting it down: it starts from whatever the attack state
    -- last held there, so the first hum's tick (a sound) can differ.
    hum: number,
}

local heat_beast_charge = {}

-- The phases, on the attack's step.
local WIND_UP, BURN = 0x0, 0x4

-- Fire (the attack's element) and the hits' counter byte.
local FIRE, HIT_PARAM = 1, 0x8A

-- The pillars: flames, 90 ticks, 4 once the navi stops, on the panel in
-- front and the two columns past it.
local PILLARS: ElementPillar = { ticks = 0x5A, after = 4 }
local REGION = regions.deep_fan
local TICKS = 0x5A

local HUM = asset.sound("roar")

-- `sub_8011BD0`: 50 damage and 30 more per point of buster Attack (up to
-- 5), Fire.
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x32, 0x1E)
    navi.hit_param = HIT_PARAM
    navi.attack_element = FIRE
    return heat_beast_charge.action
end

-- `sub_80ED8E0`: the wind-up; when its animation ends, the flames.
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
    me.step = BURN
    me.step_init = 0
end

-- `sub_80ED924`: hum every 17 ticks; after 90, idle.
local function burn(me: Object, s: State)
    local h = s.hum - 1
    s.hum = h
    if h < 0 then
        s.hum = 0x10
        battle.play_sound(HUM)
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:set_status("moving", false)
        me:exit_attack()
    end
end

heat_beast_charge.action = define.action {
    id = "heatcross-beast/charge/action",
    state = { timer = "u16", hum = "u16" },
    -- `sub_80ED8C4`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == WIND_UP then
            wind_up(me, s)
        elseif step == BURN then
            burn(me, s)
        else
            error(string.format("HeatCross Beast charge phase %#x reads past its phase table", step))
        end
    end,
}

heat_beast_charge.weapon = define.weapon {
    id = "heatcross-beast/charge",
    name = "HeatCross Beast charge",
    -- The charge table's row 7; Charge 5 reads on into row 8's first
    -- entry, as the original's table does.
    charge_ticks = { 90, 90, 90, 90, 90, 20 },
    setup = setup,
    -- (Transitional: the routine number the pack's forms name it by.)
    legacy = { routines = { 0x07 } },
}

return heat_beast_charge
