--!strict
-- The test content's custom screen: five chip slots on top, five below, OK
-- at the top row's right end and a special button under it; the last two
-- bottom slots start hidden. A neighbor that is missing is looked for along
-- its row, which wraps through OK (the top row) or the button under it (the
-- bottom row). A re-deal keeps none of the hand.

return {
    slots = {
        { kind = "chip_position", vertical = 5, left = 10, right = 1 },
        { kind = "chip_position", vertical = 6, left = 0, right = 2 },
        { kind = "chip_position", vertical = 7, left = 1, right = 3 },
        { kind = "chip_position", vertical = 8, left = 2, right = 4 },
        { kind = "chip_position", vertical = 9, left = 3, right = 10 },
        { kind = "chip_position", vertical = 0, left = 11, right = 6 },
        { kind = "chip_position", vertical = 1, left = 5, right = 7 },
        { kind = "chip_position", vertical = 2, left = 6, right = 8 },
        { kind = "hidden", vertical = 3, left = 7, right = 9 },
        { kind = "hidden", vertical = 4, left = 8, right = 11 },
        { kind = "ok", vertical = 11, left = 4, right = 0 },
        { kind = "hidden", vertical = 10, left = 9, right = 5 },
    },
    left_scan_top = { 4, 3, 2, 1, 0, 10 },
    left_scan_bottom = { 9, 8, 7, 6, 5, 11, 10 },
    right_scan_top = { 0, 1, 2, 3, 4, 10 },
    right_scan_bottom = { 5, 6, 7, 8, 9, 11, 10 },
    left_scan_start = { 5, 4, 3, 2, 1, 5, 4, 3, 2, 1, 0, 0 },
    right_scan_start = { 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 0, 0 },
    emblem_at_window_return = true,
    chatbox_commands_wait_for_text = true,
    talking_characters = { only = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz" },
}
