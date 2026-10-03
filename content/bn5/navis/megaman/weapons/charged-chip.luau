--!strict
-- MegaMan's A-charge in the souls that charge chips (`megaman/charged-chip`): the original's weapon routine 5
-- (0x0800F520, empty, as BN6's `nullsub_44`). BN5's chip use (0x0800D9CC) never runs it: a charged chip is the chip
-- with the charge's bonus (the soul's `charged_bonus`, 0x080103D0), whatever the routine.

return {
    weapon = define.weapon {
        id = "bn5:megaman/charged-chip",
        -- The charge table's row 5 (0x0801CA9E); Charge 5 reads on into row 6's first entry.
        charge_ticks = { 30, 30, 30, 30, 30, 40 },
        charged_chip = "bonus",
    },
}
