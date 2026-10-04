--!strict
-- The mine the Mine chip lays (the original's attack object #0x4C,
-- `sub_80CDD44`): the shared mine (@exelib/mine/land_mine), BN6's kind,
-- identity and look.

local collision = require("../../rules/collision")
local common = require("@exelib/mine/land_mine")

local land_mine = {}

land_mine.kind = common.make_kind {
    id = "bn6:mine/land-mine",
    identity = define.identity {
        id = "bn6:mine",
        class = "field_object",
        version = 1,
        ai_index = 1,
        absorbable = false,
        scrap = false,
        look = { anim = 0, palette = 0, shadow = false },
    },
    sprite = asset.sprite("bn6:land-mine"),
    hop_sound = asset.sound("bn6:hop"),
    -- The original's effect 0x47.
    blast = define.effect { sprite = asset.sprite("bn6:blast"), anim = 0, palette = 0 },
    blast_sound = asset.sound("bn6:hit-bomb-1"),
    self_type = collision.mine,
    target = collision.hits_both_sides,
}

-- `sub_80CDF60`: a mine for `side` with the damage word `damage`.
function land_mine.spawn(side: number, damage: number): Object?
    return common.spawn(land_mine.kind, side, damage)
end

return land_mine
