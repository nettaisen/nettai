--!strict
-- The patch card StarMan (45, スターマン in the Japanese games), 32 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "starman",
    mb = 32,
    effects = {
        cards.bug(cards.attack(-2)),
        cards.mega_folder(1),
        cards.panel_trail("none", 0),
        cards.emotion_bug(0),
        cards.hp_drain(0),
    },
}
