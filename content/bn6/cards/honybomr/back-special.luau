--!strict
-- The B+Back special of the patch card honybomr
-- (`patch-cards/honybomr/back-special`): RskyHny1 with its second parameter
-- set (its first bee drags rather than flies: chips/rskyhny/action's
-- `drags`), and a 40-tick cooldown before the next B+Back. The original's
-- weapon routine 0x88 (`sub_801255C`) loads the chip into the attack
-- (`loc_80126EA`), then sets the lockout and the parameter. (It also sets
-- the attack's byte +0x1C to 3, which starting a B+Back sets anyway.)

local rskyhny = require("../../chips/rskyhny/action")
local rskyhny_chips = require("../../chips/rskyhny/chips")

-- RskyHny's action with the level-1 bees, dragging. (Its compat key maps
-- it to 0x39.)
local HONEY = rskyhny.action {
    id = "patch-cards/honybomr/back-special/action",
    bee = rskyhny.bees[1],
    drags = true,
}

local COOLDOWN = 0x28

-- `sub_801255C`.
local function setup(navi: Object): Action
    navi:load_chip_attack(rskyhny_chips[1])
    navi.attack_lockout = COOLDOWN
    return HONEY
end

return define.weapon {
    id = "patch-cards/honybomr/back-special",
    name = "RskyH",
    -- The charge table's row 0x88, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 0, 0, 0 },
    setup = setup,
}
