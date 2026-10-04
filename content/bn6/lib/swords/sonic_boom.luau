--!strict
-- BN6's sonic boom (the original's attack object #0x58, `sub_80CF7F0`):
-- the shared boom (@exelib/swords/sonic_boom), BN6's kind and look: 16
-- pixels up; its primary element's palette (`byte_80CF8A8`); a slash that
-- hits navis.

local collision = require("../../rules/collision")
local common = require("@exelib/swords/sonic_boom")

local sonic_boom = {}

local BN6: SonicBoomLook = {
    sprite = asset.sprite("bn6:sword-slash"),
    z = 0x10_0000,
    piercing_palette = 0xB,
    palettes = { [0] = 0, 1, 2, 3, 4, 0, 0, 0 },
    element_mask = 0xF,
    is = collision.slash,
    hits = collision.hits_navis,
}

-- Its speed (16.16 pixels a tick) as its throwers set it.
sonic_boom.SPEED = common.SPEED

sonic_boom.kind = common.make_kind("bn6:sonic-boom", BN6)

-- `sub_80CF91E`: a sonic boom from `owner` on panel (x, y), with
-- `element`, the damage word and what it is. (A game's own kind of it:
-- `kind`.)
function sonic_boom.throw(owner: Object, x: number, y: number, element: number, damage: number, boom: SonicBoom, kind: Kind?): Object?
    return common.throw(kind or sonic_boom.kind, owner, x, y, element, damage, boom)
end

return sonic_boom
