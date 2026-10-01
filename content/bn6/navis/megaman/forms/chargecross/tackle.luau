--!strict
-- ChargeCross's charged shot (`chargecross/tackle`): the original's
-- weapon routine 0x27 (`sub_8011F44`) and action 0x56 (`sub_80EFA56`). The
-- navi winds up, then, invulnerable and harmless to touch, charges forward
-- with a hit riding in front (../../dash_hit, the tackle's). It stops on
-- leaving the field or running out of ground (unless it floats), after 11
-- ticks, or once its hit has struck (then it slows down, 13 ticks); it
-- drops back onto the panel it left.

local dash_hit = require("../../dash_hit")
local regions = require("../../../../lib/regions")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks of the phase.
    timer: number,
    -- AIAttackVars+0x12: ticks of slowing down after the hit.
    slowing: number,
}

local tackle = {}

-- The phases, on the attack's step.
local WIND_UP, CHARGE, LAND = 0x0, 0x4, 0x8

-- Fire, the hit's counter byte.
local FIRE, HIT_PARAM = 1, 0x94

-- The charge's speed, 16.16, and its ticks: 120 pixels' worth, and one
-- more (the game divides 0x780000 by the speed).
local SPEED = 0xC_0000
local CHARGE_TICKS = 0x78_0000 // SPEED + 1

-- Ticks of slowing down once the hit is gone.
local SLOWING_TICKS = 0xD

-- A panel's "solid" bit (the flags word), which the charge needs under it
-- unless it floats.
local SOLID = 0x10

-- Where the navi waits while off the field, 16.16.
local OFF_FIELD_X = 0xB4_0000

-- How high the hit rides, and the effects' height, 16.16.
local HIT_Z, EFFECT_Z = 0x18_0000, 0x10_0000

local SOUND = asset.sound("iron-shell")

-- The puff where it stops, and the dust it lands in.
local PUFF = define.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local LANDING_DUST = define.effect { sprite = asset.sprite("puff"), anim = 1, palette = 0 }

-- `sub_8011F44`: 30 damage and 20 more per point of buster Attack (up to
-- 5), Fire.
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x1E, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = FIRE
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    return tackle.action
end

-- `sub_80EFA74`: 9 ticks of wind-up; then the charge starts.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_status("moving", true)
        me:set_animation(0x14)
        me:open_counter_window()
        s.timer = 8
        me.step_init = 4
        return
    end
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    local v = me.vel
    me.vel = Vec3.new(me.facing * SPEED, v.y, v.z)
    s.timer = CHARGE_TICKS
    s.slowing = SLOWING_TICKS
    me:set_animation(0x15)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y + 0x1_0000, p.z)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
    dash_hit.spawn(me, me.attack_element, damage, 0, true, HIT_Z)
    me.collision.self_damage = 0
    -- sub_8019F86(0)
    me.collision.hit_mod_base = 0
    battle.play_sound(SOUND)
    -- object_setInvulnerableTime(0xFFFF)
    me:set_status_timer("invulnerable", 0xFFFF)
    me:set_status("invulnerable", true)
    me.step = CHARGE
    me.step_init = 0
end

-- The charge is over: a puff where it stopped, and the navi waits out of
-- sight.
local function stop(me: Object, s: State)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + EFFECT_Z), PUFF)
    me.pos = Vec3.new(OFF_FIELD_X, p.y, p.z)
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me.collision:set_region(nil)
    s.timer = 8
    me.step = LAND
    me.step_init = 0
end

-- `sub_80EFB0C`: charge on. The hit reports in the marker word while it
-- is there; once it is gone the navi slows down.
local function charge(me: Object, s: State)
    local hitting = me.marker ~= 0
    if not hitting or me.collision.hit_flags ~= 0 then
        me.collision:set_region(nil)
    end
    if not hitting then
        local v = me.vel
        me.vel = Vec3.new(int.asr(v.x, 1), v.y, v.z)
    end
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    if not hitting then
        local left = s.slowing - 1
        s.slowing = left
        if left > 0 then
            return
        end
        return stop(me, s)
    end
    me:update_collision_panels()
    local x, y = me.panel_x, me.panel_y
    if not me:status("air_shoe") and not bit32.btest(field.flags(x, y), SOLID) then
        return stop(me, s)
    end
    if not field.valid(x, y) then
        return stop(me, s)
    end
    local left = s.timer - 1
    s.timer = left
    if left > 0 then
        return
    end
    stop(me, s)
end

-- `sub_80EFBA0`: dust on the panel it left 3 ticks before it lands there,
-- back to normal.
local function land(me: Object, s: State)
    if s.timer == 3 then
        local x, y = field.center(me.future_panel_x, me.future_panel_y)
        battle.effect(Vec3.new(x, y, EFFECT_Z), LANDING_DUST)
    end
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_status("using_action", false)
    me:set_status("moving", false)
    -- sub_800EB08
    me:set_status_timer("invulnerable", 0)
    me:set_status("invulnerable", false)
    -- sub_80142C2, then sub_8019F86(sub_80107C0()): the body's damage and
    -- hit modifier in link battles.
    if battle.link() then
        me.collision.self_damage = 10
    end
    me.collision.hit_mod_base = if battle.link() then 3 else 0
    me.collision:set_region(regions.single)
    me:exit_attack()
end

tackle.action = define.action {
    id = "chargecross/tackle/action",
    state = { timer = "u16", slowing = "u16" },
    -- `sub_80EFA56`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == WIND_UP then
            wind_up(me, s)
        elseif step == CHARGE then
            charge(me, s)
        elseif step == LAND then
            land(me, s)
        else
            error(string.format("ChargeCross's charged shot phase %#x reads past its phase table (sub_80EFA56)", step))
        end
    end,
}

tackle.weapon = define.weapon {
    id = "chargecross/tackle",
    name = "ChargeCross tackle",
    -- The charge table's row 0x27, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 90, 80, 70, 65, 60, 80, 70, 60 },
    setup = setup,
}

return tackle
