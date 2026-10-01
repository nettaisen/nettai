--!strict
-- PunchArm's charged shot (`puncharm/charge`): the weapon the chip makes the
-- navi's charged shot (lib/navi-boost/controller). It is sticky: a form's
-- own charged shot doesn't replace it, and while the navi has it a Beast
-- buster gives way to the plain one. Nothing implements its attack yet,
-- so it has no `setup`: firing it is an error.

return define.weapon {
    id = "puncharm/charge",
    name = "PunchArm",
    charge_ticks = { 80, 80, 80, 80, 80, 100, 100, 100 }, -- by Charge 0..7; past 4, the next routine's row
    sticky = true,
}
