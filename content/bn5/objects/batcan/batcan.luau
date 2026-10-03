--!strict
-- BN5's BatCan1-4 (action 0x36, 0x080EE410) and their shot (attack object #1, 0x080C49FC): BN6's (@common/batcan)
-- with BN5's look: the bat cannon (BN5's attachment row 0x21, BN6's 0xF row for row, but on its holder itself: BN5's
-- attachment, 0x080B9AB8, has no place by its holder's NameID for the bat as BN6's has, and the row names no attach
-- point), BN5's shot collision types and bat spark, and no restart of the navi's form overlay as the cannon rises
-- (0x080EE448; BN6's restarts it). The shots (0x080EE444's counts, 0x080EE4DC's BatCan4 rows) are BN6's.

local attachment = require("../attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@common/batcan/batcan")
local sparks = require("../../lib/sparks")

return common.make {
    shot_id = "bn5:batcan/shot",
    bat = attachment.look { sprite = asset.sprite("bn5:bat-cannon"), palette = 0 },
    attachment = attachment.kind,
    sound = asset.sound("bn5:cannon"),
    shot = { is = collision.attack, hits = collision.hits_navis, spark = sparks.bat },
    restarts_overlay = false,
}
