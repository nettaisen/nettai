--!strict
-- GroundMan's charged attack (`groundman/charge`): the original's weapon
-- routine 0x4A (`sub_80122DA`) and his action table's entry 9
-- (`sub_80F1BA8`). He throws three drills ahead (./drill), one from each
-- panel of the column in front that is on the field: the middle one at
-- once, the upper after 12 ticks, the lower after 24. He holds 70 ticks
-- (the drills last only while he does), then 20 more.

local drill = require("./drill")
local link = require("../../lib/link_chips")
local weapon = require("../../lib/weapon")

export type State = { timer: number }

local charge = {}

local THROW, RECOVER = 0x0, 0x4
local ANIM_THROW, ANIM_RECOVER = 0xF, 7
local THROW_TICKS, RECOVER_TICKS = 0x46, 0x14

local SOUND = asset.sound("drill-launch")

-- The hits' counter byte, and the drills' element: Null (the attack's
-- element byte).
local HIT_PARAM = 0x8A
local NULL = 0

-- The drills, upper row first (`byte_80F1E48`): the row, whether it is a
-- side drill, the ticks it waits.
local DRILLS = { { dy = -1, side = true, wait = 0xC }, { dy = 0, side = false, wait = 0 }, { dy = 1, side = true, wait = 0x18 } }

-- `sub_80122DA`: 10 damage and 10 more per point of buster Attack (up to
-- 5).
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0xA, 0xA)
    navi.hit_param = HIT_PARAM
    navi.attack_element = NULL
    return charge.action
end

-- `sub_80F1E08`: the drills. (Each one's Z until its init, whose fraction
-- it keeps, is its spawner's register: twice its place in the table.)
local function drills(me: Object)
    local damage = link.damage_word(me)
    local x = me.panel_x + me.facing
    for i, d in DRILLS do
        local y = me.panel_y + d.dy
        if field.valid(x, y) then
            drill.spawn(me, x, y, me.attack_element, d.side, d.wait, damage, (i - 1) * 2)
        end
    end
end

-- `sub_80F1BC4`: the drills; 70 ticks.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_THROW
        drills(me)
        battle.play_sound(SOUND)
        s.timer = THROW_TICKS
    elseif link.run_out(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F1BF4`: 20 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_RECOVER
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        me.anim = 0
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [THROW] = throw, [RECOVER] = recover }

charge.action = define.action {
    id = "groundman/charge/action",
    state = { timer = "u16" },
    -- `sub_80F1BA8` (`off_80F1BBC`).
    update = function(me: Object, s: State)
        link.run(me, s, PHASES, "GroundMan's charge (sub_80F1BA8)")
    end,
}

charge.weapon = define.weapon {
    id = "groundman/charge",
    name = "Charge",
    -- The charge table's row 0x4A, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 130, 120, 110, 105, 100, 0, 0, 0 },
    setup = setup,
}

return charge
