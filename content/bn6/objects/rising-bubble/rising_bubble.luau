--!strict
-- The rising bubble (the original's effect object #0x14, `sub_80E1CF8`):
-- the shared bubble (@common/rising-bubble/rising_bubble), BN6's kind and
-- sprite (the small puff).

local common = require("@common/rising-bubble/rising_bubble")

local rising_bubble = {}

rising_bubble.kind = common.make_kind("bn6:rising-bubble", asset.sprite("bn6:small-puff"))

-- `sub_80E1D7A`: a bubble at `pos` in sprite palette `palette`.
function rising_bubble.spawn(pos: Vec3, palette: number): Object?
    return common.spawn(rising_bubble.kind, pos, palette)
end

return rising_bubble
