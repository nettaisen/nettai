--!strict
-- DustMan's charged attack (`dustman/charge`): the original's weapon
-- routine 0x49 (`sub_80122C2`) and his action table's entry 9
-- (`sub_80F1F18`). He rolls a ball of junk out from his own panel
-- (DustCross's junk ball, navis/megaman/forms/dustcross, the kind that
-- stands still while dimmed); 35 ticks, then 30 more.

local junk_ball = require("../megaman/forms/dustcross/junk_ball")
local link = require("../../lib/link_chips")
local weapon = require("../../lib/weapon")

export type State = { timer: number }

local charge = {}

local ROLL, RECOVER = 0x0, 0x4
local ANIM_ROLL = 0x17
local ROLL_TICKS, RECOVER_TICKS = 0x23, 0x1E

local SOUND = asset.sound("junk-shot")

-- The hits' counter byte, and the ball's element: Null (the attack's
-- element byte).
local HIT_PARAM = 0x8A
local NULL = 0

-- `sub_80122C2`: 60 damage and 20 more per point of buster Attack (up to
-- 5).
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x3C, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = NULL
    return charge.action
end

-- `sub_80F1F34`: the ball; 35 ticks. (The routine also loads the
-- registers for an effect at his position, and spawns none.)
local function roll(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_ROLL
        junk_ball.spawn(me, me.panel_x, me.panel_y, me.attack_element, link.damage_word(me))
        battle.play_sound(SOUND)
        s.timer = ROLL_TICKS
    elseif link.run_out(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F1F7C`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [ROLL] = roll, [RECOVER] = recover }

charge.action = define.action {
    id = "dustman/charge/action",
    state = { timer = "u16" },
    -- `sub_80F1F18` (`off_80F1F2C`).
    update = function(me: Object, s: State)
        link.run(me, s, PHASES, "DustMan's charge (sub_80F1F18)")
    end,
}

charge.weapon = define.weapon {
    id = "dustman/charge",
    name = "Charge",
    -- The charge table's row 0x49, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 100, 90, 80, 75, 70, 130, 120, 110 },
    setup = setup,
}

return charge
