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

local slash_charge = require("../forms/slashcross/charge")
local slashes = require("../forms/slashcross/slashes")

local slash_a_charge = {}

-- `sub_8011CB4`.
local function setup(navi: Object): Action
    navi:prepare_chip()
    local chip = navi.attack_chip
    local action = chip and chip.action
    if action == nil then
        -- (A chip with no action of its own: the plain slash where the
        -- navi stands, what the original's row 0 gives.)
        return slash_charge.begin(navi, slashes.sword, false)
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
    -- The charge table's row 0x11, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 60, 60, 60, 60, 60, 80, 70, 60 },
    setup = setup,
}

return slash_a_charge
