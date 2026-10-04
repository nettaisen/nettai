--!strict
-- The patch card Duo (106, デューオ in the Japanese games), 70 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "duo",
    mb = 70,
    effects = {
        cards.hp_percent(40),
        cards.mega_folder(1),
        cards.giga_folder(1),
    },
}
