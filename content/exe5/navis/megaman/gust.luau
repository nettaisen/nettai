--!strict
-- EXE5's gust (attack object #0x49, 0x080CEA0C): the shared gust (@exelib/gust/gust, EXE6's code) in EXE6's look (sheet
-- 0c-2e, EXE6's), but that no hit stops it (0x080CEB30 tests its hit flags against 0: EXE6's wind stoppers don't), an
-- unseen one pushes weaker too (0x080CEAA6: any first parameter), and its panel table's side 2 reads other code.
-- The patch cards' Vacuum blows three (navis/megaman/weapons/instants).

local collision = require("../../rules/collision")
local common = require("@exelib/gust")

local gust = {}

gust.kind = common.make_kind("gust", {
    sprite = asset.sprite("gust"),
    is = collision.pull,
    hits = collision.hits_navis,
    stopped_by = 0,
    weak_when_unseen = true,
    -- The panels it can blow over (0x080CEB94, by side: require, forbid).
    panels = {
        [0] = { 0x20, 0x00 },
        [1] = { 0x00, 0x20 },
        [2] = { 0xB427B500, 0xF7342049 },
    },
})

-- 0x080CEBA4: a gust on (x, y) for side `alliance` and flip `flip`, blowing as `how` says, with the damage word
-- `damage`, and the spawner that tracks it by `holder.state.gusts[slot]`, if any.
function gust.spawn(
    x: number,
    y: number,
    alliance: number,
    flip: number,
    how: GustHow,
    damage: number,
    holder: Object?,
    slot: number
): Object?
    return common.spawn(gust.kind, x, y, alliance, flip, how, damage, holder, slot)
end

return gust
