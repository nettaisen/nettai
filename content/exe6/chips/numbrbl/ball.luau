--!strict
-- A NumbrBl ball (the original's attack object #0x91, `sub_80D7278`;
-- docs/engine/dimming-chip-effects.md §15): the one EXE5 shares
-- (@exelib/numbrbl/ball) in EXE6's look. NumberMan (./numberman) sets it
-- down in front of him; it sits 15 ticks, waits, then rolls down the row 3
-- pixels a tick until it hits something (an explosion) or leaves the
-- field.

local collision = require("../../rules/collision")
local common = require("@exelib/numbrbl/ball")
local effects = require("../../lib/effects")

return common.make {
    id = "numbrbl/ball",
    sprite = asset.sprite("number-ball"),
    -- The ticks it sits before its wait (the first sets the count).
    sit_ticks = 0xE,
    set_sound = asset.sound("place"),
    hit_sound = asset.sound("damage"),
    explosion = effects.explosion,
    -- An attack hitting navis.
    is = collision.attack,
    hits = collision.hits_navis,
    hidden_parts = nil,
}
