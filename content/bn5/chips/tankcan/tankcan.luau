--!strict
-- BN5's TankCans (action 0x46, 0x080EEF4A): BN6's tank cannon and shell (@exelib/tankcan: the same code) with BN5's
-- look: the cannon (the attachment table's row 0x25, BN6's 0x14: the same look), the shell (attack object #0x3C,
-- 0x080CC9EC: its collision row 0, BN6's), its edge blast BN5's effect 0x47 hitting as an attack (BN6's thrown,
-- 0x080CCB40) and unshifted, its sounds and effects.

local attachment = require("../../objects/attachment/attachment")
local action = require("@exelib/tankcan/action")
local collision = require("../../rules/collision")
local shell = require("@exelib/tankcan/shell")

local tankcan = {}

tankcan.kind = shell.make_kind("bn5:tankcan/shell", {
    is = collision.attack,
    hits = collision.hits_navis,
    sound = asset.sound("bn5:cannon"),
    hit_sound = asset.sound("bn5:burst"),
    blast = define.effect { sprite = asset.sprite("bn5:tank-blast"), anim = 0, palette = 0 },
    blast_is = collision.attack,
    explosion = define.effect { sprite = asset.sprite("bn5:explosion"), anim = 0, palette = 0 },
    burst = define.effect { sprite = asset.sprite("bn5:shell-burst"), anim = 0, palette = 0 },
})

local CANNON = attachment.look { sprite = asset.sprite("bn5:tank-cannon"), palette = 0, attach_point = 8 }

-- The TankCans' action: their shell (the records' parameters: row 0, hit modifier 0x4B; cracking).
tankcan.action = action.action {
    id = "bn5:tankcan/action",
    cannon = CANNON,
    attachment = attachment.kind,
    kind = tankcan.kind,
    shell = { hit_mod = 0x4B, cracks = true, shift = 0 },
}

return tankcan
