--!strict
-- TankCan's shell (the original's attack object #0x3C, `sub_80CB6F8`):
-- the shared shell (@exelib/tankcan/shell) with EXE6's look. Unseen, it
-- steps a panel every two ticks from where it appears; its own hit ends
-- it with a fiery burst. Off the far edge it blasts the last column it
-- crossed (and cracks it), and 15, 20 and 25 ticks later explosions go off
-- over that column's three panels in an order the RNG shuffles. See
-- docs/engine/shot-chips.md §7.1.

local collision = require("../../rules/collision")
local common = require("@exelib/tankcan/shell")
local effects = require("../../lib/effects")

local shell = {}

shell.kind = common.make_kind("tankcan/shell", {
    is = collision.attack,
    hits = collision.hits_navis,
    sound = asset.sound("cannon"),
    hit_sound = asset.sound("burst"),
    -- The edge blast: its effect (effect 0x60), a thrown hit.
    blast = define.effect { sprite = asset.sprite("tank-blast"), anim = 0, palette = 0 },
    blast_is = collision.thrown,
    explosion = effects.explosion,
    burst = effects.shell_burst,
})

-- `sub_80CB8DE`: a shell from `owner` on panel (x, y) at height `z`
-- (16.16), element 0, with the damage word.
function shell.fire(owner: Object, x: number, y: number, z: number, damage: number, spec: ShellSpec): Object?
    return common.fire(shell.kind, owner, x, y, z, damage, spec)
end

return shell
