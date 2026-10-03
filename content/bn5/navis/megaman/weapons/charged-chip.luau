--!strict
-- MegaMan's A-charge in the souls that charge chips (`megaman/charged-chip`): the original's weapon routines 5, 0x16,
-- 0x18, 0x1B and 0x27 (0x0800F520, empty, as BN6's `nullsub_44`), which differ only in their rows of the charge
-- table. BN5's chip use (0x0800D9CC) never runs them: a charged chip is the chip with the charge's bonus (the soul's
-- `charged_bonus`, 0x080103D0), whatever the routine.

local charged_chip = {}

-- The routine `id`, charging for `ticks` (its row of the charge table, and Charge 5's reading on into the next).
function charged_chip.routine(id: string, ticks: { number }): Weapon
    return define.weapon { id = id, charge_ticks = ticks, charged_chip = "bonus" }
end

-- Routine 5 (ProtoSoul's): the charge table's row 5 (0x0801CA9E); Charge 5 reads on into row 6's first entry.
charged_chip.weapon = charged_chip.routine("bn5:megaman/charged-chip", { 30, 30, 30, 30, 30, 40 })

return charged_chip
