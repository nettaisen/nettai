--!strict
-- BatCan's shot (the original's attack object #1, `sub_80C50B8`) and the
-- chips' action: the projectile's twin without a look. Unseen, it steps a
-- panel every two ticks from where it appears and ends on its first hit,
-- with the bat's spark; what differs between BatCan's shots is its hit
-- modifier. See docs/engine/shot-chips.md §6.1. The behavior is BN5's too
-- (@exelib/batcan), made here with BN6's look.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/batcan/batcan")
local sparks = require("../../lib/sparks")

return common.make {
    shot_id = "bn6:batcan/shot",
    -- The bat cannon (the original's attachment row 0xF: it sits at a fixed
    -- offset by its holder, not at an attach point).
    bat = attachment.look { sprite = asset.sprite("bn6:bat-cannon"), palette = 0, by_owner = true },
    attachment = attachment.kind,
    sound = asset.sound("bn6:cannon"),
    shot = { is = collision.attack, hits = collision.hits_navis, spark = sparks.bat },
    restarts_overlay = true,
}
