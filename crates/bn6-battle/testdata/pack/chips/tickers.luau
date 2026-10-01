--!strict
-- A chip series composing the family's action with its own length (the
-- pattern that replaces a table indexed by subtype), and a chip whose
-- action starts another when it ends. Ticker2 carries the transitional
-- legacy marker (a subtype and parameters for v1 readers); Ticker4's gives
-- it a number too (what names chips by number reaches it), and it still
-- runs its own action.

local ticker = require("../lib/ticker")

return {
    define.chip { id = "test/ticker1", name = "Ticker1", action = ticker.action { length = 6 } },
    define.chip {
        id = "test/ticker2",
        name = "Ticker2",
        legacy = { subtype = 7, params = { 1, 2 } },
        action = ticker.action { length = 9 },
    },
    define.chip {
        id = "test/ticker3",
        name = "Ticker3",
        action = ticker.action { length = 3, next = ticker.action { length = 2 } },
    },
    define.chip {
        id = "test/ticker4",
        name = "Ticker4",
        legacy = { number = 0x1F0, action = 0x70, subtype = 3 },
        action = ticker.action { length = 4 },
    },
}
