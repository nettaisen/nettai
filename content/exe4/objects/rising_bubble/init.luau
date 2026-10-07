--!strict
-- EXE4's rising bubble (effect object #0x14, 0x080E37F8: EXE5's and EXE6's code, @exelib/rising_bubble), EXE4's kind;
-- its sprite the small puff (14-02).

local common = require("@exelib/rising_bubble")

local rising_bubble = {}

rising_bubble.kind = common.make_kind("rising-bubble", asset.sprite("small-puff"))

-- A bubble at `pos` in sprite palette `palette`.
function rising_bubble.spawn(pos: Vec3, palette: number): Object?
    return common.spawn(rising_bubble.kind, pos, palette)
end

return rising_bubble
