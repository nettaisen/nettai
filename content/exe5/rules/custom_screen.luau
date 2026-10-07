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
    -- A command of a chatbox script runs on the tick the script reaches it (0x0803EADC tests the print delay before a
    -- character alone, where EXE6's `chatbox_interpreteAndDrawDialogChar` tests it before a command too): a
    -- message's wait for a key starts with its last character, and its speaker's mouth closes with a line's last.
    chatbox_commands_wait_for_text = false,
    -- The characters of a message that don't move its speaker's mouth (0x0803F7CC: a space, the US charmap's 0x92 to
    -- 0xB4, its punctuation and marks with ャ before them, and its ellipsis, 0x112); every other character does, the
    -- asterisks of ToadMan's *Ribbit!!!* too.
    talking_characters = { all_but = " ャ-×=:%?+█\u{E006}ー!\u{E003}\u{E008}&,。.・;'\"~/()「」αβΩ■_Ƶ周⋯" },
    -- The tick the window is in, the screen starts choosing, and reads its keys from the next (0x08022E2C hands it to
    -- 0x08022FE4: EXE6's `sub_8026B04`, `sub_8026CCC`).
    first_choosing_tick_reads_keys = true,
    -- L's no-running message starts on the tick after the key, with its sound (EXE6's `sub_8026E98`, `sub_8026EC8`).
    run_message_at_key = false,
    -- A chip that counts as the invalid chip is picked as it (0x080248FC), and the selection counts each chip as
    -- checked (0x08024A44).
    invalid_picks = "picked",
    -- The codes 0x1B and 0x1C stand apart in a selection (0x08024A44).
    special_codes = "apart",
    -- Each Program Advance forms once a round (0x080252FA), it is the Regular chip if a part was (0x08024F7E), and what
    -- lies past the selection's end keeps what it held.
    program_advances = { once_a_round = true, keeps_regular = true, clears_past_end = false },
    -- A modifier's Regular chip mark goes with it as it folds into the chip before (0x08024F0E).
    modifier_passes_regular = false,
    -- The console says its custom screen is open until the window starts sliding out (0x08022F0C).
    status_until = "closing",
    -- The cursor's hover over a dark chip (0x08025A80, after every tick's state): the volume calls of its ramps,
    -- the music's player's and the screen's, a tick at a time while the window's fade runs (0x08025AE4,
    -- 0x08025B3A: 0x08025BCC, 0x08025BD8), 5 ticks toward the shade and 6 back (the fade back holds its
    -- first frame); its sound's counter steps every tick from the opening, 0 to 63.
    hover = {
        runs = "every_tick",
        to_dark = {
            { music = 0x100, screen = 0x80 },
            { music = 0xE0, screen = 0x80 },
            { music = 0xC0, screen = 0xA0 },
            { music = 0xA0, screen = 0xC0 },
            { music = 0x80, screen = 0xE0 },
        },
        to_clear = {
            { music = 0x80, screen = 0x100 },
            { music = 0x80, screen = 0xE0 },
            { music = 0xA0, screen = 0xC0 },
            { music = 0xC0, screen = 0xA0 },
            { music = 0xE0, screen = 0x80 },
            { music = 0x100, screen = 0x80 },
        },
        players = { 31, 22 },
        sound = { counts = "from_opening", every = 64 },
    },
    -- The players the close sets back to full volume (0x08025BA4).
    restore_players = { 31, 22 },
    -- The custom gauge empties as the screen opens (0x08022C5E), and again at the send.
    gauge_empties_at_open = true,
    -- OK leaves the fade records running until the screens close (0x08022DC0 calls EXE6's `sub_80062EC`).
    fades_clear_at_ok = false,
    -- The tick a key leaves the choosing still draws the cursor and the last turns' block (0x08022FE4 draws after
    -- its keys, as EXE6's `sub_8026CCC`).
    cursor_after_leaving = true,
    -- The frame (the cursor's blink) counts after what the choosing draws, as EXE6's.
    frame_counts_first = false,
    -- The description and L's message are the screen's own states, as EXE6's: they draw only the emblem.
    description_in_choosing = false,
    -- The chatbox's end clears the text's buffers only, as EXE6's: the text goes with the box's first closing step.
    chatbox_end_clears_tiles = false,
    -- The interpreter goes on after a character, as EXE6's: its delay counts down on its own tick.
    chatbox_character_ends_tick = false,
}
