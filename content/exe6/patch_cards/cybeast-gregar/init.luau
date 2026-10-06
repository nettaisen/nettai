--!strict
-- The patch card Cybeast Gregar (115, 電脳獣グレイガ in the Japanese games), 70 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 70,
    effects = {
        cards.hp(600),
        cards.attack(4),
        cards.rapid(4),
        cards.super_armor(true),
        cards.bug(cards.custom_hp_drain(2)),
    },
}

return { ["cybeast-gregar"] = card }
