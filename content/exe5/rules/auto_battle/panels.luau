--!strict
-- The panel searches EXE5's auto battle AI runs (./ai, ./place): where to step, where a chip reaches from. They are
-- EXE4's too (@exelib/auto_battle/panels: its functions, here as they are); the two that read EXE5's own lists by
-- name (./data: its panel offset lists and its chips' reach lists) are below.

local data = require("./data")
local shared = require("@exelib/auto_battle/panels")

local panels = table.clone(shared)

-- `GetRandomRelativePanelFiltered` (EXE5's 0x08012C26): `random_relative` over the panel offset list `name`
-- (data.offsets).
function panels.random_offset(x: number, y: number, require: number, forbid: number, name: string, side: number): (number?, number?, number)
    local list = data.offsets[name]
    if list == nil then
        error(string.format("no panel offset list %s in the AI's data", name))
    end
    return shared.random_relative(x, y, require, forbid, list, side)
end

-- EXE5's 0x08012C38: the panels from which a chip of reach list `range` (data.ranges, by name) reaches (x, y): the list
-- turned the other side's way.
function panels.reaching(x: number, y: number, require: number, forbid: number, range: string, side: number): (number?, number?, number)
    local list = data.ranges[range]
    if list == nil then
        error(string.format("no reach list %s in the AI's data", range))
    end
    return shared.reaching(x, y, require, forbid, list, side)
end

return panels
