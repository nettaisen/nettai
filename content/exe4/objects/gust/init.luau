--!strict
-- EXE4's gust (attack object #0x49, 0x080D6724): the shared gust (@exelib/gust, EXE6's `sub_80CD2EC` and EXE5's) in
-- EXE6's look (sheet 0c-2e): a wind that blows along its row 10 pixels a tick, pushing what it hits, until it leaves
-- the field or reaches a panel it can't cross. As EXE5's, no hit stops it (0x080D6842 tests its hit flags against 0)
-- and an unseen one pushes weaker too (0x080D67BA: any first parameter); its panel table's side 2 reads its spawner's
-- code. EXE4 has no flip (0x0800ACA6, the object's flip, is its side; 0x080D679E takes its speed's sign from the side
-- alone), so a gust's is 0. The wind stages' wind (objects/wind) blows them.
--
-- Its spawner (0x080D68AC) is EXE4's own: every gust has a spawner that tracks it (its update reads the slot with no
-- test for none), and the spawner's slot takes a word store.

local collision = require("../rules/collision")
local common = require("@exelib/gust")

local gust = {}

gust.kind = common.make_kind("gust", {
    sprite = asset.sprite("gust"),
    is = collision.pull,
    hits = collision.hits_navis,
    stopped_by = 0,
    weak_when_unseen = true,
    -- The panels it can blow over (0x080D689C, by side: require, forbid).
    panels = {
        [0] = { 0x20, 0x00 },
        [1] = { 0x00, 0x20 },
        [2] = { 0xB427B500, 0xF72C2049 },
    },
})

-- 0x080D68AC: a gust on (x, y) for side `alliance`, blowing as `how` says, with the damage word `damage`, tracked by
-- `holder.state.gusts[slot]`. The spawner sets its slot with a word store (`str`), which the ARM aligns down to the
-- slots' first byte: whichever slot it names, the first becomes 1 and the second 0 (a gust in the second slot finds it
-- 0 at its first update and ends unseen).
function gust.spawn(
    x: number,
    y: number,
    alliance: number,
    how: GustHow,
    damage: number,
    holder: Object,
    slot: number
): Object?
    local o = common.spawn(gust.kind, x, y, alliance, 0, how, damage, nil, 0)
    if not o then
        return nil
    end
    local s = o.state :: { holder: Object?, slot: number }
    s.holder, s.slot = holder, slot
    local gusts = (holder.state :: any).gusts
    gusts[1], gusts[2] = 1, 0
    return o
end

return gust
