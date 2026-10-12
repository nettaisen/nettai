--!strict
-- EXE4's auto battle AI's shared data (docs/design/exe4-map.md §18 item 74): the panel offset lists the AI's
-- searches read (0x080125B0, EXE6's `PanelOffsetListsPointerTable`) and the chips' reach lists (0x0801DAD0), each by
-- name, the original's number beside it. A chip names its reach list in its own `auto_battle` (./place). Lists are
-- { dx, dy } pairs, dx toward the navi's enemies.
-- Written by tools/exe4/gen_auto_battle.py (bn6battle-verify) from the EXE4 ROM; people own it now.

local data = {}

data.offsets = {
    -- 16
    around = { { 0, -1 }, { 0, 1 }, { 1, 0 }, { -1, 0 }, { 1, -1 }, { -1, 1 }, { 1, 1 }, { -1, -1 } },
} :: { [string]: { { number } } }

data.ranges = {
    -- 0
    ahead_then_column = { { 1, 0 }, { 2, -1 }, { 2, 0 }, { 2, 1 } },
    -- 1
    two_ahead = { { 2, 0 } },
    -- 2
    column_two_ahead = { { 2, -1 }, { 2, 0 }, { 2, 1 } },
    -- 3
    three_in_row = { { 1, 0 }, { 2, 0 }, { 3, 0 } },
    -- 5
    three_ahead = { { 3, 0 } },
    -- 6
    cross_two_ahead = { { 1, 0 }, { 2, -1 }, { 2, 0 }, { 2, 1 }, { 3, 0 } },
    -- 7
    one_ahead = { { 1, 0 } },
    -- 8
    column_ahead = { { 1, -1 }, { 1, 0 }, { 1, 1 } },
    -- 9
    two_in_row = { { 1, 0 }, { 2, 0 } },
    -- 10
    column_three_ahead = { { 3, -1 }, { 3, 0 }, { 3, 1 } },
    -- 11
    two_columns_ahead = { { 1, -1 }, { 1, 0 }, { 1, 1 }, { 2, -1 }, { 2, 0 }, { 2, 1 } },
    -- 12
    columns_two_to_four = { { 2, -1 }, { 2, 0 }, { 2, 1 }, { 3, -1 }, { 3, 0 }, { 3, 1 }, { 4, -1 }, { 4, 0 }, { 4, 1 } },
    -- 14
    columns_two_and_three = { { 2, -1 }, { 2, 0 }, { 2, 1 }, { 3, -1 }, { 3, 0 }, { 3, 1 } },
    -- 15
    ring_two_ahead = { { 1, -1 }, { 1, 0 }, { 1, 1 }, { 2, -1 }, { 2, 1 }, { 3, -1 }, { 3, 0 }, { 3, 1 } },
    -- 16
    x_one_ahead = { { 0, -1 }, { 0, 1 }, { 1, 0 }, { 2, -1 }, { 2, 1 } },
    -- 17
    ahead_in_row = { { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 }, { 5, 0 } },
    -- 18
    two_and_three_ahead = { { 2, 0 }, { 3, 0 } },
} :: { [string]: { { number } } }


return data
