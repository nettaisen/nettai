--!strict
-- HeatCross Beast's charged chip: weapon routine 7 (`sub_8011BD0`) and
-- action 0x35 (`sub_80ED8C4`). A charged Null-family chip becomes a wall of
-- flames: when the navi's wind-up animation ends, fire pillars
-- (objects/element-pillar, Param1 0) rise on region 0x1A's panels ahead
-- (the three rows of the two columns past the panel in front, and that
-- panel); the navi holds for 90 ticks, humming every 17, and the flames die
-- down once it stops.

local element_pillar = require("../../../../objects/element-pillar/element_pillar")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- Ticks left (AIAttackVars+0x10).
    timer: number,
    -- Ticks to the next hum (AIAttackVars+0x12). The game never sets it
    -- before counting it down: it starts from whatever the attack state
    -- last held there, so the first hum's tick (a sound) can differ.
    hum: number,
}

local heat_beast_charge = {
    state = { timer = "u16", hum = "u16" },
}

local ACTION = 0x35

-- The phases, on the attack's step.
local WIND_UP, BURN = 0x0, 0x4

-- Fire (the attack's element) and the hits' counter byte.
local FIRE, HIT_PARAM = 1, 0x8A

-- The pillars: fire (Param1 0), 90 ticks (Param2), 4 once the navi stops
-- (Param3), on region 0x1A.
local PILLARS = { 0, 0x5A, 4, 0 }
local REGION = 0x1A
local TICKS = 0x5A

-- `sub_8011BD0`: 50 damage and 30 more per point of buster Attack (up to
-- 5), Fire.
function heat_beast_charge.setup(navi: Object): number
    navi.attack_damage = weapon.charge_damage(navi, 0x32, 0x1E)
    navi.hit_param = HIT_PARAM
    navi.attack_element = FIRE
    return ACTION
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
        battle.play_sound(0x12B)
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:set_status("moving", false)
        me:exit_attack()
    end
end

-- `sub_80ED8C4`.
function heat_beast_charge.update(me: Object, s: State)
    local step = me.step
    if step == WIND_UP then
        wind_up(me, s)
    elseif step == BURN then
        burn(me, s)
    else
        error(string.format("HeatCross Beast charge phase %#x reads past its phase table", step))
    end
end

return heat_beast_charge
