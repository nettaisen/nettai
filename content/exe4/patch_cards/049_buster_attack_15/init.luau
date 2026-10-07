--!strict
-- The patch card Buster Attack 15 (49, バスター攻撃力15).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.buster_attack(14),
        cards.bug("hp", 1),
        cards.bug("move", 1),
    },
}

return { ["049-buster-attack-15"] = card }
