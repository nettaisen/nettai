--!strict
-- A point of ColorPt's and DblPoint's (the original's effect object #0x51,
-- `sub_80E655C`; docs/engine/dimming-chips.md §10.2): it gives its panel
-- to the other side (the stolen-area return brings it back), rises, and
-- flies to its side's navi; there the chip at the hand's cursor, if it does
-- damage, gets its bonus (or, while a special chip is in use, the side's
-- special bonus does). It tells its controller (the holder) when it is
-- gone. The behavior is BN5's too (@common/colorpt/point), made here with
-- BN6's look.

local common = require("@common/colorpt/point")

return common.make {
    id = "bn6:colorpt/point",
    sprite = asset.sprite("bn6:grab-shot"),
    sounds = {
        appear = asset.sound("bn6:point-appear"),
        rise = asset.sound("bn6:point-rise"),
        arrive = asset.sound("bn6:cross-merge"),
    },
    -- While a special chip is in use (the player's AI+0xBB).
    special = function(_side: number, navi: Object): boolean
        return navi.special_source ~= 0
    end,
}
