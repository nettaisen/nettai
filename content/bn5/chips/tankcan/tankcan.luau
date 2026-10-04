--!strict
-- BN5's TankCans (action 0x46, 0x080EEF4A): BN6's tank cannon and shell (@exelib/tankcan: the same code) with BN5's
-- look: the cannon (the attachment table's row 0x25, BN6's 0x14: the same look), the shell (attack object #0x3C,
-- 0x080CC9EC: its collision row 0, BN6's), its edge blast BN5's effect 0x47 hitting as an attack (BN6's thrown,
-- 0x080CCB40) and unshifted, its sounds and effects.

local attachment = require("../../objects/attachment")
local action = require("@exelib/tankcan/action")
local collision = require("../../rules/collision")
local shell = require("@exelib/tankcan/shell")

local tankcan = {}

tankcan.kind = shell.make_kind("tankcan/shell", {
    is = collision.attack,
    hits = collision.hits_navis,
    sound = asset.sound("cannon"),
    hit_sound = asset.sound("burst"),
    blast = define.effect { sprite = asset.sprite("tank-blast"), anim = 0, palette = 0 },
    blast_is = collision.attack,
    explosion = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    burst = define.effect { sprite = asset.sprite("shell-burst"), anim = 0, palette = 0 },
})

local CANNON = attachment.look { sprite = asset.sprite("tank-cannon"), palette = 0, attach_point = 8 }

-- The TankCans' action keyed `id`, its shell flying as `flies` says (the attack's parameters: MegaMan's B↓Ccann
-- loads TankCan1 and takes the cracking away, navis/megaman/weapons/chips).
function tankcan.make_action(id: string, flies: ShellSpec): Action
    return action.action { id = id, cannon = CANNON, attachment = attachment.kind, kind = tankcan.kind, shell = flies }
end

-- The TankCans' action: their shell (the records' parameters: row 0, hit modifier 0x4B; cracking).
tankcan.action = tankcan.make_action("tankcan/action", { hit_mod = 0x4B, cracks = true, shift = 0 })

return tankcan
