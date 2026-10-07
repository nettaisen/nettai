--!strict
-- EXE4's lance (attack object #0x4F: init 0x080D73F8, tick 0x080D7482, spawn 0x080D7522; EXE5's 0x080CF870 on,
-- @exelib/lance) with its sprite (0C-2B) and sound (the swords' swing, 0x8E): an attack the whole time it shows,
-- against navis with hit modifier 0x10 and its element's spark (0x08012F7A); a lance of Param1 1 (DrkLance's) in its
-- second palette.

local collision = require("../../rules/collision")
local lance = require("@exelib/lance")
local sparks = require("../../lib/sparks")

local exe4_lance = {}

exe4_lance.kind = lance.make_kind("lance", {
    sprite = asset.sprite("lance"),
    sound = asset.sound("sword-swing"),
    attack = { is = collision.attack, hits = collision.hits_navis, hit_mod = 0x10, spark = sparks.of_element },
})

-- 0x080D7540, action 0x20's variant 6: three lances of `user`'s side in the far column at its height, in palette
-- `palette`, with the damage word `damage`.
function exe4_lance.thrust(user: Object, element: number, palette: number, damage: number)
    lance.thrust(exe4_lance.kind, user, element, user.pos.z, palette, damage, true)
end

return exe4_lance
