--!strict
-- GroundCross Beast's charged Null chip (`groundcross-beast/dash`): the
-- original's weapon routine 0x17 (`sub_8011D54`) and action 0x1A
-- (`sub_80EC1F0`). The navi winds up, then, invulnerable and harmless to
-- touch, dashes forward faster and faster with two hits riding along
-- (../../dash_hit, on its panel and the next) until it leaves the field;
-- it drops back onto the panel it left.

local dash_hit = require("../../dash_hit")
local regions = require("@exelib/regions")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks of the phase, then of acceleration.
    timer: number,
}

local dash_weapon = {}

-- The phases, on the attack's step.
local WIND_UP, DASH, LAND = 0x0, 0x4, 0x8

-- Break (secondary element 0x10), the hit's counter byte.
local BREAK, HIT_PARAM = 0x10, 0x8A

-- The dash's first speed and its acceleration, 16.16 (the acceleration
-- kept in the attack's marker word, AIAttackVars+0x30).
local ACCELERATION = 0x1_0000

-- Where the navi waits while off the field, 16.16.
local OFF_FIELD_X = 0xDC_0000

-- How high the dash's first hit rides, 16.16.
local HIT_Z = 0x18_0000

local SOUND_DASH, SOUND_RUMBLE = asset.sound("ground-beast-dash"), asset.sound("ground-beast-dash-2")

-- The dust it lands in, 16.16 up.
local LANDING_DUST = define.effect { sprite = asset.sprite("puff"), anim = 1, palette = 0 }
local LANDING_Z = 0x1E_0000

-- `sub_8011D54`: 90 damage and 20 more per point of buster Attack (up to
-- 5), Break.
local function setup(navi: Object): Action
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = weapon.charge_damage(navi, 0x5A, 0x14)
    navi.attack_element = BREAK
    return dash_weapon.action
end

-- `sub_80EC210`: 6 ticks of wind-up; then the dash starts.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(0x15)
        me:open_counter_window()
        s.timer = 5
        me.step_init = 4
        return
    end
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    me.collision.self_damage = 0
    -- object_setInvulnerableTime(0xFFFF)
    me:set_status_timer("invulnerable", 0xFFFF)
    me:set_status("invulnerable", true)
    local accel = me.facing * ACCELERATION
    local v = me.vel
    me.vel = Vec3.new(accel, v.y, v.z)
    me.marker = int.u32(accel)
    s.timer = 0xB
    me:set_animation(0x16)
    battle.play_sound(SOUND_DASH)
    battle.play_sound(SOUND_RUMBLE)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y + 0x1_0000, p.z)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
    dash_hit.spawn(me, me.attack_element, damage, 0, false, HIT_Z)
    -- (The second's height is what the first spawn left in its register:
    -- the attack's element byte, a fraction of a pixel.)
    dash_hit.spawn(me, me.attack_element, damage, 1, false, me.attack_element)
    me.step = DASH
    me.step_init = 0
end

-- `sub_80EC2A4`: dash on, speeding up for 11 ticks; once off the field,
-- wait out of sight.
local function dash(me: Object, s: State)
    if me.collision.hit_flags ~= 0 then
        me.collision:set_region(nil)
    end
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if s.timer ~= 0 then
        s.timer -= 1
        local v = me.vel
        me.vel = Vec3.new(v.x + int.i32(me.marker), v.y, v.z)
    end
    if not field.valid(me.panel_x, me.panel_y) then
        local q = me.pos
        me.pos = Vec3.new(OFF_FIELD_X, q.y, q.z)
        s.timer = 8
        me.step = LAND
        me.step_init = 0
    end
end

-- `sub_80EC2EA`: dust on the panel it left 3 ticks before it lands there,
-- back to normal.
local function land(me: Object, s: State)
    if s.timer == 3 then
        local x, y = field.center(me.future_panel_x, me.future_panel_y)
        battle.effect(Vec3.new(x, y, LANDING_Z), LANDING_DUST)
    end
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me:unreserve_panel(me.panel_x, me.panel_y)
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_status("using_action", false)
    -- sub_800EB08
    me:set_status_timer("invulnerable", 0)
    me:set_status("invulnerable", false)
    -- sub_80142C2
    if battle.link() then
        me.collision.self_damage = 10
    end
    me.collision:set_region(regions.single)
    me:exit_attack()
end

dash_weapon.action = define.action {
    id = "groundcross-beast/dash/action",
    state = { timer = "u16" },
    -- `sub_80EC1F0`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == WIND_UP then
            wind_up(me, s)
        elseif step == DASH then
            dash(me, s)
        elseif step == LAND then
            land(me, s)
        else
            error(string.format("GroundCross Beast's dash phase %#x reads past its phase table (sub_80EC1F0)", step))
        end
    end,
}

dash_weapon.weapon = define.weapon {
    id = "groundcross-beast/dash",
    -- The charge table's row 0x17; Charge 5 reads on into row 0x18's first
    -- entry, as the original's table does.
    charge_ticks = { 50, 50, 50, 50, 50, 60 },
    setup = setup,
}

return dash_weapon
