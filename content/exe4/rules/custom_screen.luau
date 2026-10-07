--!strict
-- EXE4's custom screen (docs/design/exe4-map.md §5): its slots, two rows of five (the screen block's slots from
-- +0x20, a halfword each), OK (the selection's state 8) and the button under it (state 0xC, Double Soul's, DOWN
-- from OK when the screen offers souls); each place's keys in the order its handler reads them, and where each
-- direction goes: the first slot of its list that holds something (0x0801FEE4 tests the slot's flags' bit 1).
-- Written by tools/exe4/gen_rules.py (bn6battle-verify) from the EXE4 ROM; people own it now.

return {
    slots = {
        -- 0: the top row's chip 1
        {
            kind = "chip_position",
            keys = { "a", "b", "r", "l", "start", "select", "up", "down", "right", "left" },
            up = { 0 },
            down = { 5, 8 },
            right = { 1, 2, 3, 4, 8, 10 },
            left = { 10 },
        },
        -- 1: the top row's chip 2
        {
            kind = "chip_position",
            keys = { "a", "b", "r", "l", "start", "select", "up", "down", "right", "left" },
            up = { 1, 0 },
            down = { 6, 5, 8 },
            right = { 2, 3, 4, 8, 10 },
            left = { 0, 10 },
        },
        -- 2: the top row's chip 3
        {
            kind = "chip_position",
            keys = { "a", "b", "r", "l", "start", "select", "up", "down", "right", "left" },
            up = { 2, 1, 0 },
            down = { 7, 6, 8, 5 },
            right = { 3, 4, 8, 10 },
            left = { 1, 0, 10 },
        },
        -- 3: the top row's chip 4
        {
            kind = "chip_position",
            keys = { "a", "b", "r", "l", "start", "select", "up", "down", "right", "left" },
            up = { 3, 2, 1, 0 },
            down = { 8, 7, 6, 5 },
            right = { 4, 10 },
            left = { 2, 1, 0, 10 },
        },
        -- 4: the top row's chip 5
        {
            kind = "chip_position",
            keys = { "a", "b", "r", "l", "start", "select", "up", "down", "right", "left" },
            up = { 4, 3, 2, 1, 0 },
            down = { 9, 8, 7, 6, 5 },
            right = { 10 },
            left = { 3, 2, 1, 0, 10 },
        },
        -- 5: the bottom row's chip 1
        {
            kind = "chip_position",
            keys = { "a", "b", "r", "l", "start", "select", "up", "down", "right", "left" },
            up = { 0 },
            down = { 5, 8 },
            right = { 6, 7, 8, 9, 10 },
            left = { 10 },
        },
        -- 6: the bottom row's chip 2
        {
            kind = "chip_position",
            keys = { "a", "b", "r", "l", "start", "select", "up", "down", "right", "left" },
            up = { 1, 0 },
            down = { 6, 5, 8 },
            right = { 7, 8, 9, 10 },
            left = { 5, 10 },
        },
        -- 7: the bottom row's chip 3
        {
            kind = "chip_position",
            keys = { "a", "b", "r", "l", "start", "select", "up", "down", "right", "left" },
            up = { 2, 1, 0 },
            down = { 7, 6, 8, 5 },
            right = { 8, 9, 10 },
            left = { 6, 5, 10 },
        },
        -- 8: the bottom row's chip 4
        {
            kind = "hidden",
            keys = { "a", "b", "r", "l", "start", "select", "up", "down", "right", "left" },
            up = { 3, 2, 1, 0 },
            down = { 8, 7, 6, 5 },
            right = { 9, 10 },
            left = { 7, 6, 5, 2, 1, 0, 10 },
        },
        -- 9: the bottom row's chip 5
        {
            kind = "hidden",
            keys = { "a", "b", "r", "l", "start", "select", "up", "down", "right", "left" },
            up = { 4, 3, 2, 1, 0 },
            down = { 9, 8, 7, 6, 5 },
            right = { 10 },
            left = { 8, 7, 6, 5, 3, 2, 1, 0, 10 },
        },
        -- 10: OK
        {
            kind = "ok",
            keys = { "a", "b", "l", "select", "down", "right", "left" },
            down = { 11 },
            right = { 0, 9 },
            left = { 4, 3, 2, 1, 0, 9 },
        },
        -- 11: the button under OK
        {
            kind = "hidden",
            keys = { "start", "b", "select", "l", "a", "up", "right", "left" },
            up = { 10 },
            right = { 5, 0 },
            left = { 9, 8, 7, 6, 5, 4, 3, 2, 1, 0 },
        },
    },
    -- SELECT's hidden window draws nothing coming back (0x0801E59C).
    emblem_at_window_return = false,
    -- A command of a chatbox script runs on the tick the script reaches it (0x0804E136 tests the print delay before a
    -- character alone, 0x0804E162).
    chatbox_commands_wait_for_text = false,
    -- The characters that move a speaker's mouth: the character set's 1 to 63 but 37 (0x0804E8B4), in English and in
    -- Japanese.
    talking_characters = {
        only = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
            .. "アイウエオカキクケコサシスセソタチツテトナニヌネノハフヘホマミムメモヤユヨラリルレロワ熱斗ヲンガギグゲゴ",
    },
    -- The window's slide ends with the choosing state's sub-state set to 0 (0x0801E2FE); its first tick sets the
    -- cursor's state up and reads no key (0x08020340).
    first_choosing_tick_reads_keys = false,
    -- L runs the no-running message's script with the key (0x080205C6: in a netbattle, its battle flags' 0x20, the
    -- script's entry 3), and its state waits for it from the next tick (0x08020A28).
    run_message_at_key = true,
    -- A chip that counts as the invalid chip (0x08020306: past the navi's Mega or Giga limit, or a code it hasn't) is
    -- refused (0x0801F73C), and the selection counts each chip as it is (0x0801F9E0).
    invalid_picks = "refused",
    -- The code 27, the dark chips' (0x0801DF7E), is one `*` doesn't stand for (0x0801FA96).
    special_codes = "unstarred",
    -- A Program Advance forms whenever its recipe is picked (0x0801F390 keeps no record), it isn't the Regular chip
    -- (0x0801F404 clears its flags), and the entries past the selection's end are cleared (0x0801F438).
    program_advances = { once_a_round = false, keeps_regular = false, clears_past_end = true },
    -- A modifier's Regular chip mark stays on the chip it folds into (0x0801F17E).
    modifier_passes_regular = true,
    -- The console says its custom screen is open until it sends (bit 4: 0x08007618 to 0x0801E986; bit 1 from the
    -- choosing's first tick to OK, 0x08020348 to 0x08020652).
    status_until = "sending",
    -- The cursor's hover over a dark chip (0x0801E478, which the choosing state alone calls, 0x0801E40E; a tick
    -- whose selection's state left the chips turns it neither way, 0x0801E4B6): its ramps' volume calls, by tick
    -- after it turns, the music's player's and the screen's (11 ticks, 0x0801E514; a call on each odd count,
    -- 0x0801E528, from 0x0801E584); its sound on the tick after the shade settles and every 61 ticks it stays
    -- (+0x5C, 0x0801E49C).
    hover = {
        runs = "while_choosing",
        to_dark = {
            {},
            { music = 0xE0, screen = 0xE0 },
            {},
            { music = 0xC0, screen = 0x100 },
            {},
            { music = 0xA0, screen = 0x100 },
            {},
            { music = 0x80, screen = 0xE0 },
            {},
            { music = 0x60, screen = 0xC0 },
            {},
        },
        to_clear = {
            {},
            { music = 0xE0, screen = 0xE0 },
            {},
            { music = 0x100, screen = 0xC0 },
            {},
            { music = 0x100, screen = 0xA0 },
            {},
            { music = 0xE0, screen = 0x80 },
            {},
            { music = 0xC0, screen = 0x60 },
            {},
        },
        players = { 31, 9 },
        sound = { counts = "while_dark", every = 61 },
    },
    -- The players the close sets back to full volume (0x0801E180).
    restore_players = { 9, 31 },
    -- The custom gauge stays full while the screen is up (its opening, 0x0801DC28, leaves it): the send empties it
    -- (0x0801E1B4: 0x080159B0).
    gauge_empties_at_open = false,
    -- OK clears the fade records at once (0x08020656, 0x0802065A), a dark chip's shade with them.
    fades_clear_at_ok = true,
    -- The choosing state draws what its selection's state is after the keys (0x0801E412: the table 0x0801E454):
    -- the tick a key leaves the choosing draws no cursor, and OK and SELECT take the last turns' block off.
    cursor_after_leaving = false,
    -- It draws the last turns' block and counts its frame before its keys (0x0801E3DA, 0x0801E3DE): the cursor blinks
    -- a frame further on.
    frame_counts_first = true,
}
