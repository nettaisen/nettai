--!strict
-- BodyGrd's striker (the original's effect object #0x6E, `sub_80E8268`):
-- the shared striker (@exelib/bodygrd/striker) with BN6's identity, its
-- shurikens (./shuriken) and sound: the counter's unseen thrower, one of
-- its side's field objects with 1 HP, dropping a shuriken on the enemy's
-- player navi every 20 ticks, ten in all.

local common = require("@exelib/bodygrd/striker")
local shuriken = require("./shuriken")

local striker = {}

striker.kind = common.make_kind {
    id = "bn6:bodygrd/striker",
    -- Its identity: a field object nothing swallows or leaves as junk,
    -- with no look.
    identity = define.identity {
        id = "bn6:bodygrd-striker",
        class = "field_object",
        version = 0,
        ai_index = 1,
        scrap = false,
        look = { anim = 0, palette = 0, shadow = false },
    },
    shuriken = shuriken.kind,
    throw_sound = asset.sound("bn6:boomerang"),
}

-- `sub_80E8396`: the striker with `element` and the damage word, of
-- `user`'s side.
function striker.spawn(user: Object, element: number, damage: number): Object?
    return common.spawn(striker.kind, user, element, damage)
end

return striker
