--!strict
-- EraseCross Beast: one of MegaMan's forms.

local eraseman = require("../../eraseman")
local gregar_beast = require("./gregar-beast")
local gregar_beast_buster = require("../weapons/gregar-beast-buster")
local charged_chip_bonus = require("../weapons/charged-chip-bonus")
local drop = require("@self/drop")
local forms = require("../../../rules/forms")

return define.form {
    id = "erasecross-beast",
    kind = "cross_beast",
    -- What changes MegaMan into it (EXE6's five sequences).
    change = forms.action,
    version = "gregar",
    cross_of = eraseman,
    -- A weakness hit breaks the Cross: back to the Beast.
    breaks_to = gregar_beast,
    sprite = asset.sprite("erasecross-beast"),
    element = "null",
    weakness = { "wind" },
    buster_bonus = 0,
    weapons = { buster = gregar_beast_buster.weapon, a_charge = charged_chip_bonus, alt_a_charge = drop.weapon },
    -- Its face in the emotion window (`byte_801E700`): the Falzar ROM has no
    -- EraseCross Beast face, and shows GroundCross Beast's.
    mugshot = asset.mugshot("erasecross-beast"),
    -- The damage it adds to its family's damaging chips (dimming chips too).
    chip_bonus = { family = "cursor", damage = 30, dimming_chips = true },
    -- Beast Out: its damaging Null chips' bonus, where the family's doesn't apply.
    null_bonus = 30,
    -- The chips A charges.
    charged_chips = { { family = "null", damaging = false } },
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "super_armor", "target_marker", "invulnerable", "berserk" },
    -- After a NaviCust change only the SuperArmor comes back (`sub_801469C`).
    navicust_refresh = { "super_armor" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 16,
    -- The target marker shows, its afterimages stay, and a Null chip's A charge times by the
    -- alternative routine (`sub_80E1566`, `sub_80E341E`, `sub_8012F62`).
    traits = { "erases", "shows_target_marker", "afterimages_stay", "alt_charge_time" },
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 16, palette_add = 8 },
    identity = define.identity {
        class = "cross_beast",
        version = 0,
        actor_type = "player",
        ai_index = 40,
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
