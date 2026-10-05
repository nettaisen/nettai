--!strict
-- H-Burst's shot (attack object #0x9E, `sub_80D8FC4`): the shared shot
-- (@exelib/h-burst/burst) with EXE6's look: an unseen probe (row 0x3D) along
-- its row that bursts where it hits, a status 0x10 hit over the plus and
-- the X around it in turn, with an explosion (effect 0x3D) on each of
-- their panels.

local collision = require("../../rules/collision")
local common = require("@exelib/h-burst/burst")
local statuses = require("../../rules/status")

local burst = {}

burst.kind = common.make_kind("h-burst/burst", {
    probe = collision.probe,
    is = collision.attack,
    hits = collision.hits_navis,
    status = statuses.paralyze_90,
    explosion = define.effect { sprite = asset.sprite("pink-flash"), anim = 0, palette = 2 },
})

-- `sub_80D9130`: a shot on (x, y) for `owner` that bursts `bursts` times,
-- with the element and the damage word `damage`.
function burst.spawn(owner: Object, x: number, y: number, element: number, bursts: number, damage: number): Object?
    return common.spawn(burst.kind, owner, x, y, element, bursts, damage)
end

return burst
