--!strict
-- TenguMan's charged attack (`tenguman/charge`): the original's weapon
-- routine 0x42 (`sub_8012210`) and his action table's entry 9
-- (`sub_80F19D4`). He thrusts his nose out: a hit on each of the three
-- panels ahead in his row at once, the farthest with a status instead of
-- a stagger; 30 ticks, then 20 more as it draws back.

local collision = require("../../rules/collision")
local link = require("../../lib/link_chips")
local regions = require("@exelib/regions")
local statuses = require("../../rules/status")
local weapon = require("../../lib/weapon")

export type State = { timer: number }

local charge = {}

local THRUST, RECOVER = 0x0, 0x4
local ANIM_THRUST, ANIM_RECOVER = 0x16, 7
local THRUST_TICKS, RECOVER_TICKS = 0x1E, 0x14

local SOUND = asset.sound("tenguman-nose")

-- The hits' counter byte, and their element: Null (the attack's element
-- byte).
local HIT_PARAM = 0x8A
local NULL = 0

-- The hits, one, two and three panels ahead (`byte_80F1B18`): their hit
-- modifier and status.
local HITS = { { hit_mod = 3, status = nil }, { hit_mod = 3, status = nil }, { hit_mod = 0, status = statuses.paralyze_90 } }

-- `sub_8012210`: 40 damage and 10 more per point of buster Attack (up to
-- 5).
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x28, 0xA)
    navi.hit_param = HIT_PARAM
    navi.attack_element = NULL
    return charge.action
end

-- `sub_80F1ADE`: the three hits.
local function hits(me: Object)
    for ahead, h in HITS do
        battle.hitbox(me, {
            panel_x = me.panel_x + ahead * me.facing,
            panel_y = me.panel_y,
            element = me.attack_element,
            z = 0,
            region = regions.single,
            hit_effect = nil,
            target = collision.nothing,
            self_type = collision.attack,
            damage = link.damage(me),
            stamina = link.stamina(me),
            hit_mod = h.hit_mod,
            status = h.status,
        })
    end
end

-- `sub_80F19F0`: the thrust and its hits; 30 ticks.
local function thrust(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_THRUST
        hits(me)
        battle.play_sound(SOUND)
        s.timer = THRUST_TICKS
    elseif link.run_out(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F1A1E`: 20 ticks, then idle.
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

local PHASES: { [number]: (me: Object, s: State) -> () } = { [THRUST] = thrust, [RECOVER] = recover }

charge.action = define.action {
    id = "tenguman/charge/action",
    state = { timer = "u16" },
    -- `sub_80F19D4` (`off_80F19E8`).
    update = function(me: Object, s: State)
        link.run(me, s, PHASES, "TenguMan's charge (sub_80F19D4)")
    end,
}

charge.weapon = define.weapon {
    id = "tenguman/charge",
    -- The charge table's row 0x42, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 60, 55, 50, 45, 40, 40, 35, 30 },
    setup = setup,
}

return charge
