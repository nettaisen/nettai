--!strict
-- The patch card Cybeast Gregar (115, 電脳獣グレイガ in the Japanese games), 70 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "cybeast-gregar",
    mb = 70,
    effects = {
        cards.hp(600),
        cards.super_armor(true),
        cards.attack(4),
        cards.rapid(4),
        cards.bug(cards.custom_hp_drain(2)),
    },
}
