--!strict
-- EXE5's auto battle AI's shared data (docs/design/exe5-map.md §15.9): the panel offset lists the AI's searches
-- read (0x080162A4, EXE6's `PanelOffsetListsPointerTable`) and the chips' reach lists (0x08022AFC), each by name,
-- the original's number beside it. A chip names its reach list in its own `auto_battle` (./place). Lists are { dx, dy }
-- pairs, dx toward the navi's enemies.
-- Written by tools/exe5/gen_auto_battle.py (bn6battle-verify) from the EXE5 ROM; people own it now.

local data = {}

data.offsets = {
    -- 16
    around = { { 0, -1 }, { 0, 1 }, { 1, 0 }, { -1, 0 }, { 1, -1 }, { -1, 1 }, { 1, 1 }, { -1, -1 } },
    -- 17
    two_columns = { { 0, 0 }, { 0, -1 }, { 0, 1 }, { 1, 0 }, { 1, -1 }, { 1, 1 } },
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
    -- 4
    three_ahead = { { 3, 0 } },
    -- 6
    one_ahead = { { 1, 0 } },
    -- 7
    column_ahead = { { 1, -1 }, { 1, 0 }, { 1, 1 } },
    -- 8
    two_in_row = { { 1, 0 }, { 2, 0 } },
    -- 9
    column_three_ahead = { { 3, -1 }, { 3, 0 }, { 3, 1 } },
    -- 10
    two_columns_ahead = { { 1, -1 }, { 1, 0 }, { 1, 1 }, { 2, -1 }, { 2, 0 }, { 2, 1 } },
    -- 11
    columns_two_to_four = { { 2, -1 }, { 2, 0 }, { 2, 1 }, { 3, -1 }, { 3, 0 }, { 3, 1 }, { 4, -1 }, { 4, 0 }, { 4, 1 } },
    -- 12
    column_ahead_and_two = { { 1, -1 }, { 1, 0 }, { 1, 1 }, { 2, 0 } },
    -- 13
    columns_two_and_three = { { 2, -1 }, { 2, 0 }, { 2, 1 }, { 3, -1 }, { 3, 0 }, { 3, 1 } },
    -- 15
    x_one_ahead = { { 0, -1 }, { 0, 1 }, { 1, 0 }, { 2, -1 }, { 2, 1 } },
    -- 16
    ahead_in_row = { { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 }, { 5, 0 } },
    -- 18
    cross_three_ahead = { { 2, 0 }, { 3, -1 }, { 3, 0 }, { 3, 1 }, { 4, 0 } },
    -- 19
    around = { { -1, -1 }, { -1, 0 }, { -1, 1 }, { 0, -1 }, { 0, 1 }, { 1, -1 }, { 1, 0 }, { 1, 1 } },
    -- 20
    columns_three_to_five = { { 3, -1 }, { 3, 0 }, { 3, 1 }, { 4, -1 }, { 4, 0 }, { 4, 1 }, { 5, -1 }, { 5, 0 }, { 5, 1 } },
    -- 21
    ring_three_ahead = { { 2, -1 }, { 2, 0 }, { 2, 1 }, { 3, -1 }, { 3, 1 }, { 4, -1 }, { 4, 0 }, { 4, 1 } },
    -- 22
    all_ahead = { { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 }, { 5, 0 }, { 1, 1 }, { 2, 1 }, { 3, 1 }, { 4, 1 }, { 5, 1 }, { 1, -1 }, { 2, -1 }, { 3, -1 }, { 4, -1 }, { 5, -1 } },
} :: { [string]: { { number } } }


return data
