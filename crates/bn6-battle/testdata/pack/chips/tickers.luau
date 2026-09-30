--!strict
-- A chip series composing the family's action with its own length (the
-- pattern that replaces a table indexed by subtype), and a chip whose
-- action starts another when it ends.

local ticker = require("../lib/ticker")

return {
    define.chip { id = "test/ticker1", name = "Ticker1", action = ticker.action { length = 6 } },
    define.chip { id = "test/ticker2", name = "Ticker2", action = ticker.action { length = 9 } },
    define.chip {
        id = "test/ticker3",
        name = "Ticker3",
        action = ticker.action { length = 3, next = ticker.action { length = 2 } },
    },
}
