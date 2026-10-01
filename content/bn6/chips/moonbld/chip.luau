--!strict
-- MoonBld: a spin with a blade that sweeps the eight panels around the
-- navi (./action, ./blade).
--
-- Its record stays the pack's for now (SlashCross's A-charge, weapon 0x11,
-- asks for its action by number, and the Cross special picks it): this is
-- its action, which registration by number runs (chips/054-moonbld;
-- docs/design/content-model-v2.md §12).

local action = require("./action")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")

return {
    -- (SlashCross's A-charge sends its ring as a sword wave.)
    action = action.action { id = "moonbld/action", charged = slashes.moon },
}
