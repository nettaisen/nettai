--!strict
-- EXE5's rising bubble (effect object #0x14, EXE6's code: @exelib/rising-bubble/rising_bubble), EXE5's kind; its sprite
-- EXE6's small puff.

local common = require("@exelib/rising-bubble")

local rising_bubble = {}

rising_bubble.kind = common.make_kind("rising-bubble", asset.sprite("small-puff"))

-- A bubble at `pos` in sprite palette `palette`.
function rising_bubble.spawn(pos: Vec3, palette: number): Object?
    return common.spawn(rising_bubble.kind, pos, palette)
end

return rising_bubble
