--!strict
-- The patch card Anetta's Custom (49, アネッタの カスタマイズ in the Japanese games), 47 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn5:anettas-custom",
    mb = 47,
    effects = {
        cards.hp_percent(30),
        cards.body("wood"),
    },
}
