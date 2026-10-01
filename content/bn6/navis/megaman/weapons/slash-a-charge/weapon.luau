--!strict
-- SlashCross's A-charge (`megaman/slash-a-charge`): the original's weapon
-- routine 0x11 (`sub_8011CB4`), what a charged sword chip becomes in
-- SlashCross and its Beast form. The chip is used as usual (`sub_80127C0`:
-- its damage, bonuses and parameters), and then the navi slashes as
-- SlashCross's charged slash (forms/slashcross/charge) of the chip's
-- sword: the slash its action's spec names (`charged`, lib/swords/slash;
-- MoonBld's its ring), with a step sword's dash. A variable sword (VarSwrd,
-- NeoVari: `charged = "own"`) runs its own action, charged: its pick is
-- what becomes the charged slash (lib/swords/vari).

local slash_charge = require("../../forms/slashcross/charge")
local slashes = require("../../forms/slashcross/slashes")

local slash_a_charge = {}

-- Transitional: a charged chip that is still a pack record has no
-- definition to ask, and its action is a registration by number. The
-- original's own tests then: the variable swords' actions, MoonBld's (the
-- ring: row 0x11), and for any other the slash by the chip's subtype, with
-- a dash if its first parameter is set (StepSwrd's).
local VAR_SWORD, NEO_VARIABLE, MOON_BLADE = 0x53, 0x54, 0x40
local MOON_ROW = 0x11

local function by_number(navi: Object, action: number): Action | number
    if action == VAR_SWORD or action == NEO_VARIABLE then
        navi.charged = 1
        return action
    end
    local row = if action == MOON_BLADE then MOON_ROW else navi.variant
    return slash_charge.begin(navi, slashes.by_row(row), navi:attack_param(1) ~= 0)
end

-- `sub_8011CB4`.
local function setup(navi: Object): Action | number
    local number = navi:prepare_chip()
    local chip = navi.attack_chip
    local action = chip and chip.action
    if action == nil then
        return by_number(navi, number)
    end
    local spec = action.args :: { charged: (ChargedSlash | "own")? }?
    if spec and spec.charged == "own" then
        navi.charged = 1
        return action
    end
    return slash_charge.of_action(navi, action)
end

slash_a_charge.weapon = define.weapon {
    id = "megaman/slash-a-charge",
    name = "Slash A-charge",
    -- The charge table's row 0x11, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 60, 60, 60, 60, 60, 80, 70, 60 },
    setup = setup,
}

return slash_a_charge
