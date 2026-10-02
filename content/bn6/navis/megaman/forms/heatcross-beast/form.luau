--!strict
-- HeatCross Beast: one of MegaMan's forms.

local heatman = require("../../../heatman/navi")
local gregar_beast = require("../gregar-beast/form")
local gregar_beast_buster = require("../../weapons/gregar-beast-buster/weapon")
local charged_chip_bonus = require("../../weapons/charged-chip-bonus/weapon")
local charge = require("./charge")

return define.form {
    id = "heatcross-beast",
    name = "HeatCross Beast",
    kind = "cross_beast",
    game = "gregar",
    cross_of = heatman,
    -- A weakness hit breaks the Cross: back to the Beast.
    breaks_to = gregar_beast,
    sprite = asset.sprite("heatcross-beast"),
    element = "fire",
    buster_bonus = 1,
    weapons = { buster = gregar_beast_buster.weapon, a_charge = charged_chip_bonus, alt_a_charge = charge.weapon },
    -- Its face in the emotion window (`byte_801E700`): the Falzar ROM has no
    -- HeatCross Beast face, and shows SpoutCross Beast's.
    mugshot = asset.mugshot("heatcross-beast"),
    -- The damage it adds to its family's damaging chips.
    chip_bonus = { family = "fire", damage = 50 },
    -- Beast Out: its damaging Null chips' bonus, where the family's doesn't apply.
    null_bonus = 30,
    -- The chips A charges.
    charged_chips = { { family = "null", damaging = false } },
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "super_armor", "lockon_marker", "invulnerable", "berserk" },
    -- After a NaviCust change only the SuperArmor comes back (`sub_801469C`).
    navicust_refresh = { "super_armor" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 13,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 13, palette_add = 5 },
    identity = define.identity {
        class = "cross_beast",
        version = 0,
        actor_type = "player",
        ai_index = 37,
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
