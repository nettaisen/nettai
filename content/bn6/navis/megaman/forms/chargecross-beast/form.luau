--!strict
-- ChargeCross Beast: one of MegaMan's forms.

local chargeman = require("../../../chargeman/navi")
local gregar_beast = require("../gregar-beast/form")
local gregar_beast_buster = require("../../weapons/gregar-beast-buster/weapon")
local charged_chip_bonus = require("../../weapons/charged-chip-bonus/weapon")
local wave = require("./wave")
local forms = require("../../../../rules/forms/system")

return define.form {
    id = "chargecross-beast",
    kind = "cross_beast",
    -- What changes MegaMan into it (BN6's five sequences).
    change = forms.action,
    game = "gregar",
    cross_of = chargeman,
    -- A weakness hit breaks the Cross: back to the Beast.
    breaks_to = gregar_beast,
    sprite = asset.sprite("chargecross-beast"),
    element = "fire",
    buster_bonus = 0,
    weapons = { buster = gregar_beast_buster.weapon, a_charge = charged_chip_bonus, alt_a_charge = wave.weapon },
    -- Its face in the emotion window (`byte_801E700`): the Falzar ROM has no
    -- ChargeCross Beast face, and shows DustCross Beast's.
    mugshot = asset.mugshot("chargecross-beast"),
    -- Beast Out: its damaging Null chips' bonus, where the family's doesn't apply.
    null_bonus = 30,
    -- The chips A charges.
    charged_chips = { { family = "fire" }, { family = "null", damaging = false } },
    -- The A charge builds up the next Fire chip's damage, up to this much.
    fire_charge = 100,
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "super_armor", "target_marker", "invulnerable", "berserk" },
    -- After a NaviCust change only the SuperArmor comes back (`sub_801469C`).
    navicust_refresh = { "super_armor" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 17,
    -- The target marker shows, its afterimages stay, and a Null chip's A charge times by the
    -- alternative routine (`sub_80E1566`, `sub_80E341E`, `sub_8012F62`).
    traits = { "shows_target_marker", "afterimages_stay", "alt_charge_time" },
    -- The custom screen deals one more chip for each screen spent in it, up to three (the
    -- cross system's, `sub_802A49C`).
    extra_chips = true,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 17, palette_add = 9 },
    identity = define.identity {
        class = "cross_beast",
        version = 0,
        actor_type = "player",
        ai_index = 41,
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
