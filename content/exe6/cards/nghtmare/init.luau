--!strict
-- The patch card NghtMare (81, ナイトメア in the Japanese games), 5 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "nghtmare",
    mb = 5,
    effects = {
        cards.hp(20),
        cards.bug(cards.emotion_bug(1)),
        cards.bug(cards.status_bug(1)),
    },
}
