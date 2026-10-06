--!strict
-- The patch card Anetta's Custom (49, アネッタの カスタマイズ in the Japanese games), 47 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "anettas-custom",
    mb = 47,
    effects = {
        cards.hp_percent(30),
        cards.body("wood"),
    },
}
