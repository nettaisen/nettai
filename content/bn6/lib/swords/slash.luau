--!strict
-- BN6's swords' slash (the original's action 0x13, `sub_80EB776`): the
-- shared slash (@common/swords/slash) with BN6's attachment for the blade.
-- Each chip composes its own (`slash.action { blade = ..., hit = ...,
-- effect = ..., sound = ... }`, docs/design/content-model-v2.md §5.2).

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@common/swords/slash")
local regions = require("@common/regions")

local slash = {}

-- One state for every sword's slash.
slash.STATE = common.STATE

-- CrosSwrd's second hit: a slash on the panel ahead alone.
slash.CROSS_HIT = {
    region = regions.single,
    collision = { is = collision.slash, hits = collision.hits_navis },
    hit_mod = 3,
}

-- A chip's slash.
function slash.action(spec: SlashSpec): Action
    return common.action(spec, attachment.kind)
end

return slash
