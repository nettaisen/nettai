--!strict
-- ElecMan's charged attack (`elecman/charge`): the original's weapon
-- routine 0x44 (`sub_8012240`) and his action table's entry 9
-- (`sub_80F094C`). He calls thunder: a thunderbolt (chips/elecman/thunder,
-- the kind that cracks its panel and waits a dimming out) over every panel
-- an enemy navi stands on; 20 ticks, then 30 more with his arm down.

local link = require("../../lib/link_chips")
local panels = require("@exelib/panels")
local thunder = require("../../chips/elecman/thunder")
local weapon = require("../../lib/weapon")

export type State = { timer: number }

local charge = {}

local CALL, RECOVER = 0x0, 0x4
local ANIM_CALL, ANIM_RECOVER = 0x12, 0x13
local CALL_TICKS, RECOVER_TICKS = 0x14, 0x1E

-- The hits' counter byte, and the bolts' element: Elec (the attack's
-- element byte).
local HIT_PARAM = 0x8A
local ELEC = 3

-- The panels an enemy navi stands on, by side (`off_80F0B78`).
local ENEMIES = { [0] = 0x0400_0000, 0x0800_0000 }

-- `sub_8012240`: 40 damage and 20 more per point of buster Attack (up to
-- 5).
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x28, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = ELEC
    return charge.action
end

-- `sub_80F0B32`: a bolt over each enemy's panel, from the last found to
-- the first. (Each bolt's Z until its init, whose fraction it keeps, is
-- its spawner's register: 0 for the first, then the bolt before's side
-- and flip halfword, or the element if the pool was full.)
local function bolts(me: Object)
    local found = panels.filtered_except(me, ENEMIES[me.alliance], 0)
    local damage = link.damage_word(me)
    local r3 = 0
    for i = #found, 1, -1 do
        local bolt = thunder.spawn(me, panels.x(found[i]), panels.y(found[i]), me.attack_element, {}, damage, bit32.band(r3, 0xFFFF))
        r3 = if bolt then bit32.bor(me.alliance, bit32.lshift(me.flip, 8)) else me.attack_element
    end
end

-- `sub_80F0968`: the thunder; 20 ticks.
local function call(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_CALL
        bolts(me)
        s.timer = CALL_TICKS
    elseif link.run_out(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F0990`: 30 ticks, then idle.
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

local PHASES: { [number]: (me: Object, s: State) -> () } = { [CALL] = call, [RECOVER] = recover }

charge.action = define.action {
    id = "bn6:elecman/charge/action",
    state = { timer = "u16" },
    -- `sub_80F094C` (`off_80F0960`).
    update = function(me: Object, s: State)
        link.run(me, s, PHASES, "ElecMan's charge (sub_80F094C)")
    end,
}

charge.weapon = define.weapon {
    id = "bn6:elecman/charge",
    -- The charge table's row 0x44, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 110, 100, 90, 85, 80, 140, 130, 120 },
    setup = setup,
}

return charge
