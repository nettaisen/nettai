--!strict
-- HeatMan's charged attack (`heatman/charge`): the original's weapon
-- routine 0x40 (`sub_80121DC`) and his action table's entry 9
-- (`sub_80F070E`). He rears up for 8 ticks, then breathes fire: a flame
-- (chips/heatman/flame) on the panel ahead and on the three of the column
-- past it, where the floor is solid, and none at all unless the panel
-- ahead is; they burn 30 ticks. 30 ticks later he idles.

local collision = require("../../rules/collision")
local flame = require("../../chips/heatman/flame")
local link = require("../../lib/link_chips")
local weapon = require("../../lib/weapon")

export type State = { timer: number }

local charge = {}

local WIND_UP, RECOVER = 0x0, 0x4
local ANIM_WIND_UP, ANIM_BREATHE = 0x15, 0x10
local WIND_UP_TICKS, RECOVER_TICKS = 8, 0x1E

-- The hits' counter byte and the attack's element, Fire; the flames: 30
-- ticks (the original's first attack parameter), attacks that wait a
-- dimming out.
local HIT_PARAM = 0x8A
local FIRE = 1
local FLAME: HeatFlame = { while_dimmed = false, ticks = 0x1E, is = collision.attack }

-- Where a flame can burn: a panel of the field with a floor.
local FLOOR = 0x10010

-- `sub_80121DC`: 20 damage and 20 more per point of buster Attack (up to
-- 5).
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x14, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = FIRE
    return charge.action
end

-- `sub_80F08BA`: the flames. (Each flame's position until its init is its
-- spawner's registers: its row, the element, 0.)
local function flames(me: Object)
    local damage = link.damage_word(me)
    local x, y = me.panel_x + me.facing, me.panel_y
    if not field.check(x, y, FLOOR, 0) then
        return
    end
    flame.spawn(me, x, y, me.attack_element, FLAME, damage, Vec3.new(y, me.attack_element, 0))
    x += me.facing
    for row = y - 1, y + 1 do
        if field.check(x, row, FLOOR, 0) then
            flame.spawn(me, x, row, me.attack_element, FLAME, damage, Vec3.new(row, me.attack_element, 0))
        end
    end
end

-- `sub_80F0728`: he rears up; 8 ticks, then the flames.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_WIND_UP
        s.timer = WIND_UP_TICKS
    elseif link.run_out(s) then
        me.anim = ANIM_BREATHE
        flames(me)
        link.go(me, RECOVER)
    end
end

-- `sub_80F0754`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        me.anim = 0
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [WIND_UP] = wind_up, [RECOVER] = recover }

charge.action = define.action {
    id = "heatman/charge/action",
    state = { timer = "u16" },
    -- `sub_80F070E` (`off_80F0720`).
    update = function(me: Object, s: State)
        link.run(me, s, PHASES, "HeatMan's charge (sub_80F070E)")
    end,
}

charge.weapon = define.weapon {
    id = "heatman/charge",
    name = "Charge",
    -- The charge table's row 0x40, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 90, 80, 70, 65, 60, 40, 35, 30 },
    setup = setup,
}

return charge
