--!strict
-- ChargeMan's charged attack (`chargeman/charge`): the original's weapon
-- routine 0x48 (`sub_80122AA`) and his action table's entry 9
-- (`sub_80F1198`). Invulnerable and harmless to touch, he charges along
-- his row at 6 pixels a tick toward the far edge, hitting each panel he
-- reaches; as he passes his first two panel centers a train car
-- (chips/chrgeman/car) sets off behind him from the panel he left. Where
-- the floor ends he stops and his cars burst; then he is back on his
-- panel.

local car = require("../../chips/chrgeman/car")
local collision = require("../../rules/collision")
local link = require("../../lib/link_chips")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")
local weapon = require("../../lib/weapon")

export type State = {
    timer: number,
    -- His cars so far (the original's first attack parameter and the two
    -- words at +0x30).
    cars: number,
    car1: Object?,
    car2: Object?,
}

local charge = {}

local CHARGE, STOP, BACK, RECOVER = 0x0, 0x4, 0x8, 0xC
local ANIM_CHARGE, ANIM_STOP, ANIM_BACK = 0xF, 4, 3
local STOP_TICKS, BACK_TICKS, RECOVER_TICKS = 3, 3, 0x1E

local SOUND = asset.sound("charge-train")

-- The hits' counter byte, and his hits' element: Fire (the attack's
-- element byte).
local HIT_PARAM = 0x8A
local FIRE = 1

local SPEED = 0x6_0000
local PANEL_WIDTH = 0x28_0000
-- The column he charges toward, by side.
local EDGE = { [0] = 7, 0 }
local MAX_CARS = 2
local HIT_MOD = 1
-- His body's hit modifier once he is back.
local BODY_HIT_MOD = 3

-- `sub_80122AA`: 20 damage and 10 more per point of buster Attack (up to
-- 5).
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x14, 0xA)
    navi.hit_param = HIT_PARAM
    navi.attack_element = FIRE
    return charge.action
end

-- `sub_80F146E`: his speed, and the ticks to the edge column.
local function aim(me: Object, s: State)
    me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
    s.timer = int.tdiv(math.abs(EDGE[me.alliance] - me.panel_x) * PANEL_WIDTH, SPEED)
end

-- `sub_80F14BC`: a car from the panel he left (his own, with no wait and
-- no action to watch), at his speed; two at most. (Its Z until its init,
-- whose fraction it keeps, is its spawner's register: the speed.)
local function add_car(me: Object, s: State)
    local n = s.cars
    if n >= MAX_CARS then
        return
    end
    local o = car.spawn(
        me,
        me.future_panel_x,
        me.future_panel_y,
        0,
        { place = n, wait = 0 },
        link.damage_word(me),
        SPEED,
        SPEED
    )
    if n == 0 then
        s.car1 = o
    else
        s.car2 = o
    end
    s.cars = n + 1
end

-- `sub_80F149C`: a breaking hit on his panel.
local function hit(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.attack_element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.breaking,
        target = collision.hits_navis,
        self_type = collision.breaking,
        damage = link.damage(me),
        stamina = link.stamina(me),
        hit_mod = HIT_MOD,
    })
end

-- `sub_80F11BC`: the charge. Its entry holds his panel and makes him
-- invulnerable and harmless; each tick after moves him, a car setting off
-- as he passes a panel's center, a hit on each new panel; he stops where
-- the floor ends, and turns back at the edge.
local function run(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me:set_status("moving", true)
        me:set_status("using_action", true)
        -- object_setInvulnerableTime(0xFFFF)
        me:set_status_timer("invulnerable", 0xFFFF)
        me:set_status("invulnerable", true)
        -- sub_801A082
        me:reset_collision_types(collision.navi, collision.navi_target, 0)
        me.collision.self_damage = 0
        me.collision.counter_byte = 0
        me.anim = ANIM_CHARGE
        s.cars = 0
        s.car1, s.car2 = nil, nil
        battle.play_sound(SOUND)
        aim(me, s)
        return
    end
    local center = field.center(me.panel_x, me.panel_y)
    local column = me.panel_x
    local p = me.pos
    local old = p.x
    me.pos = Vec3.new(old + me.vel.x, p.y, p.z)
    if center ~= old and link.between(me.pos.x, old, center) then
        add_car(me, s)
    end
    local x, y = link.panel_at(me.pos.x, me.pos.y)
    if field.valid(x, y) and not field.solid(x, y) then
        link.go(me, STOP)
        return
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if me.panel_x ~= column then
        hit(me)
    end
    if link.run_out(s) then
        link.go(me, BACK)
    end
end

-- `sub_80F1500`: his cars burst.
local function burst_cars(s: State)
    if s.cars >= 1 and s.car1 then
        car.burst(s.car1)
    end
    if s.cars >= 2 and s.car2 then
        car.burst(s.car2)
    end
end

-- `sub_80F1284`: stopped where the floor ends: his cars burst; 3 ticks.
local function stop(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_STOP
        burst_cars(s)
        s.timer = STOP_TICKS
    elseif link.run_out(s) then
        link.go(me, BACK)
    end
end

-- `sub_80F12AC`: back on his panel, as he was; 3 ticks.
local function back(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me:set_status("moving", false)
        me:set_status("using_action", false)
        -- sub_800EB08
        me:set_status_timer("invulnerable", 0)
        me:set_status("invulnerable", false)
        -- sub_801A082, then his body's damage again (the game stores the
        -- damage's low byte as the counter byte).
        me:reset_collision_types(collision.navi, collision.navi_target, BODY_HIT_MOD)
        me.collision.self_damage = me.damage
        me.collision.counter_byte = bit32.band(me.damage, 0xFF)
        me.anim = ANIM_BACK
        s.timer = BACK_TICKS
    elseif link.run_out(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F1310`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        me:exit_attack()
    end
end

local PHASES = { [CHARGE] = run, [STOP] = stop, [BACK] = back, [RECOVER] = recover }

charge.action = define.action {
    id = "chargeman/charge/action",
    state = { timer = "u16", cars = "u8", car1 = "object", car2 = "object" },
    -- `sub_80F1198` (`off_80F11AC`).
    update = function(me: Object, s: State)
        link.run(me, s, PHASES, "ChargeMan's charge (sub_80F1198)")
    end,
}

charge.weapon = define.weapon {
    id = "chargeman/charge",
    name = "Charge",
    -- The charge table's row 0x48, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 120, 110, 100, 95, 90, 100, 90, 80 },
    setup = setup,
    -- (Transitional: the routine number the navi's row names it by.)
    legacy = { routines = { 0x48 } },
}

return charge
