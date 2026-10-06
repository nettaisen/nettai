--!strict
-- The patch card Whirly (29, ウズリム in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

return define.patch_card {
    id = "whirly",
    mb = 20,
    effects = {
        cards.hp(80),
        cards.body("aqua"),
        cards.buster_shot(variants.sea),
        cards.bug(cards.panel_trail("water", 2)),
    },
}
