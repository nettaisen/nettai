--!strict
-- The patch card Mushy (11, チャマッシュ in the Japanese games), 15 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn5:mushy",
    mb = 15,
    effects = {
        cards.body("wood"),
        cards.charge_shot_kind(variants.charged_confusing),
        cards.bug(cards.step_bug(1)),
    },
}
