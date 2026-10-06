--!strict
-- The patch card Hub Hikari (110, 光彩斗 in the Japanese games), 80 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 80,
    effects = {
        cards.attack_times(2),
        cards.custom(3),
        cards.mega_folder(2),
        cards.giga_folder(1),
        cards.bug(cards.hp_percent(-50)),
    },
}

return { ["hub-hikari"] = card }
