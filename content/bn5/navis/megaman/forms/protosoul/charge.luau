--!strict
-- ProtoSoul's charged shot (`protosoul/charge`): the original's weapon routine 3 (0x0800F60C). It is WideSwrd's
-- slash (BN5's action 0x13 as its subtype 1: the chip's own action, chips/wideswrd), not loaded as a chip, with 80
-- damage and 10 more per point of buster Attack plus one (0x0800FE5E), and its own counter byte.

local wideswrd = require("../../../../chips/wideswrd/chip")

local charge = {}

local HIT_PARAM = 0x94

-- 0x0800F60C
local function setup(navi: Object): Action
    navi.attack_element = 0
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = 80 + 10 * (battle.navi(navi.alliance).attack + 1)
    return wideswrd.action
end

charge.weapon = define.weapon {
    id = "bn5:protosoul/charge",
    -- The charge table's row 3 (0x0801CA8A); Charge 5 reads on into row 4's first entry.
    charge_ticks = { 80, 75, 70, 65, 60, 180 },
    setup = setup,
}

return charge
