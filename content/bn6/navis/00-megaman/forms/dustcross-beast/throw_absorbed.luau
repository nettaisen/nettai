--!strict
-- DustCross Beast's buster (`dustcross-beast/throw-absorbed`): the
-- original's weapon routine 0x2C (`sub_8011FCE`). It throws the last
-- obstacle absorbed (the buster's shot as a throw, 200 damage), as
-- DustCross's does, or fires the Falzar Beast forms' buster when there is
-- none.

local buster = require("../../weapons/buster/weapon")
local falzar_beast_buster = require("../../weapons/falzar-beast-buster/weapon")
local throw_absorbed = require("../dustcross/throw_absorbed")

local beast_throw_absorbed = {}

-- `sub_8011FCE`.
local function setup(navi: Object): Action
    if throw_absorbed.take(navi) then
        return buster.shot
    end
    return falzar_beast_buster.setup(navi)
end

beast_throw_absorbed.weapon = define.weapon {
    id = "dustcross-beast/throw-absorbed",
    name = "Beast throw absorbed",
    -- The charge table's row 0x2C; Charge 5 reads on into row 0x2D's first
    -- entry, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 60 },
    setup = setup,
    -- (Transitional: the routine number the pack's forms name it by.)
    legacy = { routines = { 0x2C } },
}

return beast_throw_absorbed
