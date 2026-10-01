--!strict
-- Gregar Beast: one of MegaMan's forms.

local gregar_beast_buster = require("../../weapons/gregar-beast-buster/weapon")
local charged_chip_bonus = require("../../weapons/charged-chip-bonus/weapon")
local beast_claw = require("../../weapons/beast-claw/weapon")

return define.form {
    id = "gregar-beast",
    name = "Gregar Beast",
    kind = "beast",
    game = "gregar",
    sprite = asset.sprite("gregar-beast"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = gregar_beast_buster.weapon, a_charge = charged_chip_bonus, alt_a_charge = beast_claw.weapon },
    mugshot = asset.mugshot("falzar-beast"),
    -- Beast Out: its damaging Null chips' bonus, where the family's doesn't apply.
    null_bonus = 30,
    -- The chips A charges.
    charged_chips = { { family = "null", damaging = false } },
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "super_armor", "lockon_marker" },
    -- The Cross special's buster volley (`sub_802D4F0`).
    special_volley = 2,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 11 },
    identity = define.identity {
        class = "beast",
        -- Where a held thing with no attach point sits (BatCan's cannon).
        held_offset = { 11, -7 },
        version = 0,
        actor_type = "player",
        ai_index = 35,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 12,
        ice = "medium",
        attach_points = {
            { 6, 0 },
            { 6, 0 },
            { 6, 0 },
            { 16, 44 },
            { 16, 44 },
            { 16, 44 },
            { 24, 0 },
            { 27, 17 },
            { 24, 17 },
            { 38, 18 },
            { 31, 18 },
            { 31, 18 },
            { 29, 17 },
            { 35, 17 },
            { 35, 19 },
            { 35, 17 },
            { 51, 20 },
            { 8, 10 },
            { 41, 20 },
            { 48, 19 },
            { 32, 19 },
            { 6, 16 },
            { 35, 18 },
            { 32, 18 },
            { 32, 16 },
            { 32, 19 },
            { 36, 17 },
            { 8, 16 },
            { 104, 8 },
            { 124, -10 },
            { -8, 24 },
            { 80, 14 },
            { 6, 0 },
            { 0, 4 },
        },
    },
}
