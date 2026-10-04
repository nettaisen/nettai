--!strict
-- A gust (attack object #0x49, `sub_80CD2EC`): the shared one (@common/gust/gust) with BN6's sprite and collision
-- types. WindRack blows unseen ones down each row; Wind and Fan blow one row at a time; TenguCross's B+Back blows
-- three (instant chip effect 0x14).

local collision = require("../../rules/collision")
local common = require("@common/gust/gust")

local gust = {}

local made = common.make {
    id = "bn6:gust",
    sprite = asset.sprite("bn6:gust"),
    self_type = collision.pull,
    target = collision.hits_navis,
    -- (The code after `byte_80CD478`.)
    past_table = { 0xB427B500, 0xF7352049 },
}

gust.kind = made.kind
gust.spawn = made.spawn

return gust
