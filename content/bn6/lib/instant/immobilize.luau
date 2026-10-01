--!strict
-- Instant chip effect 9 (`sub_8015AA6`, `off_80EC3F0[9]`): from the user's
-- panel forward to the field's edge, every panel with an enemy body on it
-- gets an immobilizing hit (immobilizer.luau).
--
-- No chip has effect 9: weapon routine 0x83 (`sub_8012526`, a link navi's)
-- names it, and calls `immobilize.instant` when it is ported.

local immobilizer = require("./immobilizer")

local immobilize = {}

-- An enemy body on the panel, by the user's side (`off_8015AF8`).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }

-- `sub_8015AA6`: an `instant` hook.
function immobilize.instant(user: Object, spec: InstantChipSpec)
    local x, y = spec.panel_x, spec.panel_y
    local ahead = user.facing
    local enemy = ENEMY_BODY[user.alliance]
    while field.valid(x, y) do
        if bit32.btest(field.flags(x, y), enemy) then
            immobilizer.spawn(user, x, y)
        end
        x += ahead
    end
end

return immobilize
