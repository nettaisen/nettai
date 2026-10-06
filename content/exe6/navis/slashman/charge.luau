--!strict
-- SlashMan's charged attack (`slashman/charge`): the original's weapon
-- routine 0x43 (`sub_8012228`) and his action table's entry 9
-- (`sub_80F0C48`). He slashes three sword waves out (chips/slashman/wave,
-- his own AI's kind, which hits its panel as it winds up and waits a
-- dimming out): from the panel ahead, and from the panels above and below
-- his own; 15 ticks, then 30 more.

local link = require("../../lib/link_chips")
local wave = require("../../chips/slashman/wave")
local weapon = require("../../lib/weapon")

export type State = { timer: number }

local charge = {}

local SLASH, RECOVER = 0x0, 0x4
local ANIM_SLASH = 0xC
local SLASH_TICKS, RECOVER_TICKS = 0xF, 0x1E

-- The hits' counter byte, and the waves' element: Null (the attack's
-- element byte).
local HIT_PARAM = 0x8A
local NULL = 0

-- `sub_8012228`: 10 damage and 10 more per point of buster Attack (up to
-- 5).
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0xA, 0xA)
    navi.hit_param = HIT_PARAM
    navi.attack_element = NULL
    return charge.action
end

-- What the first wave finds in the register that becomes its Z: the
-- battle state's address, which the navi's update leaves there before it
-- runs the action (its dimming test). The wave keeps its low half.
local CALLER_REGISTER = 0x0203_4880

-- Where the waves start (`sub_80F0E3A`): the panel ahead, then the panels
-- above and below his own; {dx toward his front, dy}.
local PANELS = { { 1, 0 }, { 0, -1 }, { 0, 1 } }

-- `sub_80F0E3A`: a wave from each of those panels that is on the field.
-- (Each wave's Z until its init, whose fraction it keeps, is its spawner's
-- register: after a wave, that wave's side and flip halfword, or the
-- element if the pool was full.)
local function waves(me: Object, r3: number)
    local damage = link.damage_word(me)
    for _, d in PANELS do
        local x, y = me.panel_x + d[1] * me.facing, me.panel_y + d[2]
        if field.valid(x, y) then
            local o = wave.spawn(me, x, y, me.attack_element, wave.plain, damage, bit32.band(r3, 0xFFFF))
            r3 = if o then bit32.bor(me.alliance, bit32.lshift(me.flip, 8)) else me.attack_element
        end
    end
end

-- `sub_80F0C64`: the slash and its waves; 15 ticks.
local function slash(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_SLASH
        s.timer = SLASH_TICKS
        waves(me, CALLER_REGISTER)
    elseif link.run_out(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F0C8C`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        me.anim = 0
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [SLASH] = slash, [RECOVER] = recover }

charge.action = new.action {
    id = "slashman/charge/action",
    state = { timer = "u16" },
    -- `sub_80F0C48` (`off_80F0C5C`).
    update = function(me: Object, s: State)
        link.run(me, s, PHASES, "SlashMan's charge (sub_80F0C48)")
    end,
}

charge.weapon = new.weapon {
    id = "slashman/charge",
    -- The charge table's row 0x43, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 40, 35, 30, 25, 20, 110, 100, 90 },
    setup = setup,
}

return charge
