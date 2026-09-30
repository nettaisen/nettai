--!strict
-- Instant chip effect 9 (`sub_8015AA6`, `off_80EC3F0[9]`): from the user's
-- panel forward to the field's edge, every panel with an enemy body on it
-- gets an immobilizing hit (objects/immobilizer).
--
-- No chip has subtype 9. Weapon routine 0x83 (`sub_8012526`, a link navi's)
-- names it: its weapon.toml registers this module (`instant_chip = 9`)
-- once that weapon is content.

local immobilizer = require("../../objects/immobilizer/immobilizer")

local immobilize_row = {}

-- An enemy body on the panel, by the user's side (`off_8015AF8`).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }

-- `sub_8015AA6`.
function immobilize_row.instant_chip(user: Object, spec: InstantChipSpec)
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

return immobilize_row
