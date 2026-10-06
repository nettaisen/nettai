--!strict
-- The patch card Eleball (73, パラボール in the Japanese games), 14 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 14,
    effects = {
        cards.body("elec"),
        cards.float_shoes(true),
        cards.encounter_element("battery"),
        cards.bug(cards.buster_shot(nil)),
    },
}

return { eleball = card }
