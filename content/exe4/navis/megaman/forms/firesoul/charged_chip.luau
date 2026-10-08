--!strict
-- FireSoul's charged fire chips (`firesoul/charged-chip`): a fire chip held with A charges (0x0800BC78: its family
-- alone), the charge times weapon routine 0x66's row (the routine itself does nothing, 0x0800CCD0); its charged use
-- (0x0800B76C: soul 5's) isn't the chip's: 0x0800D638 sets a flame of 150 (fire, counter byte 0xA8, the attack's chip
-- 0, every other attack variable 0, the parameters 0x1E05) and starts it (action 0x19, ./flame), its flames five
-- panels; the chip is used up all the same (the hand moves on past it).

local flame = require("./flame")

local charged_chip = {}

charged_chip.action = flame.action { id = "firesoul/charged-chip/action", count = 5 }

-- 0x0800D638.
local function setup(navi: Object): Action
    navi.attack_element = 1
    navi.attack_variant = 0
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.attack_chip = nil
    navi.hit_param = 0xA8
    navi.attack_damage = 150
    return charged_chip.action
end

charged_chip.weapon = new.weapon {
    id = "firesoul/charged-chip",
    -- The charge table's row 0x66 (0x080180B8 by AIData +0x0E), by Charge 0 to 4; Charge 5 reads on into row 0x67's
    -- first entry.
    charge_ticks = { 50, 50, 50, 50, 50, 60 },
    setup = setup,
}

return charged_chip
