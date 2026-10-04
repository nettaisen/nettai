--!strict
-- Falzar Beast Over: one of MegaMan's forms.

local falzar_beast = require("../falzar-beast/form")
local falzar_beast_buster = require("../../weapons/falzar-beast-buster/weapon")
local beast_claw = require("../../weapons/beast-claw/weapon")
local forms = require("../../../../rules/forms/system")

return define.form {
    id = "falzar-beast-over",
    kind = "beast_over",
    -- The target marker shows, its afterimages stay, a Null chip's A charge times by the claw,
    -- and its damaging Null chips deal double (`sub_80E1566`, `sub_80E341E`, `sub_8012F62`,
    -- `sub_8012ABC`); the beast system's berserk decides its idle (`sub_802D322`).
    traits = { "controlled", "shows_target_marker", "afterimages_stay", "alt_charge_time", "doubles_null" },
    -- It glows (`sub_8016A38`, `byte_8016A9C`): the navi's color shader by the battle time, a tick each.
    glow = {
        0x0000, 0x0000, 0x0402, 0x0423, 0x0444, 0x0866, 0x0887, 0x0888, 0x0CAA,
        0x0CCC, 0x0CEE, 0x110F, 0x1110, 0x1110, 0x110F, 0x0CEE, 0x0CCC, 0x0CAA,
        0x0888, 0x0887, 0x0866, 0x0865, 0x0444, 0x0443, 0x0422, 0x0421,
    },
    -- What changes MegaMan into it (BN6's five sequences).
    change = forms.action,
    game = "falzar",
    -- (No hit breaks Beast Over; the original's routine, by the form's number, would drop
    -- either to the Falzar beast.)
    breaks_to = falzar_beast,
    sprite = asset.sprite("falzar-beast"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = falzar_beast_buster.weapon, alt_a_charge = beast_claw.weapon },
    mugshot = asset.mugshot("falzar-beast-over"),
    -- What its status reset gives the navi (`sub_8014536`): untouchable, with AirShoe and FloatShoe (one
    -- literal, 0x08000030), so no hit reaches it.
    status_reset = { "untouchable", "air_shoes", "float_shoes", "floating_body", "target_marker", "berserk" },
    -- After a NaviCust change (`sub_801469C`), which no hit can bring, it is invulnerable as well.
    navicust_refresh = { "air_shoes", "float_shoes", "floating_body", "untouchable", "invulnerable", "berserk" },
    -- It floats 20 pixels up.
    hover = 0x14,
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 24,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 24 },
    identity = define.identity {
        class = "beast_over",
        version = 0,
        actor_type = "player",
        ai_index = 48,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 13,
        ice = "medium",
        -- The beast's head, whose palette follows the mood.
        parts = { beast_head = { sprite = asset.sprite("beast-head") } },
        attach_points = {
            { 4, 16 },
            { 4, 24 },
            { 4, 16 },
            { 9, 64 },
            { 9, 64 },
            { 9, 64 },
            { 16, 0 },
            { 16, 37 },
            { 16, 38 },
            { 24, 38 },
            { 17, 38 },
            { 17, 38 },
            { 18, 38 },
            { 22, 38 },
            { 22, 39 },
            { 24, 37 },
            { 40, 39 },
            { 9, 23 },
            { 29, 40 },
            { 40, 39 },
            { 18, 39 },
            { 4, 32 },
            { 22, 38 },
            { 22, 39 },
            { 24, 39 },
            { 20, 39 },
            { 25, 38 },
            { 4, 32 },
            { 88, 28 },
            { 114, 8 },
            { -8, 28 },
            { 65, 34 },
            { 4, 10 },
            { 4, 8 },
        },
    },
}
