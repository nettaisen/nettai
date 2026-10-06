--!strict
-- EXE5's TankCans (action 0x46, 0x080EEF4A): EXE6's tank cannon and shell (@exelib/tankcan: the same code) with EXE5's
-- look: the cannon (the attachment table's row 0x25, EXE6's 0x14: the same look), the shell (attack object #0x3C,
-- 0x080CC9EC: its collision row 0, EXE6's), its edge blast EXE5's effect 0x47 hitting as an attack (EXE6's thrown,
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
    blast = new.effect { sprite = asset.sprite("tank-blast"), anim = 0, palette = 0 },
    blast_is = collision.attack,
    explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    burst = new.effect { sprite = asset.sprite("shell-burst"), anim = 0, palette = 0 },
})

-- The tank cannon (the attachment table's row 0x25), which Colonel's C-Cannon holds too (navis/colonel/chip).
local CANNON = attachment.look { sprite = asset.sprite("tank-cannon"), palette = 0, attach_point = 8 }
tankcan.cannon = CANNON

-- The TankCans' action (`id`: the key compat names it by, if it does), its shell flying as `flies` says (the attack's parameters: MegaMan's B↓Ccann
-- loads TankCan1 and takes the cracking away, navis/megaman/weapons/chips).
function tankcan.make_action(flies: ShellSpec, id: string?): Action
    return action.action { id = id, cannon = CANNON, attachment = attachment.kind, kind = tankcan.kind, shell = flies }
end

-- The TankCans' action: their shell (the records' parameters: row 0, hit modifier 0x4B; cracking).
tankcan.action = tankcan.make_action({ hit_mod = 0x4B, cracks = true, shift = 0 })

return tankcan
