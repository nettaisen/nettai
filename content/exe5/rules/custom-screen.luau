--!strict
-- The custom screen's slot grid (0x08025E04) and the lists its neighbor fix-up scans (0x08025E34,
-- 0x08023E2C, 0x08023E84): EXE6's values.
-- Written by tools/exe5/gen_rules.py (bn6battle-verify) from the EXE5 ROM; people own it now.

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
    left_scan_top = { 4, 9, 3, 8, 2, 1, 0, 10 },
    left_scan_bottom = { 9, 8, 7, 6, 5, 11, 10, 7, 6, 5, 2, 1, 0, 11, 10, 6, 5, 11, 10 },
    right_scan_top = { 0, 1, 2, 3, 8, 4, 9, 10 },
    right_scan_bottom = { 5, 6, 7, 8, 9, 11, 10 },
    left_scan_start = { 7, 6, 5, 4, 2, 17, 16, 15, 7, 1, 0, 0 },
    right_scan_start = { 1, 2, 3, 5, 7, 1, 2, 3, 4, 5, 0, 0 },
    -- The re-deal (SearchSoul's Shuffle, its form's folder's shuffle: 0x080253E8): how many of the hand's chips it deals again
    -- stay in the hand, by how many they are, from none (0x080254C8; past the list, none).
    redeal_kept = { 0, 0, 0, 1, 1, 2, 2, 3, 3 },
    -- SELECT's hidden window, brought back by a key, draws no emblem on that tick or the next (0x08023022 has
    -- neither of the calls EXE6's `sub_8026D06` makes of the emblem's routine, here 0x08025824): it is back with
    -- the choosing state's own draw.
    emblem_at_window_return = false,
}
