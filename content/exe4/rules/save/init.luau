--!strict
-- What an EXE4 player's save brings to the navi's stats, which nothing else gives (docs/design/exe4-map.md §12, §3.3):
-- MegaMan's HP (the save's 0x2150, 0x2152 and 0x21CA, which 0x0800D726 copies to NaviStats +0x30, +0x32 and +0x34:
-- HP Memories raise it), the Regular memory (the save's 0x2148 plus 4: RegUps) and fighting in the sun (his block's
-- +0x29, which the overworld writes by the map, 0x0802A770, and the reset keeps, 0x08036CDC). The
-- rest of a round's stats are the navi's fresh stats (`fresh_stats`, its definition).
--
-- Not yet (docs/design/exe4-map.md §18): the NaviCust's compile and the patch cards, which change the stats after this.

type Setup = { hp: number, reg_up: number, sun: boolean }

-- What a save brings at its highest, which a setup that says nothing has (the rules' `setup_defaults`): the base HP
-- 1000 and the Regular memory 50 (0x0802D570 works the save's byte out from the RegUps' key items).
local MAX_HP = 1000
local MAX_REG_UP = 50

local function round_setup(side: number)
    local stats = battle.navi(side)
    local setup = rules.setup() :: Setup
    stats.regular_memory = setup.reg_up
    stats.max_base_hp = setup.hp
    stats.max_hp = setup.hp
    stats.hp = setup.hp
    stats.sun = setup.sun
end

-- `validate`'s part: what the save can hold.
local function validate(side: number, problem: (text: string, field: string?, entry: number?) -> ())
    local setup = rules.setup_of(side)
    if setup.hp < 1 or setup.hp > MAX_HP then
        problem(string.format("hp %d: MegaMan's HP is 1 to %d", setup.hp, MAX_HP), "hp")
    end
    if setup.reg_up > MAX_REG_UP then
        problem(string.format("reg_up %d: the Regular memory is %d at most", setup.reg_up, MAX_REG_UP), "reg_up")
    end
end

return {
    round_setup = round_setup,
    validate = validate,
    MAX_HP = MAX_HP,
    MAX_REG_UP = MAX_REG_UP,
}
