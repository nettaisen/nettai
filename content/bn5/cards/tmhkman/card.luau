--!strict
-- The patch card TmhkMan (95, トマホークマン in the Japanese games), 40 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn5:tmhkman",
    mb = 40,
    effects = {
        cards.hp(200),
        cards.body("wood"),
        cards.super_armor(true),
        cards.attack(3),
        cards.bug(cards.mega_folder(-1)),
    },
}
