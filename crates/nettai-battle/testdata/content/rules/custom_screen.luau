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
    -- The tick the window is in, the screen starts choosing, and reads its keys from the next (`sub_8026B04` hands it to
    -- `sub_8026CCC`).
    first_choosing_tick_reads_keys = true,
    -- L's no-running message starts on the tick after the key, with its sound (`sub_8026E98`, `sub_8026EC8`).
    run_message_at_key = false,
    -- A chip that counts as the invalid chip is picked as it (`sub_8028CCC`), and the selection counts each chip as
    -- checked (`sub_8028E4C`, `getChipID_802A54E`).
    invalid_picks = "picked",
    -- The codes 0x1B and 0x1C stand apart in a selection (`sub_8028E4C`).
    special_codes = "apart",
    -- Each Program Advance forms once a round (`sub_8029652`), it is the Regular chip if a part was (`sub_80292CC`),
    -- and what lies past the selection's end keeps what it held.
    program_advances = { once_a_round = true, keeps_regular = true, clears_past_end = false },
    -- A modifier's Regular chip mark goes with it as it folds into the chip before (`sub_8029224`).
    modifier_passes_regular = false,
    -- The console says its custom screen is open until the window starts sliding out (`sub_8026BF4`).
    status_until = "closing",
    -- The cursor's hover over a dark chip (`sub_802A2B0`, after every tick's state): the volume calls of its ramps,
    -- the music's player's and the screen's, a tick at a time while the window's fade runs (`sub_802A30C`,
    -- `sub_802A362`: `byte_802A3F4`, `byte_802A400`), 5 ticks toward the shade and 6 back (the fade back holds its
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
    -- The players the close sets back to full volume (`sub_802A3CC`).
    restore_players = { 31, 22 },
    -- The custom gauge empties as the screen opens (`sub_8026840` calls `sub_801DF92`), and again at the send.
    gauge_empties_at_open = true,
    -- OK leaves the fade records running until the screens close (`sub_8026A6C` calls `sub_80062EC`).
    fades_clear_at_ok = false,
    -- The tick a key leaves the choosing still draws the cursor and the last turns' block (`sub_8026CCC` draws
    -- after its keys).
    cursor_after_leaving = true,
    -- The frame counts after what the choosing draws.
    frame_counts_first = false,
}
