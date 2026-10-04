--!strict
-- Bass, the navi of BN5's Bass chip (BN5's actor object #0x4F, 0x080C2948): BN6's (@exelib/bass/navi; the same code
-- but for his sprite, 08-13, his cape's animation offset 0x10, his wind-up's and volleys' animations 3 and 4, his
-- last volley's 5, whose end ends him) with BN5's shot (./strike) and BN5's effects 0x41 and 0x12.

local common = require("@exelib/bass/navi")
local strike = require("./strike")

return common.make {
    id = "bn5:bass/navi",
    sprite = asset.sprite("bn5:bass"),
    cape_anim_offset = 0x10,
    raise_anim = 3,
    volley_anim = 4,
    ending = "animation",
    last_anim = 5,
    shoot = function(me: Object, x: number, y: number, damage: number)
        strike.spawn(me, x, y, damage)
    end,
    flash = define.effect { sprite = asset.sprite("bn5:hit-sparks"), anim = 8, palette = 0 },
    smoke = define.effect { sprite = asset.sprite("bn5:puff"), anim = 0, palette = 0 },
    appear = asset.sound("bn5:appear"),
}
