--!strict
-- EXE5's diving shuriken (attack object #0x5C, 0x080D14F0: EXE6's BodyGrd shuriken, @exelib/bodygrd/shuriken, the same
-- code but for its sprite and its dive's start), ShadowMan's throw (../../chips/shadoman/navi): EXE5's shuriken
-- sprite (0c-2f, EXE6's place), a thrown slash (row 0x1C) that hits navis, the effect table's row 2 as it shows, and,
-- as its dive starts, its marker over its panel for 15 ticks in the operation battle (0x080D15E6).

local collision = require("../../rules/collision")
local common = require("@exelib/bodygrd/shuriken")
local marker = require("../../lib/navi_chips/marker")

local shuriken = {}

shuriken.kind = common.make_kind {
    id = "shuriken",
    sprite = asset.sprite("shuriken"),
    land_sound = asset.sound("panel-crack"),
    puff = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 },
    self_type = collision.thrown_slash,
    target = collision.hits_navis,
    started = function(me: Object)
        marker.spawn(me, me.panel_x, me.panel_y, 0xF, 1, false)
    end,
}

-- 0x080D16AC: a shuriken onto (x, y) from `owner` with `element`, `hit_mod` and the damage word, of its side and
-- facing.
function shuriken.spawn(owner: Object, x: number, y: number, element: number, hit_mod: number, damage: number): Object?
    return common.spawn(shuriken.kind, owner, x, y, element, hit_mod, damage)
end

return shuriken
