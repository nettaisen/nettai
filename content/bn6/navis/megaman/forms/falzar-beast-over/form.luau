--!strict
-- Falzar Beast Over: one of MegaMan's forms.

local falzar_beast = require("../falzar-beast/form")
local falzar_beast_buster = require("../../weapons/falzar-beast-buster/weapon")
local beast_claw = require("../../weapons/beast-claw/weapon")
local forms = require("../../../../rules/forms/system")

return define.form {
    id = "bn6:falzar-beast-over",
    kind = "beast_over",
    -- The beast system's berserk decides its idle (`sub_802D322`).
    traits = { "controlled" },
    -- What changes MegaMan into it (BN6's five sequences).
    change = forms.action,
    game = "falzar",
    -- (No hit breaks Beast Over; the original's routine, by the form's number, would drop
    -- either to the Falzar beast.)
    breaks_to = falzar_beast,
    sprite = asset.sprite("bn6:falzar-beast"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = falzar_beast_buster.weapon, alt_a_charge = beast_claw.weapon },
    mugshot = asset.mugshot("bn6:falzar-beast-over"),
    -- What its status reset gives the navi (`sub_8014536`): untouchable, with AirShoe and FloatShoe (one
    -- literal, 0x08000030), so no hit reaches it.
    status_reset = { "untouchable", "air_shoes", "float_shoes", "floating_body", "lockon_marker", "berserk" },
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
        parts = { beast_head = { sprite = asset.sprite("bn6:beast-head") } },
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
