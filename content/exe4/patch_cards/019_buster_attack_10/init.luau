--!strict
-- The patch card Buster Attack 10 (19, バスター攻撃力10).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.buster_attack(9),
        cards.bug("hp", 2),
    },
}

return { ["019-buster-attack-10"] = card }
