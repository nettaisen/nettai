--!strict
-- ShadeMan's crush (EXE5's attack object #0x34, 0x080CB4AC; EXE5's own): @exelib/shademan/crush, with EXE5's look and
-- sound; his figure two panels ahead of him (40 pixels ahead of its panel, 0x080CB4F6), its second hit with 50 ticks
-- left, and the panel and the column ahead highlighted each tick, from the first.

local collision = require("../../rules/collision")

return require("@exelib/shademan/crush").make {
    id = "shademan/crush",
    sprite = asset.sprite("shademan"),
    sound = asset.sound("elec-pulse"),
    attack = collision.piercing,
    hits = collision.hits_navis,
    place = function(me: Object)
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + me.facing * 0x28_0000), p.y, p.z)
    end,
    second_at = 0x32,
    highlights = true,
}
