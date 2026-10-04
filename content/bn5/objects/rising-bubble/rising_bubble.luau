--!strict
-- BN5's rising bubble (effect object #0x14, BN6's code: @common/rising-bubble/rising_bubble), BN5's kind; its sprite
-- BN6's small puff.

local common = require("@common/rising-bubble/rising_bubble")

local rising_bubble = {}

rising_bubble.kind = common.make_kind("bn5:rising-bubble", asset.sprite("bn5:small-puff"))

-- A bubble at `pos` in sprite palette `palette`.
function rising_bubble.spawn(pos: Vec3, palette: number): Object?
    return common.spawn(rising_bubble.kind, pos, palette)
end

return rising_bubble
