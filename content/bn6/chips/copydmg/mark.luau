--!strict
-- CopyDmg's mark (the original's attack object #0x28, `sub_80C8FFC`): the
-- one BN5 shares (@common/copydmg/mark) in BN6's look: an unseen probe
-- that steps a panel a tick along its row until it hits something; a navi
-- it hits is marked for 180 ticks, taking on top of its own the damage its
-- side took the tick before.

local collision = require("../../rules/collision")
local common = require("@common/copydmg/mark")

return common.make {
    id = "bn6:copydmg/mark",
    sprite = asset.sprite("bn6:hit-sparks"),
    anim = 0x0B,
    sound = asset.sound("bn6:mark"),
    is = collision.attack,
    hits = collision.hits_navis,
    reload_anim = false,
    per_player_hit = nil,
}
