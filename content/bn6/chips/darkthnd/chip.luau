--!strict
-- DarkThnd: Thunder's shot (chips/thunder) with a big ball that homes in
-- for 12 panels.
--
-- Its record stays the pack's for now (a dark chip, with Thunder as its
-- substitute): this is its action, which registration by number runs
-- (chips/01e-thunder, docs/design/content-model-v2.md §12).

local shoot = require("../thunder/shoot")

-- Its hits' status: paralysis.
local PARALYZE = 0x10

return {
    action = shoot.action { id = "darkthnd/action", ball = { panels = 0x0C, status = PARALYZE } },
}
