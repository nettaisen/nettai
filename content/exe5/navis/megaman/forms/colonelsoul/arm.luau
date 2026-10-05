--!strict
-- ColonelSoul's charged shot with an arm chip (`colonelsoul/arm`): the original's weapon routine 0x13 (0x0800F7D8)
-- loads the turn's Arm Change chip (AIData +0x32, the navi's `weapon_chip`: ./arm-change sets it and this
-- weapon at the turn's start, 0x080124AE) as the attack (0x0800FE78: the chip's record's damage, hit parameter,
-- lockout and element; no bonus; not charged) and starts the chip's use. Its charge time is the charge table's row
-- 0x13 (0x0801CA6C), Charge 5 reading on into the next row's first entry.

local arm = {}

arm.weapon = define.weapon {
    id = "colonelsoul/arm",
    charge_ticks = { 120, 120, 120, 120, 120, 70 },
    setup = function(navi: Object): Chip
        local chip = navi.weapon_chip
        if chip == nil then
            error("ColonelSoul's arm weapon without an arm chip (AIData +0x32 0xFFFF reads past the chip table, 0x0800FE86)")
        end
        navi:load_chip_attack(chip)
        return chip
    end,
}

return arm
