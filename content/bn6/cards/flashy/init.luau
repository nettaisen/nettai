--!strict
-- The patch card Flashy (19, ピカラー in the Japanese games), 11 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "flashy",
    mb = 11,
    effects = {
        cards.hp_percent(5),
        cards.body("elec"),
        cards.bug(cards.attack(-1)),
        cards.custom_damage(0),
    },
}
