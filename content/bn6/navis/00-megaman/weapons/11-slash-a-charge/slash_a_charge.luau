--!strict
-- SlashCross's A-charge: weapon routine 0x11 (`sub_8011CB4`), what a
-- charged sword chip becomes in SlashCross (and its Beast form). The chip
-- is used as usual (`sub_80127C0`: its damage, bonuses and parameters),
-- and then: VarSwrd and NeoVari (actions 0x53 and 0x54) run their own
-- action, charged; any other chip slashes as the charged slash (action
-- 0x41, weapons/12-slash-charge), MoonBld (action 0x40) as variant 0x11.

local slash_a_charge = {}

local SLASH = 0x41

-- The chip actions that keep their own action, and MoonBld's.
local VAR_SWORD, NEO_VARIABLE, MOON_BLADE = 0x53, 0x54, 0x40

-- MoonBld's variant as a charged slash.
local MOON_VARIANT = 0x11

-- `sub_8011CB4`.
function slash_a_charge.setup(navi: Object): number
    local action = navi:prepare_chip()
    if action == VAR_SWORD or action == NEO_VARIABLE then
        navi.charged = 1
        return action
    end
    if action == MOON_BLADE then
        navi.variant = MOON_VARIANT
    end
    return SLASH
end

return slash_a_charge
